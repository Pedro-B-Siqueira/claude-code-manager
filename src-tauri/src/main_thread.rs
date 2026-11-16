//! AppKit must be called on the main thread. Command and helper threads hop there with this and wait
//! for the result; the main thread itself must never call it, or it would wait on itself.

use std::sync::mpsc;
use std::time::Duration;

use objc2::MainThreadMarker;
use tauri::AppHandle;

use crate::error::AppError;

const MAIN_THREAD_WAIT: Duration = Duration::from_secs(5);

pub fn on_main_thread<T: Send + 'static>(app: &AppHandle, work: impl FnOnce(MainThreadMarker) -> T + Send + 'static) -> Result<T, AppError> {
    let (sender, receiver) = mpsc::channel();
    app.run_on_main_thread(move || {
        if let Some(main_thread) = MainThreadMarker::new() {
            let _ = sender.send(work(main_thread));
        }
    })
    .map_err(|error| AppError::Window(error.to_string()))?;
    receiver.recv_timeout(MAIN_THREAD_WAIT).map_err(|_| AppError::Window("a thread principal não respondeu".to_owned()))
}
