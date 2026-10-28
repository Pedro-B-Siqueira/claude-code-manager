//! What each session is doing right now, for the Jarvis balloons: the tool and target of the last
//! `PreToolUse`, the question the agent asked or the last line of its answer. Built only from hook
//! events the app already injects; nothing new is requested from Claude Code.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;

use crate::hooks::payload::{HookPayload, NotificationKind};
use crate::transcript::tools::{ToolInput, truncate_chars};

const ACTION_MAX_CHARS: usize = 160;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionKind {
    Tool,
    Permission,
    Question,
    Result,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionAction {
    pub kind: ActionKind,
    pub text: String,
}

impl SessionAction {
    pub fn new(kind: ActionKind, text: &str) -> Self {
        Self { kind, text: truncate_chars(text, ACTION_MAX_CHARS) }
    }
}

/// Sent as `session:action` when a session's action changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionUpdate {
    pub key: String,
    pub action: Option<SessionAction>,
    pub last_activity_at: i64,
}

pub fn file_name(path: &str) -> &str {
    Path::new(path).file_name().and_then(|name| name.to_str()).unwrap_or(path)
}

/// `Tool · target`, with only the file name for file tools: a full path rarely fits a balloon.
pub fn tool_action(tool: &str, input: Option<&ToolInput>) -> SessionAction {
    let file = input.and_then(|input| input.file_path.as_deref().or(input.notebook_path.as_deref()));
    let target = match file {
        Some(path) => Some(file_name(path).to_owned()),
        None => input.and_then(ToolInput::activity_target),
    };
    labeled_tool(tool, target.as_deref())
}

pub fn labeled_tool(tool: &str, target: Option<&str>) -> SessionAction {
    let text = match target {
        Some(target) => format!("{tool} · {target}"),
        None => tool.to_owned(),
    };
    SessionAction::new(ActionKind::Tool, &text)
}

/// The last non-empty line of an answer: usually its conclusion.
pub fn result_line(reply: &str) -> Option<String> {
    reply.lines().map(str::trim).rfind(|line| !line.is_empty()).map(str::to_owned)
}

/// The last line with a question mark, or the last line when the answer asks nothing.
pub fn question_line(reply: &str) -> Option<String> {
    reply.lines().map(str::trim).rfind(|line| line.contains('?')).map(str::to_owned).or_else(|| result_line(reply))
}

#[derive(Debug, Default)]
struct ActionState {
    action: Option<SessionAction>,
    /// The last answer of the agent, kept to quote its question when it starts waiting.
    reply: Option<String>,
    last_activity_at: i64,
}

enum ActionChange {
    Keep,
    Replace(Option<SessionAction>),
}

fn change_for(payload: &HookPayload, state: &mut ActionState) -> ActionChange {
    match payload.event() {
        "SessionStart" | "UserPromptSubmit" => {
            state.reply = None;
            ActionChange::Replace(None)
        }
        "PreToolUse" => payload.tool_name.as_deref().map_or(ActionChange::Keep, |tool| ActionChange::Replace(Some(tool_action(tool, payload.tool_input.as_ref())))),
        "PermissionRequest" => ActionChange::Replace(payload.requested_action().map(|text| SessionAction::new(ActionKind::Permission, &text))),
        "Stop" | "StopFailure" => {
            state.reply = payload.last_assistant_message.clone();
            ActionChange::Replace(state.reply.as_deref().and_then(result_line).map(|line| SessionAction::new(ActionKind::Result, &line)))
        }
        "Notification" => notification_change(payload, state),
        _ => ActionChange::Keep,
    }
}

fn notification_change(payload: &HookPayload, state: &ActionState) -> ActionChange {
    match payload.notification_kind() {
        NotificationKind::Idle | NotificationKind::NeedsInput => {
            let question = state.reply.as_deref().and_then(question_line).or_else(|| payload.message.clone());
            ActionChange::Replace(question.map(|text| SessionAction::new(ActionKind::Question, &text)))
        }
        // The notification follows PermissionRequest; keep the command that request carried.
        NotificationKind::Permission if state.action.as_ref().is_some_and(|action| action.kind == ActionKind::Permission) => ActionChange::Keep,
        NotificationKind::Permission => ActionChange::Replace(payload.message.as_deref().map(|text| SessionAction::new(ActionKind::Permission, text))),
        NotificationKind::Other => ActionChange::Keep,
    }
}

#[derive(Default)]
pub struct ActionTracker {
    entries: Mutex<HashMap<String, ActionState>>,
}

impl ActionTracker {
    /// Records a hook event; returns the new action when it changed. The time always moves on.
    pub fn record(&self, key: &str, payload: &HookPayload, now_ms: i64) -> Option<ActionUpdate> {
        let mut entries = self.entries.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let state = entries.entry(key.to_owned()).or_default();
        state.last_activity_at = now_ms;
        let ActionChange::Replace(action) = change_for(payload, state) else { return None };
        if state.action == action {
            return None;
        }
        state.action = action.clone();
        Some(ActionUpdate { key: key.to_owned(), action, last_activity_at: now_ms })
    }

    pub fn current(&self, key: &str) -> Option<(Option<SessionAction>, i64)> {
        let entries = self.entries.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        entries.get(key).map(|state| (state.action.clone(), state.last_activity_at))
    }

