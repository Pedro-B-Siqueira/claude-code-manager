//! Leaving macOS full screen and waiting until the animation really ended. Tauri reports a window out
//! of full screen as soon as the transition starts; a window opened before it ends lands in the Space
//! being torn down, at the wrong size and stuck behind the others.

use std::cell::RefCell;
use std::ptr::NonNull;
use std::sync::mpsc;
use std::time::Duration;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObjectProtocol, ProtocolObject};
use objc2_app_kit::NSWindowDidExitFullScreenNotification;
use objc2_foundation::{NSNotification, NSNotificationCenter};
use tauri::{AppHandle, WebviewWindow};

use crate::error::AppError;
use crate::main_thread::on_main_thread;

const EXIT_WAIT: Duration = Duration::from_secs(5);

thread_local! {
    /// Lives on the main thread, where the observer is added and removed.
    static EXIT_OBSERVER: RefCell<Option<Retained<ProtocolObject<dyn NSObjectProtocol>>>> = const { RefCell::new(None) };
}

fn window_error(error: tauri::Error) -> AppError {
    AppError::Window(error.to_string())
}

/// Takes the window out of full screen and blocks until macOS says the transition finished. Must not
/// run on the main thread, which delivers that notice.
pub fn leave_and_wait(app: &AppHandle, window: &WebviewWindow) -> Result<(), AppError> {
    let (sender, exited) = mpsc::channel::<()>();
    let observed = window.clone();
    on_main_thread(app, move |_| {
        let ns_window = observed.ns_window().map_err(window_error)?;
        let notice = RcBlock::new(move |_notification: NonNull<NSNotification>| {
            let _ = sender.send(());
        });
        // SAFETY: the object is this live window's own NSWindow and the block only sends on a
        // channel; the notification name is an AppKit constant.
        let token = unsafe {
            NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                Some(NSWindowDidExitFullScreenNotification),
                Some(&*ns_window.cast::<AnyObject>()),
                None,
                &notice,
            )
        };
        EXIT_OBSERVER.with(|slot| *slot.borrow_mut() = Some(token));
        Ok::<(), AppError>(())
    })??;
    let outcome = window.set_fullscreen(false).map_err(window_error).and_then(|()| {
        exited.recv_timeout(EXIT_WAIT).map_err(|_| AppError::Window("o macOS não terminou de sair da tela cheia".to_owned()))
    });
    if let Err(error) = on_main_thread(app, |_| stop_observing()) {
        log::warn!("falha ao parar de observar a tela cheia: {error}");
    }
    outcome
}

fn stop_observing() {
    EXIT_OBSERVER.with(|slot| {
        if let Some(token) = slot.borrow_mut().take() {
            // SAFETY: the token was returned by addObserverForName on this same center.
            unsafe { NSNotificationCenter::defaultCenter().removeObserver(token.as_ref()) };
        }
    });
}
