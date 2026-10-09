//! Keeps the local library in sync with `~/.claude/projects`, reading only what was appended.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

use super::writer::{FileCursor, SessionWriter, TranscriptSource};
use crate::db::{Database, from_sql_int, sql_int};
use crate::error::AppError;
use crate::transcript::line::parse_line;
use crate::transcript::reader::{self, FileIdentity};

/// Lines per transaction: keeps the write lock short so the UI connection is never blocked for long.
const LINES_PER_COMMIT: usize = 4_000;

/// Stored instead of the real mtime while a file is only partly indexed.
const PARTIAL_MODIFIED_MS: i64 = -1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptLocation {
    pub session_id: String,
    pub project_dir: String,
    pub is_subagent: bool,
}

/// `projects/<dir>/<session>.jsonl` is a session. Agent transcripts under `<session>/subagents/`
/// (including nested workflow folders) belong to it; other files there, like workflow journals, do not.
pub fn locate(projects_root: &Path, path: &Path) -> Option<TranscriptLocation> {
    if path.extension()? != "jsonl" {
        return None;
    }
    let relative = path.strip_prefix(projects_root).ok()?;
    let parts: Vec<&str> = relative.iter().filter_map(|part| part.to_str()).collect();
    match parts.as_slice() {
        [project_dir, file] => Some(TranscriptLocation {
            session_id: file.strip_suffix(".jsonl")?.to_owned(),
            project_dir: (*project_dir).to_owned(),
            is_subagent: false,
        }),
        [project_dir, session_id, "subagents", .., file] if is_agent_transcript(file) => Some(TranscriptLocation {
            session_id: (*session_id).to_owned(),
            project_dir: (*project_dir).to_owned(),
            is_subagent: true,
        }),
        _ => None,
    }
}

