pub mod claudegauge;
pub mod commands;
pub mod context;
pub mod db;
pub mod error;
pub mod git;
pub mod hibernation;
pub mod hooks;
pub mod layout;
pub mod library;
pub mod live;
pub mod notifications;
pub mod paths;
pub mod pricing;
pub mod pty;
pub mod sessions;
pub mod settings;
pub mod shell_env;
pub mod state;
pub mod status;
pub mod transcript;
pub mod tray;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_log::{Target, TargetKind};

use crate::db::Database;
use crate::error::AppError;
use crate::hooks::server::{HookEvent, HookHandler, HookServer};
use crate::hooks::settings_file;
use crate::hooks::tokens::TokenRegistry;
use crate::library::service::{self, LibraryNotice, LibraryNotifier};
use crate::notifications::NotificationCenter;
use crate::paths::AppPaths;
use crate::pty::{PtyEvent, PtyManager, PtyNotifier, now_ms};
use crate::state::AppState;
use crate::status::StatusTracker;

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
        hook_tokens: Arc::new(TokenRegistry::default()),
        hook_settings_file: OnceLock::new(),
        status: StatusTracker::default(),
        notifications: NotificationCenter::default(),
        tray: OnceLock::new(),
    })
}

/// The live list changed: refresh the UI and the menu bar.
pub fn notify_live_changed(app: &AppHandle) {
    emit_or_log(app, "live:changed", ());
    if let Some(state) = app.try_state::<AppState>() {
        if let Some(tray) = state.tray.get() {
            tray.schedule();
        }
    }
}

fn emit_or_log<S: serde::Serialize + Clone>(app: &AppHandle, event: &str, payload: S) {
    if let Err(error) = app.emit(event, payload) {
        log::warn!("falha ao notificar a interface ({event}): {error}");
    }
}

