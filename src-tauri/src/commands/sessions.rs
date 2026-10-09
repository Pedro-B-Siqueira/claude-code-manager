use std::path::PathBuf;
use std::time::Duration;

use tauri::ipc::{Channel, Response};
use tauri::{AppHandle, Manager, State};

use crate::claudegauge::ClaudeGaugeStatus;
use crate::error::AppError;
use crate::hooks::settings_file::{SESSION_ENV, TOKEN_ENV};
use crate::library::queries;
use crate::live::service::live_views;
use crate::notifications;
use crate::pty::{LaunchMode, LaunchSpec, OutputSink, SpawnRequest, SubmitOutcome};
use crate::sessions::launch::{claude_arguments, login_shell_command, resolve_claude};
use crate::sessions::view::LiveSessionView;
use crate::settings;
use crate::state::AppState;

const CLOSE_GRACE: Duration = Duration::from_secs(3);
const QUIT_GRACE: Duration = Duration::from_secs(3);

struct ChannelSink(Channel<Response>);

impl OutputSink for ChannelSink {
    fn deliver(&self, bytes: &[u8]) -> bool {
        self.0.send(Response::new(bytes.to_vec())).is_ok()
    }
}

pub(crate) fn launch(state: &AppState, app: &AppHandle, launch: LaunchSpec) -> Result<LiveSessionView, AppError> {
    launch_with_key(state, app, launch, uuid::Uuid::new_v4().to_string())
}

fn launch_with_key(state: &AppState, app: &AppHandle, launch: LaunchSpec, key: String) -> Result<LiveSessionView, AppError> {
    let app_settings = settings::load(&state.database)?;
    let shell = state.shell();
    let claude = resolve_claude(app_settings.claude_binary.as_deref(), shell, state.paths.home())?;
    let settings_file = state.hook_settings_file.get();
    let hook_env = match settings_file {
        Some(_) => vec![(TOKEN_ENV.to_owned(), state.hook_tokens.issue(&key)), (SESSION_ENV.to_owned(), key.clone())],
        None => Vec::new(),
    };
    let arguments = claude_arguments(&launch, settings_file.map(PathBuf::as_path));
    let command = login_shell_command(shell, &claude, &arguments, hook_env);
    let spawned = state.pty.spawn(SpawnRequest { key: key.clone(), launch, command, scrollback_lines: app_settings.scrollback_lines });
    if spawned.is_err() {
        state.hook_tokens.revoke(&key);
    }
    spawned?;
    notify_live_changed(app);
    find_view(state, &key)
}

fn find_view(state: &AppState, key: &str) -> Result<LiveSessionView, AppError> {
    live_views(state)?.into_iter().find(|view| view.key == key).ok_or_else(|| AppError::UnknownSession(key.to_owned()))
}

pub fn notify_live_changed(app: &AppHandle) {
    crate::notify_live_changed(app);
}

#[tauri::command]
pub async fn session_new(state: State<'_, AppState>, app: AppHandle, cwd: String) -> Result<LiveSessionView, AppError> {
    let spec = LaunchSpec {
        session_id: uuid::Uuid::new_v4().to_string(),
        cwd: PathBuf::from(cwd),
        mode: LaunchMode::New,
        worktree: None,
    };
    launch(&state, &app, spec)
}

#[tauri::command]
pub async fn session_resume(state: State<'_, AppState>, app: AppHandle, session_id: String) -> Result<LiveSessionView, AppError> {
    let existing = state.pty.snapshots().into_iter().find(|snapshot| snapshot.launch.session_id == session_id && (!snapshot.exited || snapshot.hibernated));
    if let Some(snapshot) = existing {
        if snapshot.hibernated {
            return wake(&state, &app, &snapshot.key);
        }
        return find_view(&state, &snapshot.key);
    }
    let summary = queries::session_summary(&state.database.connection(), &session_id)?.ok_or_else(|| AppError::UnknownSession(session_id.clone()))?;
    let cwd = summary.item.cwd.ok_or_else(|| AppError::UnknownSession(session_id.clone()))?;
    let spec = LaunchSpec { session_id, cwd: PathBuf::from(cwd), mode: LaunchMode::Resume, worktree: None };
    launch(&state, &app, spec)
}

