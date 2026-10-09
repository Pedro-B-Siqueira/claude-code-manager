//! Session status from hook events (sessions opened by the app).

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;

use crate::hooks::payload::{HookPayload, NotificationKind};

/// A finished turn left untouched this long is shown as idle.
pub const DONE_TO_IDLE_MS: i64 = 15 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Working,
    Permission,
    Waiting,
    Done,
    Idle,
}

impl SessionStatus {
    pub fn needs_user(self) -> bool {
        matches!(self, Self::Permission | Self::Waiting)
    }

    pub fn is_busy(self) -> bool {
        matches!(self, Self::Working | Self::Permission)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusUpdate {
    pub status: SessionStatus,
    pub detail: Option<String>,
}

pub fn status_for_hook(payload: &HookPayload) -> Option<StatusUpdate> {
    let status = match payload.event() {
        "SessionStart" | "SessionEnd" => SessionStatus::Idle,
        "UserPromptSubmit" | "PreToolUse" | "PostToolUse" => SessionStatus::Working,
        "PermissionRequest" => SessionStatus::Permission,
        "Stop" | "StopFailure" => SessionStatus::Done,
        "Notification" => match payload.notification_kind() {
            NotificationKind::Permission => SessionStatus::Permission,
            NotificationKind::Idle | NotificationKind::NeedsInput => SessionStatus::Waiting,
            NotificationKind::Other => return None,
        },
        _ => return None,
    };
    let detail = (status == SessionStatus::Permission).then(|| payload.requested_action()).flatten();
    Some(StatusUpdate { status, detail })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusTransition {
    pub key: String,
    pub from: Option<SessionStatus>,
    pub to: SessionStatus,
    pub detail: Option<String>,
}

#[derive(Debug, Clone)]
struct TrackedStatus {
    status: SessionStatus,
    detail: Option<String>,
    changed_at: i64,
}

#[derive(Default)]
pub struct StatusTracker {
    entries: Mutex<HashMap<String, TrackedStatus>>,
}

impl StatusTracker {
    pub fn apply(&self, key: &str, update: StatusUpdate, now_ms: i64) -> Option<StatusTransition> {
        let mut entries = self.entries.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous = entries.get(key).cloned();
        // The permission notification arrives after PermissionRequest; keep the command it carried.
        let detail = match (&update, &previous) {
            (StatusUpdate { status: SessionStatus::Permission, detail: None }, Some(previous)) if previous.status == SessionStatus::Permission => previous.detail.clone(),
            _ => update.detail,
        };
        let unchanged = previous.as_ref().is_some_and(|previous| previous.status == update.status && previous.detail == detail);
        let changed_at = if unchanged { previous.as_ref().map_or(now_ms, |previous| previous.changed_at) } else { now_ms };
        entries.insert(key.to_owned(), TrackedStatus { status: update.status, detail: detail.clone(), changed_at });
        (!unchanged).then(|| StatusTransition { key: key.to_owned(), from: previous.map(|previous| previous.status), to: update.status, detail })
    }

    pub fn current(&self, key: &str, now_ms: i64) -> Option<StatusUpdate> {
        let entries = self.entries.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let tracked = entries.get(key)?;
        let status = if tracked.status == SessionStatus::Done && now_ms - tracked.changed_at > DONE_TO_IDLE_MS {
            SessionStatus::Idle
        } else {
            tracked.status
        };
        Some(StatusUpdate { status, detail: tracked.detail.clone() })
    }

    /// The current status together with when it last changed.
    pub fn current_with_time(&self, key: &str, now_ms: i64) -> Option<(StatusUpdate, i64)> {
        let changed_at = self.entries.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).get(key)?.changed_at;
        self.current(key, now_ms).map(|update| (update, changed_at))
    }

    pub fn forget(&self, key: &str) {
        self.entries.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hook(json: &str) -> HookPayload {
        serde_json::from_str(json).unwrap()
    }

    fn apply(tracker: &StatusTracker, json: &str, now: i64) -> Option<StatusTransition> {
        status_for_hook(&hook(json)).and_then(|update| tracker.apply("k", update, now))
    }

    #[test]
    fn maps_hook_events_to_statuses() {
        let cases = [
            (r#"{"hook_event_name":"SessionStart"}"#, Some(SessionStatus::Idle)),
            (r#"{"hook_event_name":"UserPromptSubmit"}"#, Some(SessionStatus::Working)),
            (r#"{"hook_event_name":"PreToolUse","tool_name":"Read"}"#, Some(SessionStatus::Working)),
            (r#"{"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"rm -rf dist"}}"#, Some(SessionStatus::Permission)),
            (r#"{"hook_event_name":"Notification","notification_type":"idle_prompt"}"#, Some(SessionStatus::Waiting)),
            (r#"{"hook_event_name":"Stop"}"#, Some(SessionStatus::Done)),
            (r#"{"hook_event_name":"Notification","notification_type":"auth_success"}"#, None),
            (r#"{"hook_event_name":"FileChanged"}"#, None),
        ];
        for (json, expected) in cases {
            assert_eq!(status_for_hook(&hook(json)).map(|update| update.status), expected, "{json}");
        }
    }

    #[test]
    fn a_turn_goes_working_permission_working_done() {
        let tracker = StatusTracker::default();
        assert_eq!(apply(&tracker, r#"{"hook_event_name":"UserPromptSubmit"}"#, 1).map(|t| t.to), Some(SessionStatus::Working));
        assert!(apply(&tracker, r#"{"hook_event_name":"PreToolUse"}"#, 2).is_none(), "same status is not a transition");
        let permission = apply(&tracker, r#"{"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"npm publish"}}"#, 3).unwrap();
        assert_eq!((permission.to, permission.detail.as_deref()), (SessionStatus::Permission, Some("$ npm publish")));
        assert!(apply(&tracker, r#"{"hook_event_name":"Notification","notification_type":"permission_prompt","message":"needs permission"}"#, 4).is_none());
        assert_eq!(tracker.current("k", 5).unwrap().detail.as_deref(), Some("$ npm publish"), "notification keeps the command");
        assert_eq!(apply(&tracker, r#"{"hook_event_name":"PostToolUse"}"#, 6).map(|t| t.to), Some(SessionStatus::Working));
        let done = apply(&tracker, r#"{"hook_event_name":"Stop"}"#, 7).unwrap();
        assert_eq!((done.from, done.to), (Some(SessionStatus::Working), SessionStatus::Done));
    }

    #[test]
    fn finished_sessions_turn_idle_after_a_while() {
        let tracker = StatusTracker::default();
        apply(&tracker, r#"{"hook_event_name":"Stop"}"#, 0);
        assert_eq!(tracker.current("k", DONE_TO_IDLE_MS).unwrap().status, SessionStatus::Done);
        assert_eq!(tracker.current("k", DONE_TO_IDLE_MS + 1).unwrap().status, SessionStatus::Idle);
    }
}
