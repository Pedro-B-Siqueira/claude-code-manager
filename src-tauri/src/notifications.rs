//! Native notifications for sessions the app opened. With ClaudeGauge's hook installed they are
//! off by default (`Auto`), so the same event is never announced twice.

use std::sync::Mutex;

use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::claudegauge::ClaudeGaugeStatus;
use crate::pty::now_ms;
use crate::settings::NotificationPreference;
use crate::status::{SessionStatus, StatusTransition};

/// Activating the app this soon after a notification opens the session it was about.
const FOCUS_WINDOW_MS: i64 = 5 * 60 * 1000;

pub fn enabled(preference: NotificationPreference, claudegauge: ClaudeGaugeStatus) -> bool {
    match preference {
        NotificationPreference::Auto => !claudegauge.hook_installed,
        NotificationPreference::Enabled => true,
        NotificationPreference::Disabled => false,
    }
}

pub fn message_for(transition: &StatusTransition) -> Option<String> {
    match transition.to {
        SessionStatus::Permission => Some(match &transition.detail {
            Some(detail) => format!("Pedindo permissão: {detail}"),
            None => "Pedindo permissão".to_owned(),
        }),
        SessionStatus::Waiting => Some("Esperando você".to_owned()),
        SessionStatus::Done if transition.from == Some(SessionStatus::Working) => Some("Terminou".to_owned()),
        _ => None,
    }
}

#[derive(Default)]
pub struct NotificationCenter {
    last_notified: Mutex<Option<(String, i64)>>,
}

impl NotificationCenter {
    pub fn announce(&self, app: &AppHandle, title: &str, transition: &StatusTransition) {
        let Some(body) = message_for(transition) else { return };
        let window_focused = app.get_webview_window("main").and_then(|window| window.is_focused().ok()).unwrap_or(false);
        if window_focused {
            return;
        }
        if let Err(error) = app.notification().builder().title(title).body(body).show() {
            log::warn!("falha ao mostrar notificação: {error}");
            return;
        }
        *self.last_notified.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some((transition.key.clone(), now_ms()));
    }

    /// The session the most recent notification was about, consumed once.
    pub fn take_recent(&self) -> Option<String> {
        let mut last = self.last_notified.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let (key, at) = last.take()?;
        (now_ms() - at <= FOCUS_WINDOW_MS).then_some(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transition(from: Option<SessionStatus>, to: SessionStatus, detail: Option<&str>) -> StatusTransition {
        StatusTransition { key: "k".to_owned(), from, to, detail: detail.map(str::to_owned) }
    }

    #[test]
    fn auto_mode_defers_to_claudegauge() {
        let with_gauge = ClaudeGaugeStatus { app_installed: true, hook_installed: true };
        assert!(!enabled(NotificationPreference::Auto, with_gauge));
        assert!(enabled(NotificationPreference::Auto, ClaudeGaugeStatus::default()));
        assert!(enabled(NotificationPreference::Enabled, with_gauge));
        assert!(!enabled(NotificationPreference::Disabled, ClaudeGaugeStatus::default()));
    }

    #[test]
    fn messages_for_the_three_announced_moments() {
        assert_eq!(message_for(&transition(None, SessionStatus::Permission, Some("$ npm test"))).as_deref(), Some("Pedindo permissão: $ npm test"));
        assert_eq!(message_for(&transition(None, SessionStatus::Waiting, None)).as_deref(), Some("Esperando você"));
        assert_eq!(message_for(&transition(Some(SessionStatus::Working), SessionStatus::Done, None)).as_deref(), Some("Terminou"));
        assert_eq!(message_for(&transition(Some(SessionStatus::Idle), SessionStatus::Done, None)), None);
        assert_eq!(message_for(&transition(None, SessionStatus::Working, None)), None);
    }
}