#[tauri::command]
pub async fn session_close(state: State<'_, AppState>, app: AppHandle, key: String) -> Result<(), AppError> {
    let was_running = state.pty.running_keys().contains(&key);
    state.pty.close(&key, CLOSE_GRACE)?;
    if !was_running {
        state.status.forget(&key);
    }
    notify_live_changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn live_list(state: State<'_, AppState>) -> Result<Vec<LiveSessionView>, AppError> {
    live_views(&state)
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub claudegauge: ClaudeGaugeStatus,
    pub notifications_enabled: bool,
    pub hooks_active: bool,
}

#[tauri::command]
pub async fn app_info(state: State<'_, AppState>) -> Result<AppInfo, AppError> {
    let claudegauge = state.claudegauge();
    let preference = settings::load(&state.database)?.notifications;
    Ok(AppInfo {
        claudegauge,
        notifications_enabled: notifications::enabled(preference, claudegauge),
        hooks_active: state.hook_settings_file.get().is_some(),
    })
}

/// The session the latest notification was about, so activating the app can open it.
#[tauri::command]
pub fn take_notified_session(state: State<'_, AppState>) -> Option<String> {
    state.notifications.take_recent()
}

#[tauri::command]
pub fn pty_attach(state: State<'_, AppState>, key: String, output: Channel<Response>) -> Result<(), AppError> {
    state.pty.attach(&key, Box::new(ChannelSink(output)))
}

#[tauri::command]
pub fn pty_detach(state: State<'_, AppState>, key: String) {
    state.pty.detach(&key);
}

#[tauri::command]
pub fn pty_write(state: State<'_, AppState>, key: String, data: String) -> Result<(), AppError> {
    state.pty.write(&key, data.as_bytes())
}

#[tauri::command]
pub fn pty_resize(state: State<'_, AppState>, key: String, cols: u16, rows: u16) -> Result<(), AppError> {
    state.pty.resize(&key, cols, rows)
}

#[tauri::command]
pub async fn recent_dirs(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    let items = queries::list_sessions(&state.database.connection())?;
    let mut directories: Vec<String> = Vec::new();
    for cwd in items.into_iter().filter_map(|item| item.cwd) {
        if !directories.contains(&cwd) && PathBuf::from(&cwd).is_dir() {
            directories.push(cwd);
        }
    }
    directories.truncate(12);
    Ok(directories)
}

const WAKE_GRACE: Duration = Duration::from_secs(5);

/// Brings a hibernated session back with `claude --resume`, keeping its card (same key). If the
/// new process cannot start (say, its folder is gone), the hibernated card stays as it was.
fn wake(state: &AppState, app: &AppHandle, key: &str) -> Result<LiveSessionView, AppError> {
    let snapshot = state.pty.snapshot(key)?;
    if !snapshot.hibernated {
        return find_view(state, key);
    }
    state.pty.wait_for_exit(key, WAKE_GRACE);
    let spec = LaunchSpec { mode: LaunchMode::Resume, ..snapshot.launch };
    let view = launch_with_key(state, app, spec, key.to_owned())?;
    state.status.forget(key);
    Ok(view)
}

#[tauri::command]
pub async fn session_wake(state: State<'_, AppState>, app: AppHandle, key: String) -> Result<LiveSessionView, AppError> {
    wake(&state, &app, &key)
}

/// The UI reports whether the window is visible; terminal previews pause while it is not.
#[tauri::command]
pub fn ui_set_visible(state: State<'_, AppState>, visible: bool) {
    state.pty.set_previews_paused(!visible);
}

/// Called by the UI after the user confirmed quitting with sessions still open.
#[tauri::command]
pub fn app_quit(state: State<'_, AppState>, app: AppHandle) {
    state.quit_confirmed.store(true, std::sync::atomic::Ordering::SeqCst);
    state.pty.shutdown_all(QUIT_GRACE);
    app.exit(0);
}

const SUBMIT_TIMEOUT: Duration = Duration::from_secs(3);

/// Ids, not paths, come from the UI: only files from the attachments folder are ever pasted.
#[tauri::command]
pub async fn session_submit_with_images(app: AppHandle, key: String, attachment_ids: Vec<String>) -> Result<SubmitOutcome, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let paths = attachment_ids.iter().map(|id| state.attachments.existing(id)).collect::<Result<Vec<_>, _>>()?;
        state.pty.submit_with_images(&key, &paths, SUBMIT_TIMEOUT)
    })
    .await
    .map_err(|error| AppError::Terminal(error.to_string()))?
}
