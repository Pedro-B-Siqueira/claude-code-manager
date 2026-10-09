//! The fields of Claude Code hook input the app uses. Unknown fields are ignored.

use serde::Deserialize;

use crate::transcript::lenient;
use crate::transcript::tools::{ToolInput, truncate_chars};

const DETAIL_MAX_CHARS: usize = 140;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct HookPayload {
    #[serde(deserialize_with = "lenient::string")]
    pub hook_event_name: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    pub session_id: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    pub notification_type: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    pub message: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    pub tool_name: Option<String>,
    pub tool_input: Option<ToolInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    Permission,
    Idle,
    NeedsInput,
    Other,
}

impl HookPayload {
    pub fn event(&self) -> &str {
        self.hook_event_name.as_deref().unwrap_or_default()
    }

    /// `notification_type` when present; otherwise inferred from the message text.
    pub fn notification_kind(&self) -> NotificationKind {
        let kind = self.notification_type.as_deref().unwrap_or_default();
        let message = self.message.as_deref().unwrap_or_default().to_lowercase();
        if kind == "permission_prompt" || message.contains("permission") {
            NotificationKind::Permission
        } else if kind == "idle_prompt" || message.contains("waiting for your input") {
            NotificationKind::Idle
        } else if kind.starts_with("elicitation") || kind == "agent_needs_input" {
            NotificationKind::NeedsInput
        } else {
            NotificationKind::Other
        }
    }

    /// What the session is asking to run: `$ command` for shell tools, `Tool target` otherwise.
    pub fn requested_action(&self) -> Option<String> {
        let tool = self.tool_name.as_deref()?;
        let input = self.tool_input.as_ref();
        let command = input.and_then(|input| input.command.as_deref()).and_then(|command| command.lines().next());
        let detail = match command {
            Some(command) => format!("$ {}", command.trim()),
            None => match input.and_then(ToolInput::activity_target) {
                Some(target) => format!("{tool} {target}"),
                None => tool.to_owned(),
            },
        };
        Some(truncate_chars(&detail, DETAIL_MAX_CHARS))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> HookPayload {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn shell_requests_show_the_command() {
        let payload = parse(r#"{"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"npm test\nsecond"}}"#);
        assert_eq!(payload.requested_action().as_deref(), Some("$ npm test"));
    }

    #[test]
    fn other_tools_show_their_target() {
        let payload = parse(r#"{"tool_name":"Edit","tool_input":{"file_path":"/repo/a.ts","old_string":"x","new_string":"y"}}"#);
        assert_eq!(payload.requested_action().as_deref(), Some("Edit /repo/a.ts"));
        assert_eq!(parse(r#"{"tool_name":"WebSearch"}"#).requested_action().as_deref(), Some("WebSearch"));
    }

    #[test]
    fn notification_kind_from_type_or_message() {
        assert_eq!(parse(r#"{"notification_type":"permission_prompt"}"#).notification_kind(), NotificationKind::Permission);
        assert_eq!(parse(r#"{"message":"Claude needs your permission to use Bash"}"#).notification_kind(), NotificationKind::Permission);
        assert_eq!(parse(r#"{"message":"Claude is waiting for your input"}"#).notification_kind(), NotificationKind::Idle);
        assert_eq!(parse(r#"{"notification_type":"elicitation_dialog"}"#).notification_kind(), NotificationKind::NeedsInput);
        assert_eq!(parse(r#"{"notification_type":"auth_success"}"#).notification_kind(), NotificationKind::Other);
    }

    #[test]
    fn unknown_fields_and_odd_types_are_tolerated() {
        let payload = parse(r#"{"hook_event_name":"Stop","stop_hook_active":false,"session_id":7,"extra":{"a":[1]}}"#);
        assert_eq!(payload.event(), "Stop");
        assert_eq!(payload.session_id, None);
    }
}
