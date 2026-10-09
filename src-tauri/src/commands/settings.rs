use tauri::State;

use crate::error::AppError;
use crate::settings::{self, AppSettings};
use crate::state::AppState;

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> Result<AppSettings, AppError> {
    settings::load(&state.database)
}

#[tauri::command]
pub fn settings_update(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, AppError> {
    settings::save(&state.database, settings)
}
