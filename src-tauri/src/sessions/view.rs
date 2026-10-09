//! The card model the UI shows for each live session: terminal state plus what the library knows.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::error::AppError;
use crate::library::models::SessionSummary;
use crate::library::{project, queries};
use crate::pty::{LaunchMode, SessionSnapshot, now_ms};

/// Without hook data yet, recent terminal output is the only sign that a session is working.
const RECENT_OUTPUT_MS: i64 = 1_500;
const CARD_FILES: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Working,
    Permission,
    Waiting,
    Done,
    Idle,
}

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
    pub context_percent: Option<f64>,
    pub total_tokens: u64,
    pub cost_usd: f64,
    pub memory_mb: Option<f64>,
    pub origin: SessionOrigin,
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
    project::ensure_project(connection, cwd)?;
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

fn interim_status(snapshot: &SessionSnapshot) -> SessionStatus {
    if snapshot.exited {
        SessionStatus::Idle
    } else if now_ms() - snapshot.last_output_at < RECENT_OUTPUT_MS {
        SessionStatus::Working
    } else {
        SessionStatus::Waiting
    }
}

fn fallback_title(mode: LaunchMode) -> String {
    match mode {
        LaunchMode::New => "Nova sessão".to_owned(),
        LaunchMode::Resume => "Sessão retomada".to_owned(),
    }
}

pub fn build_view(connection: &Connection, snapshot: &SessionSnapshot) -> Result<LiveSessionView, AppError> {
    let cwd = snapshot.launch.cwd.to_string_lossy().into_owned();
    let summary = queries::session_summary(connection, &snapshot.launch.session_id)?;
    let project = project_facts(connection, &cwd)?;
    let is_worktree = project.repo_root.as_deref().is_some_and(|root| root != cwd);
    Ok(LiveSessionView {
        key: snapshot.key.clone(),
        session_id: snapshot.launch.session_id.clone(),
        title: summary.as_ref().map(|summary| summary.item.title.clone()).unwrap_or_else(|| fallback_title(snapshot.launch.mode)),
        repo: project.name,
        branch: summary.as_ref().and_then(|summary| summary.item.branch.clone()),
        is_worktree,
        status: interim_status(snapshot),
        status_detail: None,
        preview_lines: snapshot.preview_lines.clone(),
        files: card_files(summary.as_ref()),
        context_percent: summary.as_ref().and_then(|summary| summary.context_percent),
        total_tokens: summary.as_ref().map_or(0, |summary| summary.usage.total()),
        cost_usd: summary.as_ref().map_or(0.0, |summary| summary.item.cost_usd),
        memory_mb: None,
        origin: SessionOrigin::App,
        hibernated: false,
        pinned: summary.as_ref().is_some_and(|summary| summary.item.pinned),
        exited: snapshot.exited,
        started_at: snapshot.started_at,
        cwd,
    })
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
