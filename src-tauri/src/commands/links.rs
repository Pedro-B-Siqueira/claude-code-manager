use tauri::State;

use crate::error::AppError;
use crate::links::{self, LinkAction};
use crate::platform::{self, OpenTarget};
use crate::state::AppState;

#[tauri::command]
pub async fn open_link(state: State<'_, AppState>, url: String) -> Result<(), AppError> {
    match links::classify_link(&url, links::path_kind) {
        LinkAction::Browser(url) => platform::open(OpenTarget::Url(url.as_str())),
        LinkAction::Image(path) => platform::open(OpenTarget::Image(&path)),
        LinkAction::Editor(path) => match state.shell().find_binary("code") {
            Some(code) => std::process::Command::new(code).arg("-g").arg(&path).spawn().map(drop).map_err(AppError::from),
            None => platform::open(OpenTarget::Reveal(&path)),
        },
        // Revealing only shows where the item is; it never launches an app bundle.
        LinkAction::Reveal(path) => platform::open(OpenTarget::Reveal(&path)),
        LinkAction::Unsupported => Err(AppError::Invalid("este tipo de link não é aberto pelo app".to_owned())),
    }
}
