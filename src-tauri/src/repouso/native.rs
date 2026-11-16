//! AppKit and LocalAuthentication calls behind Repouso (macOS only). Everything here runs on the
//! main thread except `authenticate`, which waits for the system prompt on the calling thread.

use std::ffi::c_void;
use std::sync::mpsc;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool};
use objc2::{MainThreadMarker, class, msg_send};
use objc2_app_kit::{NSApplication, NSApplicationPresentationOptions, NSWindow, NSWindowCollectionBehavior};
use objc2_foundation::{NSError, NSString};

#[link(name = "LocalAuthentication", kind = "framework")]
unsafe extern "C" {}

/// `kCGScreenSaverWindowLevel`: above the menu bar, the Dock and notification banners.
pub const LOCK_WINDOW_LEVEL: isize = 1000;
pub const NORMAL_WINDOW_LEVEL: isize = 0;

/// `LAPolicyDeviceOwnerAuthentication`: Touch ID with the account password as the alternative.
const DEVICE_OWNER_AUTHENTICATION: isize = 2;
const UNLOCK_REASON: &str = "desbloquear o Modo Jarvis";

/// A valid combination: hiding the menu bar requires hiding the Dock, and the disable flags require
/// both (an invalid one raises an Objective-C exception and would abort the app).
fn kiosk_options() -> NSApplicationPresentationOptions {
    NSApplicationPresentationOptions::HideDock
        | NSApplicationPresentationOptions::HideMenuBar
        | NSApplicationPresentationOptions::DisableProcessSwitching
        | NSApplicationPresentationOptions::DisableForceQuit
        | NSApplicationPresentationOptions::DisableSessionTermination
        | NSApplicationPresentationOptions::DisableHideApplication
        | NSApplicationPresentationOptions::DisableAppleMenu
}

/// Returns the options in effect before, to put them back on unlock.
pub fn enter_kiosk(main_thread: MainThreadMarker) -> usize {
    let application = NSApplication::sharedApplication(main_thread);
    let previous = application.presentationOptions();
    application.activate();
    application.setPresentationOptions(kiosk_options());
    previous.bits()
}

pub fn restore_presentation(main_thread: MainThreadMarker, bits: usize) {
    NSApplication::sharedApplication(main_thread).setPresentationOptions(NSApplicationPresentationOptions::from_bits_truncate(bits));
}

/// # Safety
/// `ns_window` must be the `NSWindow` of a live Tauri window, used on the main thread.
pub unsafe fn set_window_level(ns_window: *mut c_void, level: isize) {
    // SAFETY: guaranteed by the caller; Tauri hands out the window's own NSWindow pointer.
    let window = unsafe { &*ns_window.cast::<NSWindow>() };
    window.setLevel(level);
}

/// On every Space, including other apps' full-screen ones: a link a session opens in the browser
/// makes macOS switch Spaces, and the lock has to be there too. Returns the previous behavior.
///
/// # Safety
/// `ns_window` must be the `NSWindow` of a live Tauri window, used on the main thread.
pub unsafe fn pin_to_every_space(ns_window: *mut c_void) -> usize {
    // SAFETY: guaranteed by the caller.
    let window = unsafe { &*ns_window.cast::<NSWindow>() };
    let previous = window.collectionBehavior();
    window.setCollectionBehavior(
        previous | NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::FullScreenAuxiliary | NSWindowCollectionBehavior::Stationary,
    );
    previous.bits()
}

/// # Safety
/// `ns_window` must be the `NSWindow` of a live Tauri window, used on the main thread.
pub unsafe fn restore_collection_behavior(ns_window: *mut c_void, bits: usize) {
    // SAFETY: guaranteed by the caller.
    let window = unsafe { &*ns_window.cast::<NSWindow>() };
    window.setCollectionBehavior(NSWindowCollectionBehavior::from_bits_truncate(bits));
}

/// Takes the focus back when another app got it (or the Space changed) while locked, so neither the
/// screen nor the keyboard reach that app. Returns whether anything had to be done.
///
/// # Safety
/// `ns_window` must be the `NSWindow` of a live Tauri window, used on the main thread.
pub unsafe fn reclaim_focus(main_thread: MainThreadMarker, ns_window: *mut c_void) -> bool {
    // SAFETY: guaranteed by the caller.
    let window = unsafe { &*ns_window.cast::<NSWindow>() };
    let application = NSApplication::sharedApplication(main_thread);
    if application.isActive() && window.isOnActiveSpace() && window.isKeyWindow() {
        return false;
    }
    #[allow(deprecated)]
    application.activateIgnoringOtherApps(true);
    window.orderFrontRegardless();
    window.makeKeyAndOrderFront(None);
    true
}

/// Asks macOS for Touch ID or the user's password. The app never sees nor stores the password.
pub fn authenticate() -> bool {
    let (sender, receiver) = mpsc::channel();
    let reply = RcBlock::new(move |success: Bool, error: *mut NSError| {
        if !success.as_bool() && !error.is_null() {
            // SAFETY: LocalAuthentication passes a valid NSError when it reports a failure.
            let code = unsafe { (*error).code() };
            log::info!("desbloqueio do Repouso recusado (LAError {code})");
        }
        let _ = sender.send(success.as_bool());
    });
    let reason = NSString::from_str(UNLOCK_REASON);
    // SAFETY: LAContext is a plain NSObject subclass; the selector and argument types match
    // -[LAContext evaluatePolicy:localizedReason:reply:]. The context lives until the reply arrives,
    // since releasing it earlier would cancel the evaluation.
    let context: Retained<AnyObject> = unsafe { msg_send![class!(LAContext), new] };
    unsafe {
        let _: () = msg_send![&context, evaluatePolicy: DEVICE_OWNER_AUTHENTICATION, localizedReason: &*reason, reply: &*reply];
    }
    let approved = receiver.recv().unwrap_or(false);
    drop(context);
    approved
}
