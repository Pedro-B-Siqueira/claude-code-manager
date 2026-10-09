pub mod commands;
pub mod context;
pub mod db;
pub mod error;
pub mod library;
pub mod paths;
pub mod pricing;
pub mod pty;
pub mod sessions;
pub mod settings;
pub mod shell_env;
pub mod state;
pub mod transcript;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_log::{Target, TargetKind};

use crate::db::Database;
use crate::error::AppError;
use crate::library::service::{self, LibraryNotice, LibraryNotifier};
use crate::paths::AppPaths;
use crate::pty::{PtyEvent, PtyManager, PtyNotifier};
use crate::state::AppState;

pub fn build_state(paths: AppPaths, pty_notifier: PtyNotifier) -> Result<AppState, AppError> {
    paths.create_app_support()?;
    let database_file = paths.database_file();
    paths.ensure_writable(&database_file)?;
    let database = Database::open(&database_file)?;
    Ok(AppState {
        paths,
        database,
        library_progress: Arc::default(),
        pty: PtyManager::new(pty_notifier),
        shell_environment: OnceLock::new(),
        quit_confirmed: AtomicBool::new(false),
    })
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewPayload {
    key: String,
    lines: Vec<String>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ExitPayload {
    key: String,
    code: Option<i32>,
}

fn pty_notifier(app: AppHandle) -> PtyNotifier {
    Arc::new(move |event| {
        let emitted = match event {
            PtyEvent::PreviewChanged { key, lines } => app.emit("session:preview", PreviewPayload { key, lines }),
            PtyEvent::Exited { key, code } => {
                commands::sessions::notify_live_changed(&app);
                app.emit("session:exited", ExitPayload { key, code })
            }
        };
        if let Err(error) = emitted {
            log::warn!("falha ao notificar a interface: {error}");
        }
    })
}

/// Quitting with sessions still running needs the user's confirmation, asked by the UI.
fn needs_quit_confirmation(app: &AppHandle) -> bool {
    let state = app.state::<AppState>();
    if state.quit_confirmed.load(Ordering::SeqCst) {
        return false;
    }
    let running = state.pty.running_keys().len();
    if running == 0 {
        return false;
    }
    if let Err(error) = app.emit("app:close-requested", running) {
        log::warn!("falha ao pedir confirmação de saída: {error}");
        return false;
    }
    true
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
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let state = build_state(paths, pty_notifier(app.handle().clone()))?;
            service::start(&state.paths, state.library_progress.clone(), library_notifier(app.handle().clone()))?;
            app.manage(state);
            let warm_up = app.handle().clone();
            std::thread::spawn(move || {
                warm_up.state::<AppState>().shell();
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if needs_quit_confirmation(window.app_handle()) {
                    api.prevent_close();
                }
            }
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
            commands::sessions::session_new,
            commands::sessions::session_resume,
            commands::sessions::session_close,
            commands::sessions::live_list,
            commands::sessions::live_session,
            commands::sessions::pty_attach,
            commands::sessions::pty_detach,
            commands::sessions::pty_write,
            commands::sessions::pty_resize,
            commands::sessions::recent_dirs,
            commands::sessions::app_quit,
            commands::dev::dev_scenario,
        ])
        .build(tauri::generate_context!())
        .unwrap_or_else(|error| {
            log::error!("falha ao iniciar o app: {error}");
            std::process::exit(1);
        })
        .run(|app, event| match event {
            RunEvent::ExitRequested { api, .. } if needs_quit_confirmation(app) => api.prevent_exit(),
            RunEvent::Exit => app.state::<AppState>().pty.shutdown_all(std::time::Duration::from_secs(2)),
            _ => {}
        });
}