fn is_agent_transcript(file_name: &str) -> bool {
    file_name.starts_with("agent-") && file_name.ends_with(".jsonl")
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndexOutcome {
    pub files_changed: usize,
}

pub struct Indexer {
    database: Database,
    projects_root: PathBuf,
}

struct TrackedFile {
    identity: FileIdentity,
    cursor: FileCursor,
}

impl Indexer {
    pub fn new(database: Database, projects_root: PathBuf) -> Self {
        Self { database, projects_root }
    }

    pub fn projects_root(&self) -> &Path {
        &self.projects_root
    }

    /// All transcripts, most recently modified first so recent sessions show up early.
    pub fn discover(&self) -> Vec<PathBuf> {
        let mut files: Vec<(i64, PathBuf)> = walk_transcripts(&self.projects_root)
            .into_iter()
            .filter_map(|path| reader::identity(&path).ok().map(|identity| (identity.modified_ms, path)))
            .collect();
        files.sort_by_key(|(modified_ms, _)| std::cmp::Reverse(*modified_ms));
        files.into_iter().map(|(_, path)| path).collect()
    }

    pub fn index_paths(&self, paths: &[PathBuf]) -> Result<IndexOutcome, AppError> {
        let mut outcome = IndexOutcome::default();
        for path in paths {
            match self.index_file(path) {
                Ok(true) => outcome.files_changed += 1,
                Ok(false) => {}
                Err(error) => log::warn!("falha ao indexar {}: {error}", path.display()),
            }
        }
        Ok(outcome)
    }

    pub fn index_file(&self, path: &Path) -> Result<bool, AppError> {
        let Some(location) = locate(&self.projects_root, path) else { return Ok(false) };
        let Ok(identity) = reader::identity(path) else {
            return self.forget_missing_file(path, &location);
        };
        let tracked = self.load_tracked(path)?;
        if let Some(tracked) = &tracked {
            if tracked.identity == identity {
                return Ok(false);
            }
            if tracked.identity.inode != identity.inode || identity.size < tracked.cursor.byte_offset {
                self.reset_session(&location.session_id)?;
                return self.index_session_files(&location).map(|()| true);
            }
        }
        let cursor = tracked.map(|tracked| tracked.cursor).unwrap_or_default();
        self.append_from(path, &location, identity, cursor)?;
        Ok(true)
    }

    fn append_from(&self, path: &Path, location: &TranscriptLocation, identity: FileIdentity, mut cursor: FileCursor) -> Result<(), AppError> {
        let path_text = path.to_string_lossy().into_owned();
        let source = TranscriptSource {
            session_id: &location.session_id,
            project_dir: &location.project_dir,
            transcript_path: &path_text,
            is_subagent: location.is_subagent,
        };
        loop {
            let (next_cursor, reached_end) = self.append_chunk(path, &source, location, identity, cursor)?;
            cursor = next_cursor;
            if reached_end {
                break;
            }
        }
        let connection = self.database.connection();
        save_tracked(&connection, &path_text, location, identity, &cursor)
    }

    /// Each chunk commits its rows together with the cursor, so a crash never double-counts lines.
    /// Until the whole file is read, the stored identity is marked partial to force a resume.
    fn append_chunk(
        &self,
        path: &Path,
        source: &TranscriptSource<'_>,
        location: &TranscriptLocation,
        identity: FileIdentity,
        cursor: FileCursor,
    ) -> Result<(FileCursor, bool), AppError> {
        let mut connection = self.database.connection();
        let transaction = connection.transaction()?;
        let start = cursor.byte_offset;
        let mut writer = SessionWriter::new(&transaction, source, cursor);
        let mut write_error = None;
        let read = reader::read_complete_lines(path, start, LINES_PER_COMMIT, |offset, bytes| {
            if write_error.is_some() {
                return;
            }
            if let Some(line) = parse_line(bytes) {
                if let Err(error) = writer.write_line(offset, bytes.len() as u64 + 1, line) {
                    write_error = Some(error);
                }
            }
        })?;
        if let Some(error) = write_error {
            return Err(error);
        }
        let next_cursor = writer.finish(read.end_offset)?;
        let partial_identity = FileIdentity { modified_ms: PARTIAL_MODIFIED_MS, ..identity };
        save_tracked(&transaction, source.transcript_path, location, partial_identity, &next_cursor)?;
        transaction.commit()?;
        Ok((next_cursor, read.reached_end))
    }

    fn load_tracked(&self, path: &Path) -> Result<Option<TrackedFile>, AppError> {
        let connection = self.database.connection();
        let tracked = connection
            .query_row(
                "SELECT inode, size, modified_ms, byte_offset, last_message_id, last_assistant_text
                 FROM transcript_files WHERE path = ?1",
                params![path.to_string_lossy()],
                |row| {
                    Ok(TrackedFile {
                        identity: FileIdentity {
                            inode: from_sql_int(row.get(0)?),
                            size: from_sql_int(row.get(1)?),
                            modified_ms: row.get(2)?,
                        },
                        cursor: FileCursor {
                            byte_offset: from_sql_int(row.get(3)?),
                            last_message_id: row.get(4)?,
                            last_assistant_text: row.get(5)?,
                        },
                    })
                },
            )
            .optional()?;
        Ok(tracked)
    }

    fn index_session_files(&self, location: &TranscriptLocation) -> Result<(), AppError> {
        let project_root = self.projects_root.join(&location.project_dir);
        let mut files = vec![project_root.join(format!("{}.jsonl", location.session_id))];
        let subagents = project_root.join(&location.session_id).join("subagents");
        files.extend(agent_transcripts_under(&subagents));
        for file in files.iter().filter(|file| file.is_file()) {
            self.index_file(file)?;
        }
        Ok(())
    }

    fn forget_missing_file(&self, path: &Path, location: &TranscriptLocation) -> Result<bool, AppError> {
        let was_tracked = self.load_tracked(path)?.is_some();
        if was_tracked && !location.is_subagent {
            self.reset_session(&location.session_id)?;
        }
        Ok(was_tracked)
    }

    /// Drops everything derived from a session's transcripts; user data (name, pin, tags) is kept.
    pub fn reset_session(&self, session_id: &str) -> Result<(), AppError> {
        let mut connection = self.database.connection();
        let transaction = connection.transaction()?;
        for table in ["file_edits", "activity", "session_fts", "transcript_files"] {
            transaction.execute(&format!("DELETE FROM {table} WHERE session_id = ?1"), params![session_id])?;
        }
        transaction.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;
        transaction.commit()?;
        Ok(())
    }

    /// Sessions whose main transcript was deleted (e.g. `claude purge`) leave the library.
    pub fn prune_missing(&self) -> Result<usize, AppError> {
        let tracked_main_files: Vec<(String, String)> = {
            let connection = self.database.connection();
            let mut statement =
                connection.prepare("SELECT path, session_id FROM transcript_files WHERE is_subagent = 0")?;
            let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
            rows.collect::<Result<_, _>>()?
        };
        let missing: Vec<&String> = tracked_main_files
            .iter()
            .filter(|(path, _)| !Path::new(path).exists())
            .map(|(_, session_id)| session_id)
            .collect();
        for session_id in &missing {
            self.reset_session(session_id)?;
        }
        Ok(missing.len())
    }
}

fn save_tracked(connection: &Connection, path: &str, location: &TranscriptLocation, identity: FileIdentity, cursor: &FileCursor) -> Result<(), AppError> {
    connection.execute(
        "INSERT INTO transcript_files (path, session_id, is_subagent, inode, size, byte_offset, modified_ms,
             last_message_id, last_assistant_text)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(path) DO UPDATE SET inode = excluded.inode, size = excluded.size,
             byte_offset = excluded.byte_offset, modified_ms = excluded.modified_ms,
             last_message_id = excluded.last_message_id, last_assistant_text = excluded.last_assistant_text",
        params![
            path,
            location.session_id,
            location.is_subagent,
            sql_int(identity.inode),
            sql_int(identity.size),
            sql_int(cursor.byte_offset),
            identity.modified_ms,
            cursor.last_message_id,
            cursor.last_assistant_text,
        ],
    )?;
    Ok(())
}

