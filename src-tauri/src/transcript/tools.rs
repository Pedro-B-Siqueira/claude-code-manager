//! The parts of tool inputs and results the app needs, read without keeping large payloads in memory.

use std::fmt;

use serde::de::{self, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

use super::lenient;

const TARGET_MAX_CHARS: usize = 160;

/// Number of lines in a string field, counted while deserializing so the text itself is dropped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LineCount(pub u32);

impl<'de> Deserialize<'de> for LineCount {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(LineCountVisitor)
    }
}

struct LineCountVisitor;

impl<'de> Visitor<'de> for LineCountVisitor {
    type Value = LineCount;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(LineCount(u32::try_from(value.lines().count()).unwrap_or(u32::MAX)))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(LineCount(0))
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(LineCount(0))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        while sequence.next_element::<IgnoredAny>()?.is_some() {}
        Ok(LineCount(0))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(LineCount(0))
    }
}

/// Added/removed lines in a unified-diff hunk, counted from the `lines` array without storing it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LineDelta {
    pub added: u32,
    pub removed: u32,
}

impl<'de> Deserialize<'de> for LineDelta {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(LineDeltaVisitor)
    }
}

struct LineDeltaVisitor;

impl<'de> Visitor<'de> for LineDeltaVisitor {
    type Value = LineDelta;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an array of diff lines")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut delta = LineDelta::default();
        while let Some(line) = sequence.next_element::<DiffLinePrefix>()? {
            match line {
                DiffLinePrefix::Added => delta.added += 1,
                DiffLinePrefix::Removed => delta.removed += 1,
                DiffLinePrefix::Other => {}
            }
        }
        Ok(delta)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(LineDelta::default())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(LineDelta::default())
    }
}

enum DiffLinePrefix {
    Added,
    Removed,
    Other,
}

impl<'de> Deserialize<'de> for DiffLinePrefix {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let prefix = lenient::string(deserializer)?;
        Ok(match prefix.as_deref().and_then(|line| line.chars().next()) {
            Some('+') => Self::Added,
            Some('-') => Self::Removed,
            _ => Self::Other,
        })
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PatchHunk {
    #[serde(deserialize_with = "lenient::count")]
    pub old_start: u64,
    #[serde(deserialize_with = "lenient::count")]
    pub old_lines: u64,
    #[serde(deserialize_with = "lenient::count")]
    pub new_start: u64,
    #[serde(deserialize_with = "lenient::count")]
    pub new_lines: u64,
    pub lines: LineDelta,
}

/// `toolUseResult` of an Edit/Write call. `kind` is `create` for brand-new files written by Write.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EditOutcome {
    #[serde(deserialize_with = "lenient::string")]
    pub file_path: Option<String>,
    #[serde(rename = "type", deserialize_with = "lenient::string")]
    pub kind: Option<String>,
    pub structured_patch: Vec<PatchHunk>,
}

impl EditOutcome {
    pub fn is_new_file(&self) -> bool {
        self.kind.as_deref() == Some("create")
    }

    pub fn delta(&self) -> LineDelta {
        self.structured_patch.iter().fold(LineDelta::default(), |total, hunk| LineDelta {
            added: total.added + hunk.lines.added,
            removed: total.removed + hunk.lines.removed,
        })
    }

