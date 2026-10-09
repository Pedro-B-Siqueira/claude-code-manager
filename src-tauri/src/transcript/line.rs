//! One transcript line → zero or more events. Unknown line types and fields are ignored, and a
//! malformed line yields `None` instead of an error.

use chrono::DateTime;
use serde::Deserialize;
use serde_json::value::RawValue;

use super::lenient;
use super::prompt::{self, PromptFlags};
use super::tools::{EditOutcome, ToolInput};
use super::usage::{RawUsage, TokenUsage};

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RawLine<'a> {
    #[serde(rename = "type", deserialize_with = "lenient::string")]
    kind: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    timestamp: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    cwd: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    git_branch: Option<String>,
    #[serde(deserialize_with = "lenient::flag")]
    is_sidechain: bool,
    #[serde(deserialize_with = "lenient::flag")]
    is_meta: bool,
    #[serde(deserialize_with = "lenient::flag")]
    is_compact_summary: bool,
    #[serde(borrow)]
    origin: Option<&'a RawValue>,
    #[serde(borrow)]
    message: Option<&'a RawValue>,
    #[serde(borrow)]
    tool_use_result: Option<&'a RawValue>,
    #[serde(deserialize_with = "lenient::string")]
    ai_title: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    pr_url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawOrigin {
    #[serde(deserialize_with = "lenient::string")]
    kind: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawMessage<'a> {
    #[serde(deserialize_with = "lenient::string")]
    id: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    model: Option<String>,
    #[serde(borrow)]
    content: Option<&'a RawValue>,
    #[serde(borrow)]
    usage: Option<&'a RawValue>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawBlock<'a> {
    #[serde(rename = "type", deserialize_with = "lenient::string")]
    kind: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    text: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    id: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    name: Option<String>,
    #[serde(borrow)]
    input: Option<&'a RawValue>,
    #[serde(deserialize_with = "lenient::string")]
    tool_use_id: Option<String>,
    #[serde(deserialize_with = "lenient::flag")]
    is_error: bool,
}

#[derive(Debug, Clone)]
pub struct ToolUse {
    pub id: String,
    pub name: String,
    pub input: ToolInput,
}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool_use_id: String,
    pub is_error: bool,
    pub outcome: Option<EditOutcome>,
}

#[derive(Debug, Clone)]
pub struct ResponseUsage {
    pub message_id: Option<String>,
    pub model: Option<String>,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone)]
pub enum LineEvent {
    HumanPrompt(String),
    AssistantText(String),
    ToolUse(ToolUse),
    ToolResult(ToolResult),
    Usage(ResponseUsage),
    Title(String),
    PullRequest(String),
}

#[derive(Debug, Clone, Default)]
pub struct ParsedLine {
    pub timestamp_ms: Option<i64>,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub is_sidechain: bool,
    pub events: Vec<LineEvent>,
}

pub fn parse_line(bytes: &[u8]) -> Option<ParsedLine> {
    let raw: RawLine<'_> = serde_json::from_slice(bytes).ok()?;
    let events = match raw.kind.as_deref() {
        Some("assistant") => assistant_events(&raw),
        Some("user") => user_events(&raw),
        Some("ai-title") => raw.ai_title.clone().filter(|title| !title.trim().is_empty()).map(LineEvent::Title).into_iter().collect(),
        Some("pr-link") => raw.pr_url.clone().map(LineEvent::PullRequest).into_iter().collect(),
        _ => Vec::new(),
    };
    Some(ParsedLine {
        timestamp_ms: raw.timestamp.as_deref().and_then(parse_timestamp),
        cwd: raw.cwd.filter(|cwd| !cwd.is_empty()),
        git_branch: raw.git_branch.filter(|branch| !branch.is_empty() && branch != "HEAD"),
        is_sidechain: raw.is_sidechain,
        events,
    })
}

fn parse_timestamp(value: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(value).ok().map(|moment| moment.timestamp_millis())
}

fn parse_message<'a>(raw: &RawLine<'a>) -> Option<RawMessage<'a>> {
    serde_json::from_str(raw.message?.get()).ok()
}

