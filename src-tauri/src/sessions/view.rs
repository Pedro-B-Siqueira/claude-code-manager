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
    pub total_tokens: u64,
    pub cost_usd: f64,
    pub memory_mb: Option<f64>,
    pub origin: SessionOrigin,
    pub pid: Option<u32>,
    pub hibernated: bool,
    pub pinned: bool,
    pub exited: bool,
    pub started_at: i64,
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
}

impl ConversationFacts {
    fn load(connection: &Connection, session_id: &str, cwd: &str) -> Result<Self, AppError> {
        let summary = queries::session_summary(connection, session_id)?;
        Ok(Self { project: project_facts(connection, cwd)?, summary, cwd: cwd.to_owned() })
    }

    fn view(self, base: ViewBase) -> LiveSessionView {
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
            total_tokens: summary.map_or(0, |summary| summary.usage.total()),
            cost_usd: summary.map_or(0.0, |summary| summary.item.cost_usd),
            memory_mb: base.memory_mb,
            origin: base.origin,
            pid: base.pid,
            hibernated: base.hibernated,
            pinned: summary.is_some_and(|summary| summary.item.pinned),
            exited: base.exited,
            started_at: base.started_at,
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

    #[test]
    fn external_registry_status_maps_to_card_status() {
        assert_eq!(external_status(&external(RegistryStatus::Busy, 0), 0), SessionStatus::Working);
        assert_eq!(external_status(&external(RegistryStatus::NeedsPermission, 0), 0), SessionStatus::Permission);
        assert_eq!(external_status(&external(RegistryStatus::Idle, 1_000), 2_000), SessionStatus::Done);
        assert_eq!(external_status(&external(RegistryStatus::Idle, 0), DONE_TO_IDLE_MS + 1), SessionStatus::Idle);
    }
}
