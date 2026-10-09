use tauri::State;

use crate::error::AppError;
use crate::library::details::{self, ActivityItem, EditDetail};
use crate::library::models::{IndexProgress, SearchHit, SessionListItem, SessionSummary};
use crate::library::queries;
use crate::state::AppState;

#[tauri::command]
pub async fn library_list(state: State<'_, AppState>) -> Result<Vec<SessionListItem>, AppError> {
    queries::list_sessions(&state.database.connection())
}

#[tauri::command]
pub async fn library_search(state: State<'_, AppState>, query: String) -> Result<Vec<SearchHit>, AppError> {
    queries::search(&state.database.connection(), &query)
}

#[tauri::command]
pub async fn library_summary(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Option<SessionSummary>, AppError> {
    queries::session_summary(&state.database.connection(), &session_id)
}

#[tauri::command]
pub async fn library_rename(
    state: State<'_, AppState>,
    session_id: String,
    name: Option<String>,
) -> Result<(), AppError> {
    queries::rename_session(&state.database.connection(), &session_id, name.as_deref())
}

#[tauri::command]
pub async fn library_pin(state: State<'_, AppState>, session_id: String, pinned: bool) -> Result<(), AppError> {
    queries::set_pinned(&state.database.connection(), &session_id, pinned)
}

#[tauri::command]
pub async fn library_set_tags(
    state: State<'_, AppState>,
    session_id: String,
    tags: Vec<String>,
) -> Result<Vec<String>, AppError> {
    queries::set_tags(&mut state.database.connection(), &session_id, &tags)
}

#[tauri::command]
pub async fn library_set_category(
    state: State<'_, AppState>,
    session_id: String,
    category: Option<String>,
) -> Result<(), AppError> {
    queries::set_category(&state.database.connection(), &session_id, category.as_deref())
}

#[tauri::command]
pub async fn library_tags(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    queries::list_tags(&state.database.connection())
}

#[tauri::command]
pub async fn library_categories(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    queries::list_categories(&state.database.connection())
}

#[tauri::command]
pub async fn library_status(state: State<'_, AppState>) -> Result<IndexProgress, AppError> {
    Ok(state.library_progress.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone())
}

#[tauri::command]
pub async fn library_file_edits(
    state: State<'_, AppState>,
    session_id: String,
    file_path: String,
) -> Result<Vec<EditDetail>, AppError> {
    details::file_edits(&state.database.connection(), &session_id, &file_path)
}

#[tauri::command]
pub async fn library_edit(state: State<'_, AppState>, edit_id: i64) -> Result<Option<EditDetail>, AppError> {
    details::edit_by_id(&state.database.connection(), edit_id)
}

#[tauri::command]
pub async fn library_activity(state: State<'_, AppState>, session_id: String, limit: u32) -> Result<Vec<ActivityItem>, AppError> {
    details::activity(&state.database.connection(), &session_id, limit.min(500))
}
