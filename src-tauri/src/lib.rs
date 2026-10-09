pub mod commands;
pub mod db;
pub mod error;
pub mod paths;
pub mod settings;
pub mod state;

use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

use crate::db::Database;
use crate::error::AppError;
use crate::paths::AppPaths;
use crate::state::AppState;

pub fn build_state(paths: AppPaths) -> Result<AppState, AppError> {
    paths.create_app_support()?;
    let database_file = paths.database_file();
    paths.ensure_writable(&database_file)?;
    let database = Database::open(&database_file)?;
    Ok(AppState { paths, database })
}

fn log_plugin(paths: &AppPaths) -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_log::Builder::new()
        .clear_targets()
        .target(Target::new(TargetKind::Stdout))
        .target(Target::new(TargetKind::Folder {
            path: paths.logs_dir(),
            file_name: Some("ccm".to_owned()),
        }))
        .level(log::LevelFilter::Info)
        .build()
}

pub fn run() {
    let paths = match AppPaths::from_environment() {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("Claude Code Manager não pôde iniciar: {error}");
            std::process::exit(1);
        }
    };
    tauri::Builder::default()
        .plugin(log_plugin(&paths))
        .setup(move |app| {
            app.manage(build_state(paths)?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::settings_get,
            commands::settings::settings_update,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            log::error!("falha ao executar o app: {error}");
            std::process::exit(1);
        });
}
