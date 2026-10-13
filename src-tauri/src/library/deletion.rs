//! The one exception to "never touch `~/.claude`": when the user deletes a session in the resume
//! dialog, its conversation (`projects/<project>/<id>.jsonl` and the `<id>/` folder beside it) goes to
//! the system Trash. Nothing else under `~/.claude` is ever written, moved or removed.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};

use crate::db::Database;
use crate::error::AppError;

/// Every table that keeps something about a session, derived or set by the user.
const SESSION_TABLES: [&str; 7] = ["file_edits", "activity", "session_fts", "transcript_files", "session_meta", "session_tags", "grid_order"];

pub trait Discard {
    fn discard(&self, path: &Path) -> io::Result<()>;
}

/// The system Trash, so a conversation deleted by mistake can be put back.
pub struct SystemTrash;

impl Discard for SystemTrash {
    fn discard(&self, path: &Path) -> io::Result<()> {
        system_trash().delete(path).map_err(io::Error::other)
    }
}

/// On macOS the crate defaults to asking Finder through AppleScript, which pops up an automation
/// permission prompt; `NSFileManager` moves to the Trash without one.
#[cfg(target_os = "macos")]
fn system_trash() -> trash::TrashContext {
    use trash::macos::{DeleteMethod, TrashContextExtMacos};
    let mut context = trash::TrashContext::default();
    context.set_delete_method(DeleteMethod::NsFileManager);
    context
}

#[cfg(not(target_os = "macos"))]
fn system_trash() -> trash::TrashContext {
    trash::TrashContext::default()
}

pub fn delete_conversation(database: &Database, claude_home: &Path, session_id: &str, session_open: bool, discard: &dyn Discard) -> Result<(), AppError> {
    if session_open {
        return Err(AppError::Invalid("feche a sessão antes de apagar a conversa".to_owned()));
    }
    let transcript: String = database
        .connection()
        .query_row("SELECT transcript_path FROM sessions WHERE id = ?1", params![session_id], |row| row.get(0))
        .optional()?
        .ok_or_else(|| AppError::UnknownSession(session_id.to_owned()))?;
    // The folder first: if it cannot be moved, the conversation is still whole.
    for path in conversation_files(claude_home, Path::new(&transcript), session_id)?.iter().rev() {
        discard.discard(path)?;
    }
    forget_session(database, session_id)
}

/// The transcript and, when present, its sibling folder — only after checking that they are exactly
/// `<claude_home>/projects/<project>/<id>.jsonl` and `<id>/`, with no symlinks on the way.
pub fn conversation_files(claude_home: &Path, transcript: &Path, session_id: &str) -> Result<Vec<PathBuf>, AppError> {
    let refused = || AppError::Invalid("a conversa não está onde o Claude Code guarda as sessões; nada foi apagado".to_owned());
    let canonical_id = uuid::Uuid::parse_str(session_id).map_err(|_| refused())?.hyphenated().to_string();
    if canonical_id != session_id || transcript.file_name().and_then(|name| name.to_str()) != Some(&format!("{session_id}.jsonl")) {
        return Err(refused());
    }
    if !fs::symlink_metadata(transcript)?.file_type().is_file() {
        return Err(refused());
    }
    let project = transcript.parent().ok_or_else(refused)?;
    let projects = claude_home.join("projects").canonicalize()?;
    if project.canonicalize()?.parent() != Some(projects.as_path()) {
        return Err(refused());
    }
    let mut files = vec![transcript.to_path_buf()];
    let folder = project.join(session_id);
    match fs::symlink_metadata(&folder) {
        Ok(metadata) if metadata.file_type().is_dir() => files.push(folder),
        Ok(_) => return Err(refused()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(files)
}

fn forget_session(database: &Database, session_id: &str) -> Result<(), AppError> {
    let mut connection = database.connection();
    let transaction = connection.transaction()?;
    for table in SESSION_TABLES {
        transaction.execute(&format!("DELETE FROM {table} WHERE session_id = ?1"), params![session_id])?;
    }
    transaction.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;
    transaction.execute("DELETE FROM tags WHERE id NOT IN (SELECT tag_id FROM session_tags)", [])?;
    transaction.commit()?;
    Ok(())
}