fn walk_transcripts(projects_root: &Path) -> Vec<PathBuf> {
    let Ok(project_dirs) = fs::read_dir(projects_root) else { return Vec::new() };
    let mut files = Vec::new();
    for project_dir in project_dirs.flatten().map(|entry| entry.path()).filter(|path| path.is_dir()) {
        files.extend(jsonl_files_in(&project_dir));
        let Ok(children) = fs::read_dir(&project_dir) else { continue };
        for session_dir in children.flatten().map(|entry| entry.path()).filter(|path| path.is_dir()) {
            files.extend(agent_transcripts_under(&session_dir.join("subagents")));
        }
    }
    files
}

/// Agent transcripts at any depth below `subagents/` (workflows nest them one level per run).
fn agent_transcripts_under(directory: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(directory) else { return Vec::new() };
    let mut files = Vec::new();
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_dir() {
            files.extend(agent_transcripts_under(&path));
        } else if path.file_name().and_then(|name| name.to_str()).is_some_and(is_agent_transcript) {
            files.push(path);
        }
    }
    files
}

fn jsonl_files_in(directory: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(directory) else { return Vec::new() };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|extension| extension == "jsonl"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn location(path: &str) -> Option<TranscriptLocation> {
        locate(Path::new("/p"), Path::new(path))
    }

    #[test]
    fn locates_main_and_agent_transcripts() {
        let main = location("/p/-repo/abc.jsonl").unwrap();
        assert_eq!((main.session_id.as_str(), main.is_subagent), ("abc", false));
        let agent = location("/p/-repo/abc/subagents/agent-1.jsonl").unwrap();
        assert_eq!((agent.session_id.as_str(), agent.is_subagent), ("abc", true));
        let workflow = location("/p/-repo/abc/subagents/workflows/wf_1/agent-2.jsonl").unwrap();
        assert_eq!((workflow.session_id.as_str(), workflow.is_subagent), ("abc", true));
    }

    #[test]
    fn ignores_journals_metadata_and_unknown_layouts() {
        assert!(location("/p/-repo/abc/subagents/workflows/wf_1/journal.jsonl").is_none());
        assert!(location("/p/-repo/abc/subagents/agent-1.meta.json").is_none());
        assert!(location("/p/-repo/abc/tool-results/x.jsonl").is_none());
        assert!(location("/elsewhere/-repo/abc.jsonl").is_none());
    }
}
