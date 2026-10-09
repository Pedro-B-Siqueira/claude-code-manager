//! Assembles the live session list: sessions the app opened (status from hooks) plus sessions
//! running elsewhere (Claude Code's registry, or the process table as a fallback).

use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension, params};

use super::processes::{ProcessTable, process_cwd};
use super::registry::{ExternalSession, RegistryStatus, read_registry};
use crate::error::AppError;
use crate::pty::{SessionSnapshot, now_ms};
use crate::sessions::view::{LiveSessionView, build_app_view, build_external_view};
use crate::state::AppState;

/// Without the registry, a transcript written this recently means the session is working.
const RECENT_TRANSCRIPT_MS: i64 = 10_000;

pub fn live_views(state: &AppState) -> Result<Vec<LiveSessionView>, AppError> {
    let table = ProcessTable::capture();
    let connection = state.database.connection();
    let snapshots = state.pty.snapshots();
    let now = now_ms();
    let mut views = Vec::with_capacity(snapshots.len());
    for snapshot in &snapshots {
        let memory = snapshot.pid.and_then(|pid| table.tree_rss_mb(pid));
        views.push(build_app_view(&connection, snapshot, state.status.current(&snapshot.key, now), memory)?);
    }
    for session in external_sessions(state, &connection, &table, &snapshots)? {
        views.push(build_external_view(&connection, &session, table.tree_rss_mb(session.pid))?);
    }
    Ok(views)
}

fn external_sessions(state: &AppState, connection: &Connection, table: &ProcessTable, snapshots: &[SessionSnapshot]) -> Result<Vec<ExternalSession>, AppError> {
    let app_session_ids: HashSet<&str> = snapshots.iter().filter(|snapshot| !snapshot.exited).map(|snapshot| snapshot.launch.session_id.as_str()).collect();
    let app_shell_pids: Vec<u32> = snapshots.iter().filter_map(|snapshot| snapshot.pid).collect();
    let started_by_app = |session: &ExternalSession| {
        app_session_ids.contains(session.session_id.as_str()) || app_shell_pids.iter().any(|shell| table.is_descendant_of(session.pid, *shell))
    };
    let registry_available = state.paths.claude_home().join("sessions").is_dir();
    let sessions = if registry_available { read_registry(state.paths.claude_home()) } else { sessions_from_processes(connection, table)? };
    Ok(sessions.into_iter().filter(|session| !started_by_app(session)).collect())
}

/// Older Claude Code versions have no registry: match each `claude` process to the most recently
/// updated session recorded for its working directory.
fn sessions_from_processes(connection: &Connection, table: &ProcessTable) -> Result<Vec<ExternalSession>, AppError> {
    let mut sessions = Vec::new();
    for pid in table.claude_pids() {
        let Some(cwd) = process_cwd(pid) else { continue };
        let latest: Option<(String, Option<i64>)> = connection
            .query_row(
                "SELECT id, updated_at FROM sessions WHERE cwd = ?1 ORDER BY updated_at DESC LIMIT 1",
                params![cwd],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((session_id, updated_at)) = latest else { continue };
        let updated_at = updated_at.unwrap_or(0);
        let status = if now_ms() - updated_at < RECENT_TRANSCRIPT_MS { RegistryStatus::Busy } else { RegistryStatus::Idle };
        sessions.push(ExternalSession { pid, session_id, cwd, status, status_updated_at: updated_at });
    }
    Ok(sessions)
}
