use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::AppError;
use crate::jarvis;
use crate::pty::now_ms;
use crate::repouso::AppRepouso;
use crate::repouso::lock::{LockSnapshot, UnlockOutcome};
use crate::repouso::lock_delay::{LockDelay, read_lock_delay};
use crate::repouso::preferences::{load_show_details, save_show_details};
use crate::state::AppState;

const CHANGED_EVENT: &str = "repouso:changed";
const DEV_TIMEOUT_ENV: &str = "CCM_REPOUSO_DEV_TIMEOUT";
const DEV_AUTOLOCK_ENV: &str = "CCM_REPOUSO_DEV_AUTOLOCK";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepousoPreparation {
    pub lock_delay: LockDelay,
    pub show_details: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UnlockResult {
    Unlocked,
    Refused,
    NotLocked,
    AlreadyAsking,
}

impl From<UnlockOutcome> for UnlockResult {
    fn from(outcome: UnlockOutcome) -> Self {
        match outcome {
            UnlockOutcome::Unlocked => Self::Unlocked,
            UnlockOutcome::Refused => Self::Refused,
            UnlockOutcome::NotLocked => Self::NotLocked,
            UnlockOutcome::AlreadyAsking => Self::AlreadyAsking,
        }
    }
}

fn repouso(state: &AppState) -> Result<Arc<AppRepouso>, AppError> {
    state.repouso.get().cloned().ok_or_else(|| AppError::Invalid("o Repouso ainda não está pronto".to_owned()))
}

fn announce(app: &AppHandle, snapshot: LockSnapshot) {
    if let Err(error) = app.emit(CHANGED_EVENT, snapshot) {
        log::warn!("falha ao avisar as janelas do Repouso: {error}");
    }
}

/// What the activation dialog needs: whether the screen locks at once, and the remembered choice.
#[tauri::command]
pub fn repouso_prepare(state: State<'_, AppState>) -> RepousoPreparation {
    let lock_delay = if cfg!(target_os = "macos") { read_lock_delay() } else { LockDelay::Unknown };
    RepousoPreparation { lock_delay, show_details: load_show_details(&state.database) }
}

#[tauri::command]
pub async fn repouso_activate(app: AppHandle, state: State<'_, AppState>, show_details: bool) -> Result<LockSnapshot, AppError> {
    if let Err(error) = save_show_details(&state.database, show_details) {
        log::warn!("não foi possível lembrar a escolha do Repouso: {error}");
    }
    let repouso = repouso(&state)?;
    let snapshot = repouso.activate(show_details, now_ms())?;
    announce(&app, snapshot);
    start_dev_timeout(&app, &repouso, snapshot);
    Ok(snapshot)
}

/// Waits for the system prompt off the async runtime: the user may take a while to answer.
#[tauri::command]
pub async fn repouso_unlock(app: AppHandle, state: State<'_, AppState>) -> Result<UnlockResult, AppError> {
    let repouso = repouso(&state)?;
    let waiting = Arc::clone(&repouso);
    let outcome = tauri::async_runtime::spawn_blocking(move || waiting.unlock()).await.map_err(|error| AppError::Invalid(error.to_string()))?;
    announce(&app, repouso.snapshot());
    if outcome == UnlockOutcome::Unlocked {
        if let Some(window) = app.get_webview_window(jarvis::WINDOW_LABEL) {
            let _ = window.set_focus();
        }
    }
    Ok(outcome.into())
}

#[tauri::command]
pub fn repouso_state(state: State<'_, AppState>) -> Result<LockSnapshot, AppError> {
    Ok(repouso(&state)?.snapshot())
}

/// Development builds only: `CCM_REPOUSO_DEV_AUTOLOCK=<seconds>` locks on its own that long after
/// launch, so the native steps can be exercised on a real Mac without anyone at the keyboard.
pub fn start_dev_autolock(app: &AppHandle) {
    if !cfg!(debug_assertions) {
        return;
    }
    let Some(seconds) = std::env::var(DEV_AUTOLOCK_ENV).ok().and_then(|value| value.trim().parse::<u64>().ok()) else { return };
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(seconds));
        let Some(repouso) = app.try_state::<AppState>().and_then(|state| state.repouso.get().cloned()) else { return };
        match repouso.activate(true, now_ms()) {
            Ok(snapshot) => {
                log::info!("Repouso: ativado pelo autoteste de desenvolvimento");
                announce(&app, snapshot);
                start_dev_timeout(&app, &repouso, snapshot);
            }
            Err(error) => log::warn!("Repouso: autoteste falhou ao ativar: {error}"),
        }
    });
}

/// Development builds only: `CCM_REPOUSO_DEV_TIMEOUT=<seconds>` unlocks without asking, so a bug in
/// the unlock path cannot lock the developer out. Release builds ignore it.
fn start_dev_timeout(app: &AppHandle, repouso: &Arc<AppRepouso>, snapshot: LockSnapshot) {
    if !cfg!(debug_assertions) {
        return;
    }
    let Some(seconds) = std::env::var(DEV_TIMEOUT_ENV).ok().and_then(|value| value.trim().parse::<u64>().ok()) else { return };
    let (app, repouso) = (app.clone(), Arc::clone(repouso));
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(seconds));
        if repouso.snapshot().since == snapshot.since {
            log::warn!("Repouso: tempo-limite de desenvolvimento, desbloqueando sem autenticação");
            repouso.release_without_authentication();
            announce(&app, repouso.snapshot());
        }
    });
}
