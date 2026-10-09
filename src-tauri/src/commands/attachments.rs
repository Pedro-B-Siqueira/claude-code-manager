use std::path::Path;
use std::process::Command;

use tauri::State;
use tauri::ipc::{InvokeBody, Request, Response};

use crate::attachments::{Attachment, AttachmentUsage};
use crate::error::AppError;
use crate::state::AppState;

/// The image arrives as the raw request body: no base64 round trip for a 10 MB screenshot.
#[tauri::command]
pub async fn attachment_save(state: State<'_, AppState>, request: Request<'_>) -> Result<Attachment, AppError> {
    match request.body() {
        InvokeBody::Raw(bytes) => state.attachments.save(bytes),
        InvokeBody::Json(_) => Err(AppError::Invalid("o anexo precisa chegar como bytes".to_owned())),
    }
}

#[tauri::command]
pub async fn attachment_import(state: State<'_, AppState>, path: String) -> Result<Attachment, AppError> {
    state.attachments.import(Path::new(&path))
}

#[tauri::command]
pub async fn attachment_preview(state: State<'_, AppState>, id: String) -> Result<Response, AppError> {
    Ok(Response::new(state.attachments.read(&id)?))
}

#[tauri::command]
pub async fn attachment_open(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let path = state.attachments.existing(&id)?;
    Command::new("/usr/bin/open").args(["-a", "Preview"]).arg(path).spawn()?;
    Ok(())
}

#[tauri::command]
pub async fn attachment_remove(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    state.attachments.remove(&id)
}

#[tauri::command]
pub async fn attachments_usage(state: State<'_, AppState>) -> Result<AttachmentUsage, AppError> {
    state.attachments.usage()
}

#[tauri::command]
pub async fn attachments_clear(state: State<'_, AppState>) -> Result<AttachmentUsage, AppError> {
    state.attachments.clear()
}
