//! On-demand details: the diff behind each file edit (re-read from the transcript line where the
//! tool call was recorded) and the per-session activity feed.

use std::path::Path;

use rusqlite::{Connection, Row, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::diff::{DiffLine, added_lines, line_diff};
use crate::db::from_sql_int;
use crate::error::AppError;
use crate::transcript::reader::read_line_at;

const MAX_EDITS_PER_FILE: usize = 10;
const MAX_DIFF_LINES: usize = 220;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditDetail {
    pub id: i64,
    pub tool: String,
    pub file_path: String,
    pub timestamp: Option<i64>,
    pub new_start: Option<u64>,
    pub new_end: Option<u64>,
    pub added: u32,
    pub removed: u32,
    pub is_new_file: bool,
    pub why: Option<String>,
    pub lines: Vec<DiffLine>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityItem {
    pub id: i64,
    pub timestamp: Option<i64>,
    pub kind: String,
    pub tool: Option<String>,
    pub target: Option<String>,
    pub edit_id: Option<i64>,
    pub from_subagent: bool,
}

struct EditRow {
    id: i64,
    tool_use_id: String,
    tool: String,
    file_path: String,
    timestamp: Option<i64>,
    new_start: Option<u64>,
    new_end: Option<u64>,
    added: u32,
    removed: u32,
    is_new_file: bool,
    why: Option<String>,
    transcript_path: String,
    line_offset: u64,
    line_length: u64,
}

const EDIT_COLUMNS: &str = "id, tool_use_id, tool, file_path, timestamp, new_start, new_end, added, removed, is_new_file, why, transcript_path, line_offset, line_length";

fn edit_row(row: &Row<'_>) -> rusqlite::Result<EditRow> {
    Ok(EditRow {
        id: row.get(0)?,
        tool_use_id: row.get(1)?,
        tool: row.get(2)?,
        file_path: row.get(3)?,
        timestamp: row.get(4)?,
        new_start: row.get::<_, Option<i64>>(5)?.map(from_sql_int),
        new_end: row.get::<_, Option<i64>>(6)?.map(from_sql_int),
        added: u32::try_from(row.get::<_, i64>(7)?).unwrap_or(0),
        removed: u32::try_from(row.get::<_, i64>(8)?).unwrap_or(0),
        is_new_file: row.get::<_, i64>(9)? != 0,
        why: row.get(10)?,
        transcript_path: row.get(11)?,
        line_offset: from_sql_int(row.get(12)?),
        line_length: from_sql_int(row.get(13)?),
    })
}

/// Most recent edits of one file in a session, newest first, each with its diff.
pub fn file_edits(connection: &Connection, session_id: &str, file_path: &str) -> Result<Vec<EditDetail>, AppError> {
    let mut statement = connection.prepare(&format!(
        "SELECT {EDIT_COLUMNS} FROM file_edits
         WHERE session_id = ?1 AND file_path = ?2 AND status <> 'failed' ORDER BY id DESC LIMIT {MAX_EDITS_PER_FILE}"
    ))?;
    let rows = statement.query_map(params![session_id, file_path], edit_row)?;
    rows.map(|row| Ok(edit_detail(row?))).collect()
}

pub fn edit_by_id(connection: &Connection, edit_id: i64) -> Result<Option<EditDetail>, AppError> {
    let mut statement = connection.prepare(&format!("SELECT {EDIT_COLUMNS} FROM file_edits WHERE id = ?1"))?;
    let mut rows = statement.query_map(params![edit_id], edit_row)?;
    Ok(rows.next().transpose()?.map(edit_detail))
}

fn edit_detail(row: EditRow) -> EditDetail {
    let (mut lines, is_new_file) = diff_from_transcript(&row).unwrap_or_else(|| (Vec::new(), row.is_new_file));
    let truncated = lines.len() > MAX_DIFF_LINES;
    lines.truncate(MAX_DIFF_LINES);
    EditDetail {
        id: row.id,
        tool: row.tool,
        file_path: row.file_path,
        timestamp: row.timestamp,
        new_start: row.new_start,
        new_end: row.new_end,
        added: row.added,
        removed: row.removed,
        is_new_file: is_new_file || row.is_new_file,
        why: row.why,
        lines,
        truncated,
    }
}

#[derive(Deserialize)]
struct EditInput {
    #[serde(default)]
    old_string: Option<String>,
    #[serde(default)]
    new_string: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    edits: Option<Vec<EditInput>>,
}

/// Returns the diff lines and whether the tool call created the file (a Write is shown as new).
fn diff_from_transcript(row: &EditRow) -> Option<(Vec<DiffLine>, bool)> {
    let bytes = read_line_at(Path::new(&row.transcript_path), row.line_offset, row.line_length.saturating_sub(1)).ok()?;
    let line: Value = serde_json::from_slice(&bytes).ok()?;
    let input = line
        .pointer("/message/content")?
        .as_array()?
        .iter()
        .find(|block| block.get("id").and_then(Value::as_str) == Some(row.tool_use_id.as_str()))?
        .get("input")?
        .clone();
    let input: EditInput = serde_json::from_value(input).ok()?;
    Some(diff_for_input(&input))
}

fn diff_for_input(input: &EditInput) -> (Vec<DiffLine>, bool) {
    if let Some(content) = &input.content {
        return (added_lines(content), true);
    }
    if let Some(edits) = &input.edits {
        return (edits.iter().flat_map(|edit| diff_for_input(edit).0).collect(), false);
    }
    let old = input.old_string.as_deref().unwrap_or_default();
    let new = input.new_string.as_deref().unwrap_or_default();
    (line_diff(old, new), false)
}

pub fn activity(connection: &Connection, session_id: &str, limit: u32) -> Result<Vec<ActivityItem>, AppError> {
    let mut statement = connection.prepare(
        "SELECT id, timestamp, kind, tool, target, edit_id, from_subagent FROM activity
         WHERE session_id = ?1 ORDER BY id DESC LIMIT ?2",
    )?;
    let rows = statement.query_map(params![session_id, limit], |row| {
        Ok(ActivityItem {
            id: row.get(0)?,
            timestamp: row.get(1)?,
            kind: row.get(2)?,
            tool: row.get(3)?,
            target: row.get(4)?,
            edit_id: row.get(5)?,
            from_subagent: row.get::<_, i64>(6)? != 0,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::diff::DiffLineKind;

    #[test]
    fn multi_edit_input_concatenates_each_edit_diff() {
        let input: EditInput = serde_json::from_str(
            r#"{"file_path":"/a.ts","edits":[{"old_string":"a","new_string":"b"},{"old_string":"x","new_string":"y\nz"}]}"#,
        )
        .unwrap();
        let (lines, is_new_file) = diff_for_input(&input);
        assert!(!is_new_file);
        let added: Vec<&str> = lines.iter().filter(|line| line.kind == DiffLineKind::Added).map(|line| line.text.as_str()).collect();
        assert_eq!(added, vec!["b", "y", "z"]);
    }

    #[test]
    fn a_vanished_transcript_degrades_to_an_empty_diff() {
        let row = EditRow {
            id: 1,
            tool_use_id: "t".to_owned(),
            tool: "Edit".to_owned(),
            file_path: "/a.ts".to_owned(),
            timestamp: None,
            new_start: None,
            new_end: None,
            added: 1,
            removed: 0,
            is_new_file: false,
            why: None,
            transcript_path: "/nonexistent/session.jsonl".to_owned(),
            line_offset: 0,
            line_length: 10,
        };
        let detail = edit_detail(row);
        assert!(detail.lines.is_empty());
        assert!(!detail.truncated);
    }
}