fn library_notifier(app: AppHandle) -> LibraryNotifier {
    Arc::new(move |notice| match notice {
        LibraryNotice::Progress(progress) => emit_or_log(&app, "library:progress", progress),
        LibraryNotice::Changed => emit_or_log(&app, "library:changed", ()),
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
    Arc::new(move |event| match event {
        PtyEvent::PreviewChanged { key, lines } => emit_or_log(&app, "session:preview", PreviewPayload { key, lines }),
        PtyEvent::Exited { key, code, instance } => {
            // A woken session reuses its key; the old process exiting must not touch the new one.
            let current = app.try_state::<AppState>().is_some_and(|state| state.pty.is_current(&key, instance));
            if !current {
                return;
            }
            if let Some(state) = app.try_state::<AppState>() {
                state.hook_tokens.revoke(&key);
            }
            notify_live_changed(&app);
            emit_or_log(&app, "session:exited", ExitPayload { key, code });
        }
    })
}

fn hook_handler(app: AppHandle) -> HookHandler {
    Arc::new(move |event: HookEvent| {
        let Some(state) = app.try_state::<AppState>() else { return };
        let Some(update) = status::status_for_hook(&event.payload) else { return };
        let Some(transition) = state.status.apply(&event.session_key, update, now_ms()) else { return };
        emit_or_log(&app, "session:status", transition.clone());
        notify_live_changed(&app);
        announce(&app, &state, &transition);
    })
}

fn announce(app: &AppHandle, state: &AppState, transition: &status::StatusTransition) {
    let preference = settings::load(&state.database).map(|settings| settings.notifications).unwrap_or_default();
    if !notifications::enabled(preference, state.claudegauge()) {
        return;
    }
    let title = state
        .pty
        .snapshot(&transition.key)
        .ok()
        .and_then(|snapshot| library::queries::session_summary(&state.database.connection(), &snapshot.launch.session_id).ok().flatten())
        .map(|summary| summary.item.title)
        .unwrap_or_else(|| "Claude Code".to_owned());
    state.notifications.announce(app, &title, transition);
}

/// Hooks are optional: if the local server cannot start, sessions still open, just without them.
fn start_hooks(app: &AppHandle, state: &AppState) {
    let server = match HookServer::start(Arc::clone(&state.hook_tokens), hook_handler(app.clone())) {
        Ok(server) => server,
        Err(error) => {
            log::warn!("servidor de hooks indisponível; status virá só do terminal: {error}");
            return;
        }
    };
    match settings_file::write_settings(&state.paths, server.port()) {
        Ok(file) => {
            let _ = state.hook_settings_file.set(file);
        }
        Err(error) => log::warn!("não foi possível gravar o arquivo de hooks: {error}"),
    }
}

/// Quitting while a session is working or asking for permission needs the user's confirmation.
fn needs_quit_confirmation(app: &AppHandle) -> bool {
    let state = app.state::<AppState>();
    if state.quit_confirmed.load(Ordering::SeqCst) {
        return false;
    }
    let now = now_ms();
    let hooks_active = state.hook_settings_file.get().is_some();
    let busy = state
        .pty
        .running_keys()
        .iter()
        .filter(|key| !hooks_active || state.status.current(key, now).is_some_and(|current| current.status.is_busy()))
        .count();
    if busy == 0 {
        return false;
    }
    tray::show_main_window(app);
    emit_or_log(app, "app:close-requested", busy);
    true
}

/// "Sair" from the menu bar: same confirmation as ⌘Q when something is still working.
fn request_quit(app: &AppHandle) {
    if needs_quit_confirmation(app) {
        return;
    }
    let state = app.state::<AppState>();
    state.quit_confirmed.store(true, Ordering::SeqCst);
    state.pty.shutdown_all(Duration::from_secs(3));
    app.exit(0);
}

fn log_plugin(paths: &AppPaths) -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_log::Builder::new()
        .clear_targets()
        .target(Target::new(TargetKind::Stdout))
        .target(Target::new(TargetKind::Folder { path: paths.logs_dir(), file_name: Some("ccm".to_owned()) }))
        .level(log::LevelFilter::Info)
        .build()
}

fn setup(app: &mut tauri::App, paths: AppPaths) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    let state = build_state(paths, pty_notifier(handle.clone()))?;
    service::start(&state.paths, state.library_progress.clone(), library_notifier(handle.clone()))?;
    start_hooks(&handle, &state);
    let registry_notifier = handle.clone();
    live::watcher::start(state.paths.claude_home(), Arc::new(move || notify_live_changed(&registry_notifier)));
    app.manage(state);
    let hibernation_notifier = handle.clone();
    hibernation::start(handle.clone(), Arc::new(move || notify_live_changed(&hibernation_notifier)));
    match tray::install(&handle, request_quit) {
        Ok(updater) => {
            updater.schedule();
            let _ = handle.state::<AppState>().tray.set(updater);
        }
        Err(error) => log::warn!("ícone da barra de menus indisponível: {error}"),
    }
    std::thread::spawn(move || {
        handle.state::<AppState>().shell();
    });
    Ok(())
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
        .plugin(tauri_plugin_notification::init())
        .setup(move |app| setup(app, paths))
        .on_window_event(|window, event| {
            // Closing the window keeps the app (and its sessions) running in the menu bar.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::settings_get,
            commands::settings::settings_update,
            commands::settings::layout_get,
            commands::settings::layout_set,
            commands::settings::grid_order_get,
            commands::settings::grid_order_set,
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
            commands::library::library_file_edits,
            commands::library::library_edit,
            commands::library::library_activity,
            commands::sessions::session_new,
            commands::sessions::session_resume,
            commands::sessions::session_close,
            commands::sessions::live_list,
            commands::sessions::pty_attach,
            commands::sessions::pty_detach,
            commands::sessions::pty_write,
            commands::sessions::pty_resize,
            commands::sessions::recent_dirs,
            commands::sessions::app_quit,
            commands::sessions::app_info,
            commands::sessions::take_notified_session,
            commands::sessions::session_wake,
            commands::sessions::ui_set_visible,
            commands::dev::dev_scenario,
            commands::git::git_status,
            commands::git::worktree_plan,
            commands::git::worktree_create,
            commands::git::worktree_list,
            commands::git::worktree_remove,
            commands::git::open_vscode,
            commands::git::open_finder,
            commands::git::open_pr,
        ])
        .build(tauri::generate_context!())
        .unwrap_or_else(|error| {
            log::error!("falha ao iniciar o app: {error}");
            std::process::exit(1);
        })
        .run(|app, event| match event {
            RunEvent::ExitRequested { api, .. } if needs_quit_confirmation(app) => api.prevent_exit(),
            RunEvent::Exit => app.state::<AppState>().pty.shutdown_all(Duration::from_secs(2)),
            RunEvent::Reopen { .. } => tray::show_main_window(app),
            _ => {}
        });
}
