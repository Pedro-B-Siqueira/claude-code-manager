//! Turns parsed transcript events into rows: session totals, file edits, activity and search text.

use rusqlite::{Transaction, params};

use crate::db::sql_int;
use crate::error::AppError;
use crate::pricing;
use crate::transcript::line::{LineEvent, ParsedLine, ResponseUsage, ToolResult, ToolUse};
use crate::transcript::tools::{EditTool, truncate_chars};
use crate::transcript::usage::TokenUsage;

const WHY_MAX_CHARS: usize = 1_200;
const SEARCH_TEXT_MAX_CHARS: usize = 2_000;
const SUMMARY_TEXT_MAX_CHARS: usize = 2_000;
const ACTIVITY_TARGET_MAX_CHARS: usize = 160;

/// Parse state that must survive between incremental passes over the same file.
#[derive(Debug, Clone, Default)]
pub struct FileCursor {
    pub byte_offset: u64,
    pub last_message_id: Option<String>,
    pub last_assistant_text: Option<String>,
}

#[derive(Debug, Default)]
struct SessionDelta {
    cwd: Option<String>,
    git_branch: Option<String>,
    ai_title: Option<String>,
    first_prompt: Option<String>,
    last_prompt: Option<String>,
    last_assistant: Option<String>,
    model: Option<String>,
    pr_url: Option<String>,
    started_at: Option<i64>,
    updated_at: Option<i64>,
    usage: TokenUsage,
    cost_usd: f64,
    unknown_pricing: bool,
    context_tokens: Option<u64>,
}

pub struct TranscriptSource<'a> {
    pub session_id: &'a str,
    pub project_dir: &'a str,
    pub transcript_path: &'a str,
    pub is_subagent: bool,
}

pub struct SessionWriter<'t, 's> {
    transaction: &'t Transaction<'t>,
    source: &'s TranscriptSource<'s>,
    cursor: FileCursor,
    delta: SessionDelta,
    line_timestamp: Option<i64>,
}

impl<'t, 's> SessionWriter<'t, 's> {
    pub fn new(transaction: &'t Transaction<'t>, source: &'s TranscriptSource<'s>, cursor: FileCursor) -> Self {
        Self { transaction, source, cursor, delta: SessionDelta::default(), line_timestamp: None }
    }

    pub fn write_line(&mut self, offset: u64, line_length: u64, line: ParsedLine) -> Result<(), AppError> {
        let is_main_thread = !self.source.is_subagent && !line.is_sidechain;
        self.track_line_context(&line, is_main_thread);
        for event in line.events {
            self.write_event(event, offset, line_length, is_main_thread)?;
        }
        Ok(())
    }

    fn track_line_context(&mut self, line: &ParsedLine, is_main_thread: bool) {
        self.line_timestamp = line.timestamp_ms;
        if let Some(timestamp) = line.timestamp_ms {
            self.delta.started_at = Some(self.delta.started_at.map_or(timestamp, |start| start.min(timestamp)));
            self.delta.updated_at = Some(self.delta.updated_at.map_or(timestamp, |end| end.max(timestamp)));
        }
        if is_main_thread {
            if self.delta.cwd.is_none() {
                self.delta.cwd.clone_from(&line.cwd);
            }
            if line.git_branch.is_some() {
                self.delta.git_branch.clone_from(&line.git_branch);
            }
        }
    }

