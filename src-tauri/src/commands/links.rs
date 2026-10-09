use std::path::Path;
use std::process::Command;

use tauri::State;

use crate::error::AppError;
use crate::links::{self, LinkAction};
use crate::state::AppState;

#[tauri::command]
pub async fn open_link(state: State<'_, AppState>, url: String) -> Result<(), AppError> {
    match links::classify_link(&url, links::path_kind) {
        LinkAction::Browser(url) => open(&[url.as_str()]),
        LinkAction::Image(path) => Command::new("/usr/bin/open").args(["-a", "Preview"]).arg(path).spawn().map(drop).map_err(AppError::from),
        LinkAction::Editor(path) => match state.shell().find_binary("code") {
            Some(code) => Command::new(code).arg("-g").arg(path).spawn().map(drop).map_err(AppError::from),
            None => reveal(&path),
        },
        LinkAction::Reveal(path) => reveal(&path),
        LinkAction::Unsupported => Err(AppError::Invalid("este tipo de link não é aberto pelo app".to_owned())),
    }
}

fn open(arguments: &[&str]) -> Result<(), AppError> {
    Command::new("/usr/bin/open").args(arguments).spawn()?;
    Ok(())
}

/// `open -R` only selects the item in Finder; it never launches an app bundle.
fn reveal(path: &Path) -> Result<(), AppError> {
    Command::new("/usr/bin/open").arg("-R").arg(path).spawn()?;
    Ok(())
}