enum MessageContent<'a> {
    Text(String),
    Blocks(Vec<RawBlock<'a>>),
}

fn parse_content<'a>(message: &RawMessage<'a>) -> Option<MessageContent<'a>> {
    let content = message.content?.get();
    match content.trim_start().as_bytes().first() {
        Some(b'"') => serde_json::from_str(content).ok().map(MessageContent::Text),
        Some(b'[') => serde_json::from_str(content).ok().map(MessageContent::Blocks),
        _ => None,
    }
}

fn assistant_events(raw: &RawLine<'_>) -> Vec<LineEvent> {
    let Some(message) = parse_message(raw) else { return Vec::new() };
    let mut events = Vec::new();
    if let Some(MessageContent::Blocks(blocks)) = parse_content(&message) {
        events.extend(blocks.into_iter().filter_map(assistant_block_event));
    }
    if let Some(usage) = message.usage.and_then(|usage| serde_json::from_str::<RawUsage>(usage.get()).ok()) {
        events.push(LineEvent::Usage(ResponseUsage {
            message_id: message.id.clone(),
            model: message.model.clone(),
            usage: TokenUsage::from(usage),
        }));
    }
    events
}

fn assistant_block_event(block: RawBlock<'_>) -> Option<LineEvent> {
    match block.kind.as_deref() {
        Some("text") => block.text.filter(|text| !text.trim().is_empty()).map(LineEvent::AssistantText),
        Some("tool_use") => {
            let input = block
                .input
                .and_then(|input| serde_json::from_str::<ToolInput>(input.get()).ok())
                .unwrap_or_default();
            Some(LineEvent::ToolUse(ToolUse { id: block.id?, name: block.name?, input }))
        }
        _ => None,
    }
}

fn user_events(raw: &RawLine<'_>) -> Vec<LineEvent> {
    let Some(message) = parse_message(raw) else { return Vec::new() };
    match parse_content(&message) {
        Some(MessageContent::Text(text)) => prompt_event(raw, &text).into_iter().collect(),
        Some(MessageContent::Blocks(blocks)) => user_block_events(raw, blocks),
        None => Vec::new(),
    }
}

fn user_block_events(raw: &RawLine<'_>, blocks: Vec<RawBlock<'_>>) -> Vec<LineEvent> {
    let has_tool_result = blocks.iter().any(|block| block.kind.as_deref() == Some("tool_result"));
    if !has_tool_result {
        let text: Vec<String> = blocks
            .into_iter()
            .filter(|block| block.kind.as_deref() == Some("text"))
            .filter_map(|block| block.text)
            .collect();
        return prompt_event(raw, &text.join("\n")).into_iter().collect();
    }
    let mut outcome = edit_outcome(raw);
    blocks
        .into_iter()
        .filter(|block| block.kind.as_deref() == Some("tool_result"))
        .filter_map(|block| {
            Some(LineEvent::ToolResult(ToolResult {
                tool_use_id: block.tool_use_id?,
                is_error: block.is_error,
                outcome: outcome.take(),
            }))
        })
        .collect()
}

fn edit_outcome(raw: &RawLine<'_>) -> Option<EditOutcome> {
    let result = raw.tool_use_result?.get();
    if !result.trim_start().starts_with('{') {
        return None;
    }
    serde_json::from_str(result).ok()
}

