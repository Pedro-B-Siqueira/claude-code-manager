use serde::Serialize;

use crate::transcript::usage::TokenUsage;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionListItem {
    pub id: String,
    pub title: String,
    pub custom_name: Option<String>,
    pub first_prompt: Option<String>,
    pub project: String,
    pub cwd: Option<String>,
    pub branch: Option<String>,
    pub started_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub pinned: bool,
    pub pin_order: Option<i64>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub cost_usd: f64,
    pub files_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTouched {
    pub path: String,
    pub added: u32,
    pub removed: u32,
    pub edits: u32,
    pub is_new_file: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub item: SessionListItem,
    pub ai_title: Option<String>,
    pub last_prompt: Option<String>,
    pub last_assistant: Option<String>,
    pub cwd_exists: bool,
    pub repo_root: Option<String>,
    pub model: Option<String>,
    pub pr_url: Option<String>,
    pub duration_ms: Option<i64>,
    pub usage: TokenUsage,
    pub unknown_pricing: bool,
    pub context_tokens: Option<u64>,
    pub context_window: u64,
    pub context_percent: Option<f64>,
    pub files: Vec<FileTouched>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MatchSource {
    Title,
    Project,
    Branch,
    File,
    Tag,
    Text,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub session_id: String,
    pub matched_in: MatchSource,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexProgress {
    pub running: bool,
    pub files_done: u32,
    pub files_total: u32,
}