    /// First and last line touched in the new file, across all hunks.
    pub fn new_line_range(&self) -> Option<(u64, u64)> {
        let first = self.structured_patch.first()?;
        let last = self.structured_patch.last()?;
        Some((first.new_start, last.new_start + last.new_lines.saturating_sub(1)))
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct MultiEditItem {
    pub old_string: LineCount,
    pub new_string: LineCount,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ToolInput {
    #[serde(deserialize_with = "lenient::string")]
    pub file_path: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    pub notebook_path: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    pub path: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    pub command: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    pub pattern: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    pub url: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    pub query: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    pub description: Option<String>,
    #[serde(deserialize_with = "lenient::string")]
    pub skill: Option<String>,
    pub old_string: LineCount,
    pub new_string: LineCount,
    pub content: LineCount,
    pub edits: Vec<MultiEditItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditTool {
    Edit,
    MultiEdit,
    Write,
}

impl EditTool {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Edit" => Some(Self::Edit),
            "MultiEdit" => Some(Self::MultiEdit),
            "Write" => Some(Self::Write),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Edit => "Edit",
            Self::MultiEdit => "MultiEdit",
            Self::Write => "Write",
        }
    }
}

impl ToolInput {
    pub fn edited_file(&self) -> Option<&str> {
        self.file_path.as_deref().or(self.notebook_path.as_deref())
    }

    /// Provisional +/- before the tool result arrives; replaced by the real patch counts later.
    pub fn estimated_delta(&self, tool: EditTool) -> LineDelta {
        match tool {
            EditTool::Edit => LineDelta { added: self.new_string.0, removed: self.old_string.0 },
            EditTool::Write => LineDelta { added: self.content.0, removed: 0 },
            EditTool::MultiEdit => self.edits.iter().fold(LineDelta::default(), |total, item| LineDelta {
                added: total.added + item.new_string.0,
                removed: total.removed + item.old_string.0,
            }),
        }
    }

    /// Short human-readable target for the activity feed (file, command, pattern…).
    pub fn activity_target(&self) -> Option<String> {
        let command_first_line = self.command.as_deref().and_then(|command| command.lines().next());
        [
            self.file_path.as_deref(),
            self.notebook_path.as_deref(),
            command_first_line,
            self.pattern.as_deref(),
            self.url.as_deref(),
            self.query.as_deref(),
            self.skill.as_deref(),
            self.description.as_deref(),
            self.path.as_deref(),
        ]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|candidate| !candidate.is_empty())
        .map(|target| truncate_chars(target, TARGET_MAX_CHARS))
    }
}

pub fn truncate_chars(text: &str, max_chars: usize) -> String {
    match text.char_indices().nth(max_chars) {
        Some((byte_index, _)) => format!("{}…", &text[..byte_index]),
        None => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_input_counts_lines_without_keeping_text() {
        let input: ToolInput = serde_json::from_str(
            r#"{"file_path":"/repo/a.ts","old_string":"one\ntwo","new_string":"one\ntwo\nthree","replace_all":false}"#,
        )
        .unwrap();
        assert_eq!(input.edited_file(), Some("/repo/a.ts"));
        assert_eq!(input.estimated_delta(EditTool::Edit), LineDelta { added: 3, removed: 2 });
    }

    #[test]
    fn write_counts_content_lines() {
        let input: ToolInput =
            serde_json::from_str(r##"{"file_path":"/repo/new.md","content":"# Title\n\nBody\n"}"##).unwrap();
        assert_eq!(input.estimated_delta(EditTool::Write), LineDelta { added: 3, removed: 0 });
    }

    #[test]
    fn edit_outcome_sums_hunks_and_range() {
        let outcome: EditOutcome = serde_json::from_str(
            r#"{"filePath":"/repo/a.ts","originalFile":"x","structuredPatch":[
                {"oldStart":10,"oldLines":3,"newStart":10,"newLines":4,"lines":[" a","-b","+c","+d"," e"]},
                {"oldStart":40,"oldLines":1,"newStart":41,"newLines":1,"lines":["-x","+y"]}
            ]}"#,
        )
        .unwrap();
        assert_eq!(outcome.delta(), LineDelta { added: 3, removed: 2 });
        assert_eq!(outcome.new_line_range(), Some((10, 41)));
        assert!(!outcome.is_new_file());
    }

    #[test]
    fn create_outcome_is_new_file() {
        let outcome: EditOutcome =
            serde_json::from_str(r#"{"type":"create","filePath":"/repo/n.ts","content":"a","structuredPatch":[]}"#)
                .unwrap();
        assert!(outcome.is_new_file());
        assert_eq!(outcome.new_line_range(), None);
    }

    #[test]
    fn activity_target_prefers_file_then_first_command_line() {
        let bash: ToolInput =
            serde_json::from_str(r#"{"command":"npm test\n# second line","description":"Run tests"}"#).unwrap();
        assert_eq!(bash.activity_target().as_deref(), Some("npm test"));
        let grep: ToolInput = serde_json::from_str(r#"{"pattern":"TODO","path":"src"}"#).unwrap();
        assert_eq!(grep.activity_target().as_deref(), Some("TODO"));
    }

    #[test]
    fn unexpected_field_types_do_not_fail() {
        let input: ToolInput =
            serde_json::from_str(r#"{"file_path":42,"old_string":{"x":1},"edits":[]}"#).unwrap();
        assert_eq!(input.edited_file(), None);
    }

    #[test]
    fn truncates_on_char_boundaries() {
        assert_eq!(truncate_chars("ação", 2), "aç…");
        assert_eq!(truncate_chars("abc", 5), "abc");
    }
}