fn prompt_event(raw: &RawLine<'_>, text: &str) -> Option<LineEvent> {
    let origin_kind = raw
        .origin
        .and_then(|origin| serde_json::from_str::<RawOrigin>(origin.get()).ok())
        .and_then(|origin| origin.kind);
    let flags = PromptFlags {
        is_meta: raw.is_meta,
        is_compact_summary: raw.is_compact_summary,
        is_sidechain: raw.is_sidechain,
        origin_kind: origin_kind.as_deref(),
    };
    prompt::human_prompt(flags, text).map(LineEvent::HumanPrompt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_json_is_skipped() {
        assert!(parse_line(b"{not json").is_none());
        assert!(parse_line(b"").is_none());
    }

    #[test]
    fn unknown_line_types_only_carry_context() {
        let parsed = parse_line(br#"{"type":"queue-operation","timestamp":"2026-01-02T03:04:05.000Z","newField":[1,2]}"#).unwrap();
        assert!(parsed.events.is_empty());
        assert_eq!(parsed.timestamp_ms, Some(1_767_323_045_000));
    }

    #[test]
    fn assistant_line_yields_text_tool_use_and_usage() {
        let line = br#"{"type":"assistant","cwd":"/repo","gitBranch":"feat-x","timestamp":"2026-01-02T03:04:05Z",
            "message":{"id":"msg_1","model":"claude-sonnet-5","content":[
              {"type":"thinking","thinking":"secret"},
              {"type":"text","text":"Ajustando o handler."},
              {"type":"tool_use","id":"toolu_1","name":"Edit","input":{"file_path":"/repo/a.ts","old_string":"a","new_string":"b\nc"}}
            ],"usage":{"input_tokens":3,"output_tokens":9,"cache_read_input_tokens":100}}}"#;
        let parsed = parse_line(line).unwrap();
        assert_eq!(parsed.git_branch.as_deref(), Some("feat-x"));
        assert!(matches!(&parsed.events[0], LineEvent::AssistantText(text) if text == "Ajustando o handler."));
        assert!(matches!(&parsed.events[1], LineEvent::ToolUse(tool) if tool.name == "Edit" && tool.input.edited_file() == Some("/repo/a.ts")));
        assert!(matches!(&parsed.events[2], LineEvent::Usage(usage) if usage.usage.total() == 112 && usage.message_id.as_deref() == Some("msg_1")));
    }

    #[test]
    fn tool_result_line_carries_edit_outcome() {
        let line = br#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"ok"}]},
            "toolUseResult":{"filePath":"/repo/a.ts","structuredPatch":[{"oldStart":1,"oldLines":1,"newStart":1,"newLines":2,"lines":["-a","+b","+c"]}]}}"#;
        let parsed = parse_line(line).unwrap();
        let LineEvent::ToolResult(result) = &parsed.events[0] else { panic!("expected tool result") };
        assert_eq!(result.tool_use_id, "toolu_1");
        assert_eq!(result.outcome.as_ref().unwrap().delta().added, 2);
    }

    #[test]
    fn error_string_tool_results_have_no_outcome() {
        let line = br#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t","is_error":true,"content":"denied"}]},"toolUseResult":"Error: denied"}"#;
        let LineEvent::ToolResult(result) = &parse_line(line).unwrap().events[0] else { panic!() };
        assert!(result.is_error);
        assert!(result.outcome.is_none());
    }

    #[test]
    fn typed_prompt_in_string_or_text_blocks() {
        let string = br#"{"type":"user","origin":{"kind":"human"},"message":{"content":"Add pagination"}}"#;
        assert!(matches!(&parse_line(string).unwrap().events[0], LineEvent::HumanPrompt(text) if text == "Add pagination"));
        let blocks = br#"{"type":"user","message":{"content":[{"type":"image","source":{}},{"type":"text","text":"What is this?"}]}}"#;
        assert!(matches!(&parse_line(blocks).unwrap().events[0], LineEvent::HumanPrompt(text) if text == "What is this?"));
    }

    #[test]
    fn title_and_pr_link_lines() {
        let title = parse_line(br#"{"type":"ai-title","aiTitle":"Paginate orders","sessionId":"s"}"#).unwrap();
        assert!(matches!(&title.events[0], LineEvent::Title(text) if text == "Paginate orders"));
        let pr = parse_line(br#"{"type":"pr-link","prUrl":"https://example.com/pr/1","prNumber":1}"#).unwrap();
        assert!(matches!(&pr.events[0], LineEvent::PullRequest(url) if url.ends_with("/pr/1")));
    }

    #[test]
    fn detached_head_branch_is_ignored() {
        let parsed = parse_line(br#"{"type":"attachment","gitBranch":"HEAD"}"#).unwrap();
        assert!(parsed.git_branch.is_none());
    }
}