    fn write_event(&mut self, event: LineEvent, offset: u64, line_length: u64, is_main_thread: bool) -> Result<(), AppError> {
        match event {
            LineEvent::HumanPrompt(text) if is_main_thread => self.write_prompt(&text),
            LineEvent::AssistantText(text) => self.write_assistant_text(text, is_main_thread),
            LineEvent::ToolUse(tool_use) => self.write_tool_use(&tool_use, offset, line_length),
            LineEvent::ToolResult(result) => self.write_tool_result(&result),
            LineEvent::Usage(usage) => {
                self.add_usage(usage, is_main_thread);
                Ok(())
            }
            LineEvent::Title(title) if is_main_thread => {
                self.delta.ai_title = Some(title);
                Ok(())
            }
            LineEvent::PullRequest(url) if is_main_thread => {
                self.delta.pr_url = Some(url);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn write_prompt(&mut self, text: &str) -> Result<(), AppError> {
        let summary = truncate_chars(text, SUMMARY_TEXT_MAX_CHARS);
        if self.delta.first_prompt.is_none() {
            self.delta.first_prompt = Some(summary.clone());
        }
        self.delta.last_prompt = Some(summary);
        self.insert_activity("prompt", None, Some(&truncate_chars(text, ACTIVITY_TARGET_MAX_CHARS)), None)?;
        self.insert_search_text(text)
    }

    fn write_assistant_text(&mut self, text: String, is_main_thread: bool) -> Result<(), AppError> {
        self.cursor.last_assistant_text = Some(truncate_chars(&text, WHY_MAX_CHARS));
        if !is_main_thread {
            return Ok(());
        }
        self.delta.last_assistant = Some(truncate_chars(&text, SUMMARY_TEXT_MAX_CHARS));
        self.insert_search_text(&text)
    }

    fn write_tool_use(&mut self, tool_use: &ToolUse, offset: u64, line_length: u64) -> Result<(), AppError> {
        let edit = EditTool::from_name(&tool_use.name).zip(tool_use.input.edited_file());
        let Some((tool, file_path)) = edit else {
            let target = tool_use.input.activity_target();
            return self.insert_activity("tool", Some(&tool_use.name), target.as_deref(), None);
        };
        let estimate = tool_use.input.estimated_delta(tool);
        self.transaction.execute(
            "INSERT INTO file_edits (session_id, tool_use_id, timestamp, tool, file_path, added, removed,
                 transcript_path, line_offset, line_length, why, from_subagent)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(tool_use_id) DO NOTHING",
            params![
                self.source.session_id, tool_use.id, self.line_timestamp, tool.as_str(), file_path,
                estimate.added, estimate.removed, self.source.transcript_path, sql_int(offset), sql_int(line_length),
                self.cursor.last_assistant_text, self.source.is_subagent,
            ],
        )?;
        let edit_id = self.transaction.last_insert_rowid();
        self.insert_activity("edit", Some(tool.as_str()), Some(file_path), Some(edit_id))
    }

    fn write_tool_result(&mut self, result: &ToolResult) -> Result<(), AppError> {
        if result.is_error {
            self.transaction.execute(
                "UPDATE file_edits SET status = 'failed' WHERE tool_use_id = ?1",
                params![result.tool_use_id],
            )?;
            return Ok(());
        }
        let Some(outcome) = &result.outcome else {
            self.transaction.execute(
                "UPDATE file_edits SET status = 'applied' WHERE tool_use_id = ?1",
                params![result.tool_use_id],
            )?;
            return Ok(());
        };
        let delta = outcome.delta();
        let range = outcome.new_line_range();
        self.transaction.execute(
            "UPDATE file_edits SET status = 'applied', is_new_file = ?2,
                 added = CASE WHEN ?2 THEN added ELSE ?3 END,
                 removed = CASE WHEN ?2 THEN 0 ELSE ?4 END,
                 new_start = ?5, new_end = ?6
             WHERE tool_use_id = ?1",
            params![result.tool_use_id, outcome.is_new_file(), delta.added, delta.removed, range.map(|(start, _)| sql_int(start)), range.map(|(_, end)| sql_int(end))],
        )?;
        Ok(())
    }

    fn add_usage(&mut self, response: ResponseUsage, is_main_thread: bool) {
        if response.message_id.is_some() && response.message_id == self.cursor.last_message_id {
            return;
        }
        self.cursor.last_message_id = response.message_id;
        self.delta.usage.add(&response.usage);
        let model = response.model.unwrap_or_default();
        match pricing::pricing_for(&model) {
            Some(price) => self.delta.cost_usd += price.cost(&response.usage),
            None if response.usage.total() > 0 => self.delta.unknown_pricing = true,
            None => {}
        }
        if is_main_thread && response.usage.total() > 0 {
            self.delta.context_tokens = Some(response.usage.context_tokens());
            self.delta.model = Some(model);
        }
    }

    fn insert_activity(&self, kind: &str, tool: Option<&str>, target: Option<&str>, edit_id: Option<i64>) -> Result<(), AppError> {
        self.transaction.execute(
            "INSERT INTO activity (session_id, timestamp, kind, tool, target, edit_id, from_subagent)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![self.source.session_id, self.line_timestamp, kind, tool, target, edit_id, self.source.is_subagent],
        )?;
        Ok(())
    }

    fn insert_search_text(&self, text: &str) -> Result<(), AppError> {
        self.transaction.execute(
            "INSERT INTO session_fts (session_id, body) VALUES (?1, ?2)",
            params![self.source.session_id, truncate_chars(text, SEARCH_TEXT_MAX_CHARS)],
        )?;
        Ok(())
    }

    /// Persists the accumulated totals and returns the cursor to store for the next pass.
    pub fn finish(self, byte_offset: u64) -> Result<FileCursor, AppError> {
        upsert_session(self.transaction, self.source, &self.delta)?;
        if let Some(cwd) = &self.delta.cwd {
            super::project::ensure_project(self.transaction, cwd)?;
        }
        Ok(FileCursor { byte_offset, ..self.cursor })
    }
}

fn upsert_session(transaction: &Transaction<'_>, source: &TranscriptSource<'_>, delta: &SessionDelta) -> Result<(), AppError> {
    let transcript_path = if source.is_subagent { "" } else { source.transcript_path };
    transaction.execute(
        "INSERT INTO sessions (id, project_dir, transcript_path, cwd, git_branch, ai_title, first_prompt, last_prompt,
             last_assistant, model, pr_url, started_at, updated_at, input_tokens, output_tokens, cache_read_tokens,
             cache_write_5m_tokens, cache_write_1h_tokens, cost_usd, unknown_pricing, context_tokens, context_model)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?10)
         ON CONFLICT(id) DO UPDATE SET
             transcript_path = CASE WHEN excluded.transcript_path <> '' THEN excluded.transcript_path ELSE sessions.transcript_path END,
             cwd = COALESCE(sessions.cwd, excluded.cwd),
             git_branch = COALESCE(excluded.git_branch, sessions.git_branch),
             ai_title = COALESCE(excluded.ai_title, sessions.ai_title),
             first_prompt = COALESCE(sessions.first_prompt, excluded.first_prompt),
             last_prompt = COALESCE(excluded.last_prompt, sessions.last_prompt),
             last_assistant = COALESCE(excluded.last_assistant, sessions.last_assistant),
             model = COALESCE(excluded.model, sessions.model),
             pr_url = COALESCE(excluded.pr_url, sessions.pr_url),
             started_at = MIN(COALESCE(sessions.started_at, excluded.started_at), COALESCE(excluded.started_at, sessions.started_at)),
             updated_at = MAX(COALESCE(sessions.updated_at, excluded.updated_at), COALESCE(excluded.updated_at, sessions.updated_at)),
             input_tokens = sessions.input_tokens + excluded.input_tokens,
             output_tokens = sessions.output_tokens + excluded.output_tokens,
             cache_read_tokens = sessions.cache_read_tokens + excluded.cache_read_tokens,
             cache_write_5m_tokens = sessions.cache_write_5m_tokens + excluded.cache_write_5m_tokens,
             cache_write_1h_tokens = sessions.cache_write_1h_tokens + excluded.cache_write_1h_tokens,
             cost_usd = sessions.cost_usd + excluded.cost_usd,
             unknown_pricing = MAX(sessions.unknown_pricing, excluded.unknown_pricing),
             context_tokens = COALESCE(excluded.context_tokens, sessions.context_tokens),
             context_model = COALESCE(excluded.context_model, sessions.context_model)",
        params![
            source.session_id, source.project_dir, transcript_path, delta.cwd, delta.git_branch, delta.ai_title,
            delta.first_prompt, delta.last_prompt, delta.last_assistant, delta.model, delta.pr_url, delta.started_at,
            delta.updated_at, sql_int(delta.usage.input), sql_int(delta.usage.output), sql_int(delta.usage.cache_read),
            sql_int(delta.usage.cache_write_5m), sql_int(delta.usage.cache_write_1h), delta.cost_usd,
            delta.unknown_pricing, delta.context_tokens.map(sql_int),
        ],
    )?;
    Ok(())
}
