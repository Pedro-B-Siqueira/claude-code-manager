//! Claude Code sessions running anywhere on the machine, read from `~/.claude/sessions/<pid>.json`
//! (written by Claude Code itself). Read-only; entries whose process is gone are skipped.

use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::transcript::lenient;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RegistryEntry {
    #[serde(deserialize_with = "lenient::count")]
    pid: u64,
    #[serde(deserialize_with = "lenient::string")]
    session_id: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    cwd: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    status: Option<String>,
    #[serde(deserialize_with = "lenient::count")]
    status_updated_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryStatus {
    Busy,
    Idle,
    NeedsPermission,
    NeedsInput,
    Unknown,
}

impl RegistryStatus {
    fn parse(value: Option<&str>) -> Self {
        let value = value.unwrap_or_default().to_lowercase();
        match value.as_str() {
            "busy" | "working" | "running" => Self::Busy,
            "idle" => Self::Idle,
            _ if value.contains("perm") => Self::NeedsPermission,
            _ if value.contains("wait") || value.contains("input") => Self::NeedsInput,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalSession {
    pub pid: u32,
    pub session_id: String,
    pub cwd: String,
    pub status: RegistryStatus,
    pub status_updated_at: i64,
}

pub fn read_registry(claude_home: &Path) -> Vec<ExternalSession> {
    let Ok(entries) = fs::read_dir(claude_home.join("sessions")) else { return Vec::new() };
    let mut sessions: Vec<ExternalSession> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "json"))
        .filter_map(|path| fs::read(path).ok())
        .filter_map(|bytes| serde_json::from_slice::<RegistryEntry>(&bytes).ok())
        .filter_map(into_session)
        .filter(|session| process_alive(session.pid))
        .collect();
    sessions.sort_by_key(|session| session.pid);
    sessions
}

fn into_session(entry: RegistryEntry) -> Option<ExternalSession> {
    Some(ExternalSession {
        pid: u32::try_from(entry.pid).ok().filter(|pid| *pid > 1)?,
        session_id: entry.session_id.filter(|id| !id.is_empty())?,
        cwd: entry.cwd.unwrap_or_default(),
        status: RegistryStatus::parse(entry.status.as_deref()),
        status_updated_at: i64::try_from(entry.status_updated_at).unwrap_or(0),
    })
}

/// `kill(pid, 0)` checks existence without signalling; EPERM still means the process exists.
pub fn process_alive(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else { return false };
    let result = unsafe { libc::kill(pid, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_entry(directory: &Path, name: &str, json: &str) {
        fs::write(directory.join(name), json).unwrap();
    }

    #[test]
    fn reads_live_entries_and_skips_dead_or_broken_ones() {
        let home = tempfile::tempdir().unwrap();
        let sessions = home.path().join("sessions");
        fs::create_dir_all(&sessions).unwrap();
        let own_pid = std::process::id();
        write_entry(&sessions, "a.json", &format!(r#"{{"pid":{own_pid},"sessionId":"live","cwd":"/repo","status":"busy","statusUpdatedAt":5,"extra":1}}"#));
        write_entry(&sessions, "b.json", r#"{"pid":999999,"sessionId":"dead","cwd":"/x","status":"idle"}"#);
        write_entry(&sessions, "c.json", "{broken");
        write_entry(&sessions, "d.key", "not json");
        let found = read_registry(home.path());
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].session_id.as_str(), found[0].status), ("live", RegistryStatus::Busy));
    }

    #[test]
    fn unknown_status_strings_are_classified_conservatively() {
        assert_eq!(RegistryStatus::parse(Some("idle")), RegistryStatus::Idle);
        assert_eq!(RegistryStatus::parse(Some("awaiting_permission")), RegistryStatus::NeedsPermission);
        assert_eq!(RegistryStatus::parse(Some("waiting_for_input")), RegistryStatus::NeedsInput);
        assert_eq!(RegistryStatus::parse(Some("compacting")), RegistryStatus::Unknown);
        assert_eq!(RegistryStatus::parse(None), RegistryStatus::Unknown);
    }

    #[test]
    fn missing_registry_directory_means_no_sessions() {
        assert!(read_registry(Path::new("/nonexistent/claude-home")).is_empty());
    }
}
