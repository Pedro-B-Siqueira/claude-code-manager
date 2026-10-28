use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::jarvis::{self, MockData, ShapeRule};
use crate::pty::now_ms;
use crate::state::AppState;

/// Async so the window is created off the IPC thread (sync commands that build windows can deadlock).
#[tauri::command]
pub async fn jarvis_open(app: AppHandle) -> Result<(), AppError> {
    jarvis::open(&app)
}

#[tauri::command]
pub async fn jarvis_close(app: AppHandle) {
    jarvis::close(&app);
}

#[tauri::command]
pub async fn jarvis_focus_session(app: AppHandle, key: String) {
    jarvis::hand_over(&app, "tray:focus-session", Some(&key));
}

#[tauri::command]
pub async fn jarvis_new_session(app: AppHandle) {
    jarvis::hand_over(&app, "app:new-session", None);
}

#[tauri::command]
pub fn jarvis_shape_rules(state: State<'_, AppState>) -> Vec<ShapeRule> {
    jarvis::load_shape_rules(state.paths.app_support())
}

#[tauri::command]
pub fn jarvis_mock_data() -> Option<MockData> {
    jarvis::load_mock_data(now_ms())
}
