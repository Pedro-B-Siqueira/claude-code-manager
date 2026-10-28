//! The card model the UI shows for each live session: terminal or registry state plus what the
//! library knows about the conversation.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::error::AppError;
use crate::library::models::SessionSummary;
use crate::library::{project, queries};
use crate::live::registry::{ExternalSession, RegistryStatus};
use crate::pty::{LaunchMode, SessionSnapshot, now_ms};
use crate::sessions::action::{ActionKind, SessionAction, file_name, labeled_tool, question_line, result_line};
use crate::status::{DONE_TO_IDLE_MS, SessionStatus, StatusUpdate};

/// Without hook data, recent terminal output is the only sign that an app session is working.
const RECENT_OUTPUT_MS: i64 = 1_500;
const CARD_FILES: usize = 6;
const EXTERNAL_PREVIEW_LINES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionOrigin {
    App,
    External,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChangeSummary {
    pub path: String,
    pub added: u32,
    pub removed: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSessionView {
    pub key: String,
    pub session_id: String,
    pub title: String,
    pub repo: String,
    pub cwd: String,
    pub branch: Option<String>,
    pub is_worktree: bool,
    pub status: SessionStatus,
    pub status_detail: Option<String>,
    pub preview_lines: Vec<String>,
    pub files: Vec<FileChangeSummary>,
    /// All files the session touched; `files` carries only the first few for the card.
    pub files_total: u32,
    pub context_percent: Option<f64>,
    /// Tokens Claude wrote. Totals with input are dominated by the cached context re-read on every call.
    pub output_tokens: u64,
    pub memory_mb: Option<f64>,
    pub origin: SessionOrigin,
    pub pid: Option<u32>,
    pub hibernated: bool,
    pub pinned: bool,
    pub exited: bool,
    pub started_at: i64,
    /// What the agent is doing now (Jarvis balloons); the cards do not show it.
    pub action: Option<SessionAction>,
    pub last_activity_at: Option<i64>,
}

/// The latest tool call recorded in the transcript, for sessions that send no hook events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastTool {
    pub tool: String,
    pub target: Option<String>,
}

/// Tools whose activity target is a file path; for the rest it is a command, pattern or URL.
const FILE_TOOLS: [&str; 5] = ["Edit", "Write", "MultiEdit", "Read", "NotebookEdit"];

pub fn transcript_action(status: SessionStatus, last_reply: Option<&str>, last_tool: Option<&LastTool>) -> Option<SessionAction> {
    match status {
        SessionStatus::Working => last_tool.map(|last| {
            let target = last.target.as_deref().map(|target| if FILE_TOOLS.contains(&last.tool.as_str()) { file_name(target) } else { target });
            labeled_tool(&last.tool, target)
        }),
        SessionStatus::Waiting => last_reply.and_then(question_line).map(|line| SessionAction::new(ActionKind::Question, &line)),
        SessionStatus::Done | SessionStatus::Idle => last_reply.and_then(result_line).map(|line| SessionAction::new(ActionKind::Result, &line)),
        SessionStatus::Permission => None,
    }
}

fn last_tool(connection: &Connection, session_id: &str) -> Result<Option<LastTool>, AppError> {
    Ok(connection
        .query_row(
            "SELECT tool, target FROM activity WHERE session_id = ?1 AND kind IN ('tool', 'edit') AND tool IS NOT NULL ORDER BY id DESC LIMIT 1",
            params![session_id],
            |row| Ok(LastTool { tool: row.get(0)?, target: row.get(1)? }),
        )
        .optional()?)
}

struct ProjectFacts {
    name: String,
    repo_root: Option<String>,
}

fn project_facts(connection: &Connection, cwd: &str) -> Result<ProjectFacts, AppError> {
    if !cwd.is_empty() {
        project::ensure_project(connection, cwd)?;
    }
    let facts = connection
        .query_row("SELECT name, repo_root FROM projects WHERE cwd = ?1", params![cwd], |row| {
            Ok(ProjectFacts { name: row.get(0)?, repo_root: row.get(1)? })
        })
        .optional()?;
    Ok(facts.unwrap_or_else(|| ProjectFacts { name: folder_name(cwd), repo_root: None }))
}

fn folder_name(cwd: &str) -> String {
    Path::new(cwd).file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_else(|| cwd.to_owned())
}

/// Library-derived fields shared by app and external cards.
struct ConversationFacts {
    summary: Option<SessionSummary>,
    project: ProjectFacts,
    cwd: String,
    last_tool: Option<LastTool>,
}

impl ConversationFacts {
    fn load(connection: &Connection, session_id: &str, cwd: &str) -> Result<Self, AppError> {
        let summary = queries::session_summary(connection, session_id)?;
        Ok(Self { project: project_facts(connection, cwd)?, summary, cwd: cwd.to_owned(), last_tool: last_tool(connection, session_id)? })
    }

    /// Hook events win; without them the transcript tells what the session did last.
    fn activity(&self, base: &ViewBase) -> (Option<SessionAction>, Option<i64>) {
        if base.hibernated || base.exited {
            return (None, None);
        }
        let summary = self.summary.as_ref();
        match &base.hook_activity {
            Some((action, at)) => (action.clone(), Some(*at)),
            None => (
                transcript_action(base.status.status, summary.and_then(|summary| summary.last_assistant.as_deref()), self.last_tool.as_ref()),
                summary.and_then(|summary| summary.item.updated_at),
            ),
        }
    }

    fn view(self, base: ViewBase) -> LiveSessionView {
        let (action, last_activity_at) = self.activity(&base);
        let summary = self.summary.as_ref();
        LiveSessionView {
            key: base.key,
            session_id: base.session_id,
            title: summary.map(|summary| summary.item.title.clone()).unwrap_or(base.fallback_title),
            repo: self.project.name,
            is_worktree: self.project.repo_root.as_deref().is_some_and(|root| root != self.cwd),
            branch: summary.and_then(|summary| summary.item.branch.clone()),
            status: base.status.status,
            status_detail: base.status.detail,
            preview_lines: base.preview_lines.unwrap_or_else(|| last_assistant_lines(summary)),
            files: card_files(summary),
            files_total: summary.map_or(0, |summary| u32::try_from(summary.files.len()).unwrap_or(u32::MAX)),
            context_percent: summary.and_then(|summary| summary.context_percent),
            output_tokens: summary.map_or(0, |summary| summary.usage.output),
            memory_mb: base.memory_mb,
            origin: base.origin,
            pid: base.pid,
            hibernated: base.hibernated,
            pinned: summary.is_some_and(|summary| summary.item.pinned),
            exited: base.exited,
            started_at: base.started_at,
            action,
            last_activity_at,
            cwd: self.cwd,
        }
    }
}

struct ViewBase {
    key: String,
    session_id: String,
    fallback_title: String,
    status: StatusUpdate,
    preview_lines: Option<Vec<String>>,
    memory_mb: Option<f64>,
    origin: SessionOrigin,
    pid: Option<u32>,
    hibernated: bool,
    exited: bool,
    started_at: i64,
    /// Action and time from hook events; None for sessions that send none.
    hook_activity: Option<(Option<SessionAction>, i64)>,
}

fn interim_status(snapshot: &SessionSnapshot) -> StatusUpdate {
    let status = if snapshot.exited {
        SessionStatus::Idle
    } else if now_ms() - snapshot.last_output_at < RECENT_OUTPUT_MS {
        SessionStatus::Working
    } else {
        SessionStatus::Waiting
    };
    StatusUpdate { status, detail: None }
}

pub fn build_app_view(
    connection: &Connection,
    snapshot: &SessionSnapshot,
    hook_status: Option<StatusUpdate>,
    hook_activity: Option<(Option<SessionAction>, i64)>,
    memory_mb: Option<f64>,
) -> Result<LiveSessionView, AppError> {
    let cwd = snapshot.launch.cwd.to_string_lossy().into_owned();
    let status = match hook_status {
        _ if snapshot.hibernated => StatusUpdate { status: SessionStatus::Idle, detail: None },
        Some(_) if snapshot.exited => StatusUpdate { status: SessionStatus::Idle, detail: None },
        Some(status) => status,
        None => interim_status(snapshot),
    };
    let fallback_title = match snapshot.launch.mode {
        LaunchMode::New => "Nova sessão",
        LaunchMode::Resume => "Sessão retomada",
    };
    let facts = ConversationFacts::load(connection, &snapshot.launch.session_id, &cwd)?;
    Ok(facts.view(ViewBase {
        key: snapshot.key.clone(),
        session_id: snapshot.launch.session_id.clone(),
        fallback_title: fallback_title.to_owned(),
        status,
        preview_lines: Some(snapshot.preview_lines.clone()),
        memory_mb: if snapshot.exited { None } else { memory_mb },
        origin: SessionOrigin::App,
        pid: snapshot.pid,
        hibernated: snapshot.hibernated,
        exited: snapshot.exited && !snapshot.hibernated,
        started_at: snapshot.started_at,
        hook_activity,
    }))
}

pub fn external_status(session: &ExternalSession, now_ms: i64) -> SessionStatus {
    match session.status {
        RegistryStatus::Busy => SessionStatus::Working,
        RegistryStatus::NeedsPermission => SessionStatus::Permission,
        RegistryStatus::NeedsInput => SessionStatus::Waiting,
        RegistryStatus::Idle if now_ms - session.status_updated_at <= DONE_TO_IDLE_MS => SessionStatus::Done,
        RegistryStatus::Idle | RegistryStatus::Unknown => SessionStatus::Idle,
    }
}

pub fn build_external_view(connection: &Connection, session: &ExternalSession, memory_mb: Option<f64>) -> Result<LiveSessionView, AppError> {
    let facts = ConversationFacts::load(connection, &session.session_id, &session.cwd)?;
    let started_at = facts.summary.as_ref().and_then(|summary| summary.item.started_at).unwrap_or(session.status_updated_at);
    Ok(facts.view(ViewBase {
        key: format!("external:{}", session.pid),
        session_id: session.session_id.clone(),
        fallback_title: "Sessão em outro terminal".to_owned(),
        status: StatusUpdate { status: external_status(session, now_ms()), detail: None },
        preview_lines: None,
        memory_mb,
        origin: SessionOrigin::External,
        pid: Some(session.pid),
        hibernated: false,
        exited: false,
        started_at,
        hook_activity: None,
    }))
}

fn last_assistant_lines(summary: Option<&SessionSummary>) -> Vec<String> {
    let Some(text) = summary.and_then(|summary| summary.last_assistant.as_deref()) else { return Vec::new() };
    let lines: Vec<String> = text.lines().map(str::trim_end).filter(|line| !line.is_empty()).map(str::to_owned).collect();
    let start = lines.len().saturating_sub(EXTERNAL_PREVIEW_LINES);
    lines[start..].to_vec()
}

fn card_files(summary: Option<&SessionSummary>) -> Vec<FileChangeSummary> {
    let Some(summary) = summary else { return Vec::new() };
    summary
        .files
        .iter()
        .take(CARD_FILES)
        .map(|file| FileChangeSummary { path: file.path.clone(), added: file.added, removed: file.removed })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn external(status: RegistryStatus, updated_at: i64) -> ExternalSession {
        ExternalSession { pid: 10, session_id: "s".to_owned(), cwd: "/r".to_owned(), status, status_updated_at: updated_at }
    }

    fn tool(name: &str, target: &str) -> Option<LastTool> {
        Some(LastTool { tool: name.to_owned(), target: Some(target.to_owned()) })
    }

    fn action_text(action: Option<SessionAction>) -> Option<(ActionKind, String)> {
        action.map(|action| (action.kind, action.text))
    }

    #[test]
    fn transcripts_give_working_sessions_their_last_tool() {
        let edit = transcript_action(SessionStatus::Working, None, tool("Edit", "/repo/src/a.component.ts").as_ref());
        assert_eq!(action_text(edit), Some((ActionKind::Tool, "Edit · a.component.ts".to_owned())));
        let shell = transcript_action(SessionStatus::Working, None, tool("Bash", "cat /etc/hosts").as_ref());
        assert_eq!(action_text(shell), Some((ActionKind::Tool, "Bash · cat /etc/hosts".to_owned())));
    }

    #[test]
    fn transcripts_give_finished_and_waiting_sessions_a_line_of_the_last_answer() {
        let reply = Some("Rodei tudo.\nPosso abrir o PR?\nFico no aguardo.");
        assert_eq!(action_text(transcript_action(SessionStatus::Done, reply, None)), Some((ActionKind::Result, "Fico no aguardo.".to_owned())));
        assert_eq!(action_text(transcript_action(SessionStatus::Waiting, reply, None)), Some((ActionKind::Question, "Posso abrir o PR?".to_owned())));
        assert_eq!(action_text(transcript_action(SessionStatus::Idle, reply, None)), Some((ActionKind::Result, "Fico no aguardo.".to_owned())));
        assert_eq!(transcript_action(SessionStatus::Permission, reply, None), None);
        assert_eq!(transcript_action(SessionStatus::Working, None, None), None);
    }

    #[test]
    fn the_last_tool_includes_edits_and_skips_prompts() {
        let database = crate::db::Database::open_in_memory().unwrap();
        let connection = database.connection();
        connection
            .execute_batch(
                "INSERT INTO activity (session_id, kind, tool, target) VALUES
                    ('s', 'tool', 'Bash', 'npm test'),
                    ('s', 'edit', 'Edit', '/repo/a.ts'),
                    ('s', 'prompt', NULL, 'next step'),
                    ('other', 'tool', 'Read', 'b.ts');",
            )
            .unwrap();
        assert_eq!(last_tool(&connection, "s").unwrap(), tool("Edit", "/repo/a.ts"));
        assert_eq!(last_tool(&connection, "missing").unwrap(), None);
    }

    #[test]
    fn external_registry_status_maps_to_card_status() {
        assert_eq!(external_status(&external(RegistryStatus::Busy, 0), 0), SessionStatus::Working);
        assert_eq!(external_status(&external(RegistryStatus::NeedsPermission, 0), 0), SessionStatus::Permission);
        assert_eq!(external_status(&external(RegistryStatus::Idle, 1_000), 2_000), SessionStatus::Done);
        assert_eq!(external_status(&external(RegistryStatus::Idle, 0), DONE_TO_IDLE_MS + 1), SessionStatus::Idle);
    }
}
