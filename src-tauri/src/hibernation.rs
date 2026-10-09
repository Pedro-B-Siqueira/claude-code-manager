//! Frees memory by ending app sessions that have been idle for a while (configurable, on by
//! default). A hibernated session is resumed with `claude --resume` when the user opens it again.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::pty::now_ms;
use crate::settings;
use crate::state::AppState;
use crate::status::SessionStatus;

const CHECK_INTERVAL: Duration = Duration::from_secs(30);
const HIBERNATE_GRACE: Duration = Duration::from_secs(5);

/// Only sessions nobody is waiting on: never while working, asking for permission or waiting for
/// the user. Without hook data, the absence of terminal output is the only signal.
pub fn should_hibernate(status: Option<SessionStatus>, last_activity_ms: i64, now_ms: i64, idle_ms: i64) -> bool {
    let quiet_for_long = now_ms - last_activity_ms >= idle_ms;
    let idle_status = match status {
        Some(SessionStatus::Idle | SessionStatus::Done) | None => true,
        Some(SessionStatus::Working | SessionStatus::Permission | SessionStatus::Waiting) => false,
    };
    quiet_for_long && idle_status
}

pub fn start(app: AppHandle, on_change: Arc<dyn Fn() + Send + Sync>) {
    let spawned = thread::Builder::new().name("hibernation".to_owned()).spawn(move || loop {
        thread::sleep(CHECK_INTERVAL);
        let Some(state) = app.try_state::<AppState>() else { continue };
        if hibernate_idle_sessions(&state) > 0 {
            on_change();
        }
    });
    if let Err(error) = spawned {
        log::warn!("hibernação indisponível: {error}");
    }
}

fn hibernate_idle_sessions(state: &AppState) -> usize {
    let Ok(preferences) = settings::load(&state.database).map(|settings| settings.hibernation) else { return 0 };
    if !preferences.enabled {
        return 0;
    }
    let idle_ms = i64::from(preferences.idle_minutes) * 60_000;
    let now = now_ms();
    let mut hibernated = 0;
    for snapshot in state.pty.snapshots().into_iter().filter(|snapshot| !snapshot.exited && !snapshot.hibernated) {
        let tracked = state.status.current_with_time(&snapshot.key, now);
        let last_activity = tracked.as_ref().map_or(snapshot.last_output_at, |(_, changed_at)| snapshot.last_output_at.max(*changed_at));
        let status = tracked.map(|(update, _)| update.status);
        if should_hibernate(status, last_activity, now, idle_ms) && state.pty.hibernate(&snapshot.key, HIBERNATE_GRACE).is_ok() {
            log::info!("sessão {} hibernada após {} min sem atividade", snapshot.launch.session_id, preferences.idle_minutes);
            hibernated += 1;
        }
    }
    hibernated
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDLE: i64 = 30 * 60_000;

    #[test]
    fn hibernates_only_quiet_idle_sessions() {
        assert!(should_hibernate(Some(SessionStatus::Idle), 0, IDLE, IDLE));
        assert!(should_hibernate(Some(SessionStatus::Done), 0, IDLE + 1, IDLE));
        assert!(should_hibernate(None, 0, IDLE, IDLE));
        assert!(!should_hibernate(Some(SessionStatus::Idle), 1, IDLE, IDLE), "not quiet long enough");
    }

    #[test]
    fn never_hibernates_sessions_that_need_attention_or_are_busy() {
        for status in [SessionStatus::Working, SessionStatus::Permission, SessionStatus::Waiting] {
            assert!(!should_hibernate(Some(status), 0, IDLE * 10, IDLE), "{status:?}");
        }
    }
}
