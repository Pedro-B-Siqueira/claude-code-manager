use tauri::State;

use crate::error::AppError;
use crate::layout::{self, PanelLayout};
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

#[tauri::command]
pub async fn layout_get(state: State<'_, AppState>) -> Result<PanelLayout, AppError> {
    layout::load_layout(&state.database)
}

#[tauri::command]
pub async fn layout_set(state: State<'_, AppState>, layout: PanelLayout) -> Result<PanelLayout, AppError> {
    layout::save_layout(&state.database, layout)
}

#[tauri::command]
pub async fn grid_order_get(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    layout::load_grid_order(&state.database)
}

#[tauri::command]
pub async fn grid_order_set(state: State<'_, AppState>, session_ids: Vec<String>) -> Result<(), AppError> {
    layout::save_grid_order(&state.database, &session_ids)
}
