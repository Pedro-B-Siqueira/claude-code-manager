pub mod commands;
pub mod context;
pub mod db;
pub mod error;
pub mod library;
pub mod paths;
pub mod pricing;
pub mod settings;
pub mod state;
pub mod transcript;

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_log::{Target, TargetKind};

use crate::db::Database;
use crate::error::AppError;
use crate::library::service::{self, LibraryNotice, LibraryNotifier};
use crate::paths::AppPaths;
use crate::state::AppState;

pub fn build_state(paths: AppPaths) -> Result<AppState, AppError> {
    paths.create_app_support()?;
    let database_file = paths.database_file();
    paths.ensure_writable(&database_file)?;
    let database = Database::open(&database_file)?;
    Ok(AppState { paths, database, library_progress: Arc::default() })
}

fn library_notifier(app: AppHandle) -> LibraryNotifier {
    Arc::new(move |notice| {
        let emitted = match notice {
            LibraryNotice::Progress(progress) => app.emit("library:progress", progress),
            LibraryNotice::Changed => app.emit("library:changed", ()),
        };
        if let Err(error) = emitted {
            log::warn!("falha ao notificar a interface: {error}");
        }
    })
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
            let state = build_state(paths)?;
            service::start(&state.paths, state.library_progress.clone(), library_notifier(app.handle().clone()))?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::settings_get,
            commands::settings::settings_update,
            commands::library::library_list,
            commands::library::library_search,
            commands::library::library_summary,
            commands::library::library_rename,
            commands::library::library_pin,
            commands::library::library_set_tags,
            commands::library::library_set_category,
            commands::library::library_tags,
            commands::library::library_categories,
            commands::library::library_status,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            log::error!("falha ao executar o app: {error}");
            std::process::exit(1);
        });
}
