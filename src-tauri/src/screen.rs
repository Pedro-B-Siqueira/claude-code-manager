//! The top band a window must keep clear. On a Mac with a notch, the menu bar strip beside the camera
//! stays black over full-screen spaces and covers whatever a window draws there.

use std::ffi::c_void;

use objc2_app_kit::NSWindow;
use tauri::{AppHandle, WebviewWindow};

use crate::main_thread::on_main_thread;

/// Points to keep clear at the top of the screen `window` is on; 0 without a notch or on failure.
pub fn safe_area_top(app: &AppHandle, window: WebviewWindow) -> f64 {
    let top = on_main_thread(app, move |_| match window.ns_window() {
        // SAFETY: the pointer is this live window's own NSWindow, used here on the main thread.
        Ok(ns_window) => unsafe { screen_top_inset(ns_window) },
        Err(_) => 0.0,
    });
    top.unwrap_or_else(|error| {
        log::warn!("não foi possível ler a área segura da tela: {error}");
        0.0
    })
}

/// # Safety
/// `ns_window` must be the `NSWindow` of a live window, used on the main thread.
unsafe fn screen_top_inset(ns_window: *mut c_void) -> f64 {
    // SAFETY: guaranteed by the caller.
    let window = unsafe { &*ns_window.cast::<NSWindow>() };
    window.screen().map_or(0.0, |screen| screen.safeAreaInsets().top.max(0.0))
}