    pub fn forget(&self, key: &str) {
        self.entries.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::payload::HookPayload;

    fn hook(json: &str) -> HookPayload {
        serde_json::from_str(json).unwrap()
    }

    fn record(tracker: &ActionTracker, json: &str, now: i64) -> Option<ActionUpdate> {
        tracker.record("k", &hook(json), now)
    }

    fn text(tracker: &ActionTracker) -> Option<(ActionKind, String)> {
        tracker.current("k").and_then(|(action, _)| action).map(|action| (action.kind, action.text))
    }

    #[test]
    fn a_tool_call_shows_the_tool_and_the_file_name() {
        let tracker = ActionTracker::default();
        record(&tracker, r#"{"hook_event_name":"PreToolUse","tool_name":"Edit","tool_input":{"file_path":"/repo/src/timeline-filter.component.ts"}}"#, 10);
        assert_eq!(text(&tracker), Some((ActionKind::Tool, "Edit · timeline-filter.component.ts".to_owned())));
    }

    #[test]
    fn a_shell_call_shows_the_first_line_of_the_command() {
        let tracker = ActionTracker::default();
        record(&tracker, r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"yarn ngc --noEmit -p control-panel\necho done"}}"#, 10);
        assert_eq!(text(&tracker), Some((ActionKind::Tool, "Bash · yarn ngc --noEmit -p control-panel".to_owned())));
    }

    #[test]
    fn a_tool_without_a_target_shows_only_its_name() {
        let tracker = ActionTracker::default();
        record(&tracker, r#"{"hook_event_name":"PreToolUse","tool_name":"TodoWrite","tool_input":{}}"#, 10);
        assert_eq!(text(&tracker), Some((ActionKind::Tool, "TodoWrite".to_owned())));
    }

    #[test]
    fn a_finished_turn_shows_the_last_line_of_the_answer() {
        let tracker = ActionTracker::default();
        record(&tracker, r#"{"hook_event_name":"Stop","last_assistant_message":"Resumo:\n- testes passando\n\nPR 8 enviado (d60d09)\n"}"#, 10);
        assert_eq!(text(&tracker), Some((ActionKind::Result, "PR 8 enviado (d60d09)".to_owned())));
    }

    #[test]
    fn waiting_for_input_shows_the_question_of_the_last_answer() {
        let tracker = ActionTracker::default();
        record(&tracker, r#"{"hook_event_name":"Stop","last_assistant_message":"Rodei o seed.\nQuer que eu fique acompanhando e rode de novo?\nSe não, paro aqui."}"#, 10);
        record(&tracker, r#"{"hook_event_name":"Notification","notification_type":"idle_prompt","message":"Claude is waiting for your input"}"#, 20);
        assert_eq!(text(&tracker), Some((ActionKind::Question, "Quer que eu fique acompanhando e rode de novo?".to_owned())));
    }

    #[test]
    fn waiting_without_a_known_answer_falls_back_to_the_notification() {
        let tracker = ActionTracker::default();
        record(&tracker, r#"{"hook_event_name":"Notification","notification_type":"idle_prompt","message":"Claude is waiting for your input"}"#, 20);
        assert_eq!(text(&tracker), Some((ActionKind::Question, "Claude is waiting for your input".to_owned())));
    }

    #[test]
    fn a_permission_request_shows_what_it_wants_to_run_and_survives_its_notification() {
        let tracker = ActionTracker::default();
        record(&tracker, r#"{"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"rm -rf dist"}}"#, 10);
        record(&tracker, r#"{"hook_event_name":"Notification","notification_type":"permission_prompt","message":"Claude needs your permission to use Bash"}"#, 20);
        assert_eq!(text(&tracker), Some((ActionKind::Permission, "$ rm -rf dist".to_owned())));
    }

    #[test]
    fn a_new_prompt_clears_the_previous_action() {
        let tracker = ActionTracker::default();
        record(&tracker, r#"{"hook_event_name":"Stop","last_assistant_message":"Pronto."}"#, 10);
        record(&tracker, r#"{"hook_event_name":"UserPromptSubmit"}"#, 20);
        assert_eq!(text(&tracker), None);
        assert_eq!(tracker.current("k").map(|(_, at)| at), Some(20));
    }

    #[test]
    fn reports_a_change_only_when_the_action_changes_but_always_keeps_the_time() {
        let tracker = ActionTracker::default();
        let edit = r#"{"hook_event_name":"PreToolUse","tool_name":"Edit","tool_input":{"file_path":"/a/b.ts"}}"#;
        let first = record(&tracker, edit, 10).expect("first action is a change");
        assert_eq!(first.key, "k");
        assert_eq!(first.last_activity_at, 10);
        assert!(record(&tracker, r#"{"hook_event_name":"PostToolUse","tool_name":"Edit"}"#, 15).is_none());
        assert!(record(&tracker, edit, 20).is_none());
        assert_eq!(tracker.current("k").map(|(_, at)| at), Some(20));
    }

    #[test]
    fn forgets_a_closed_session() {
        let tracker = ActionTracker::default();
        record(&tracker, r#"{"hook_event_name":"UserPromptSubmit"}"#, 10);
        tracker.forget("k");
        assert!(tracker.current("k").is_none());
    }

    #[test]
    fn transcript_fallbacks_pick_the_same_lines() {
        assert_eq!(result_line("a\n\n  b  \n"), Some("b".to_owned()));
        assert_eq!(question_line("Posso seguir?\nOu prefere revisar antes?\nAguardo."), Some("Ou prefere revisar antes?".to_owned()));
        assert_eq!(question_line("Sem pergunta aqui."), Some("Sem pergunta aqui.".to_owned()));
        assert_eq!(result_line("   \n"), None);
    }
}
