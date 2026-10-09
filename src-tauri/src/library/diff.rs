//! Line diff between an edit's `old_string` and `new_string`, compacted to the changed lines plus
//! a little context. Small inputs only: oversized edits fall back to plain removed/added blocks.

use serde::Serialize;

const CONTEXT_LINES: usize = 3;
const MAX_LCS_LINES: usize = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
    Gap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub text: String,
}

impl DiffLine {
    fn new(kind: DiffLineKind, text: &str) -> Self {
        Self { kind, text: text.to_owned() }
    }
}

pub fn line_diff(old: &str, new: &str) -> Vec<DiffLine> {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let full = if old_lines.len() > MAX_LCS_LINES || new_lines.len() > MAX_LCS_LINES {
        old_lines.iter().map(|line| DiffLine::new(DiffLineKind::Removed, line)).chain(new_lines.iter().map(|line| DiffLine::new(DiffLineKind::Added, line))).collect()
    } else {
        lcs_diff(&old_lines, &new_lines)
    };
    compact(full)
}

pub fn added_lines(content: &str) -> Vec<DiffLine> {
    content.lines().map(|line| DiffLine::new(DiffLineKind::Added, line)).collect()
}

fn lcs_diff(old: &[&str], new: &[&str]) -> Vec<DiffLine> {
    let columns = new.len() + 1;
    let mut lengths = vec![0u16; (old.len() + 1) * columns];
    for row in (0..old.len()).rev() {
        for column in (0..new.len()).rev() {
            lengths[row * columns + column] = if old[row] == new[column] {
                lengths[(row + 1) * columns + column + 1] + 1
            } else {
                lengths[(row + 1) * columns + column].max(lengths[row * columns + column + 1])
            };
        }
    }
    let (mut row, mut column) = (0, 0);
    let mut lines = Vec::with_capacity(old.len() + new.len());
    while row < old.len() && column < new.len() {
        if old[row] == new[column] {
            lines.push(DiffLine::new(DiffLineKind::Context, old[row]));
            row += 1;
            column += 1;
        } else if lengths[(row + 1) * columns + column] >= lengths[row * columns + column + 1] {
            lines.push(DiffLine::new(DiffLineKind::Removed, old[row]));
            row += 1;
        } else {
            lines.push(DiffLine::new(DiffLineKind::Added, new[column]));
            column += 1;
        }
    }
    lines.extend(old[row..].iter().map(|line| DiffLine::new(DiffLineKind::Removed, line)));
    lines.extend(new[column..].iter().map(|line| DiffLine::new(DiffLineKind::Added, line)));
    lines
}

/// Keeps changed lines with up to three lines of context around them; long unchanged runs become a gap.
fn compact(lines: Vec<DiffLine>) -> Vec<DiffLine> {
    let changed: Vec<usize> = lines.iter().enumerate().filter(|(_, line)| line.kind != DiffLineKind::Context).map(|(index, _)| index).collect();
    if changed.is_empty() {
        return Vec::new();
    }
    let near_change = |index: usize| changed.iter().any(|changed_index| changed_index.abs_diff(index) <= CONTEXT_LINES);
    let mut compacted = Vec::new();
    let mut skipped = false;
    for (index, line) in lines.into_iter().enumerate() {
        if line.kind != DiffLineKind::Context || near_change(index) {
            if skipped && !compacted.is_empty() {
                compacted.push(DiffLine::new(DiffLineKind::Gap, "…"));
            }
            skipped = false;
            compacted.push(line);
        } else {
            skipped = true;
        }
    }
    compacted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(lines: &[DiffLine]) -> Vec<(DiffLineKind, &str)> {
        lines.iter().map(|line| (line.kind, line.text.as_str())).collect()
    }

    #[test]
    fn marks_replaced_and_inserted_lines() {
        let diff = line_diff("a\nb\nc", "a\nB\nc\nd");
        assert_eq!(
            kinds(&diff),
            vec![
                (DiffLineKind::Context, "a"),
                (DiffLineKind::Removed, "b"),
                (DiffLineKind::Added, "B"),
                (DiffLineKind::Context, "c"),
                (DiffLineKind::Added, "d"),
            ]
        );
    }

    #[test]
    fn collapses_long_unchanged_runs() {
        let old: String = (1..=20).map(|number| format!("line {number}\n")).collect();
        let new = old.replace("line 10\n", "line ten\n");
        let diff = line_diff(&old, &new);
        assert_eq!(diff.first().map(|line| line.text.as_str()), Some("line 7"));
        assert_eq!(diff.iter().filter(|line| line.kind == DiffLineKind::Context).count(), 6);
        assert_eq!(diff.last().map(|line| line.text.as_str()), Some("line 13"));
    }

    #[test]
    fn identical_text_has_no_diff() {
        assert!(line_diff("same\ntext", "same\ntext").is_empty());
    }

    #[test]
    fn huge_edits_fall_back_to_blocks() {
        let old = "x\n".repeat(MAX_LCS_LINES + 1);
        let diff = line_diff(&old, "y");
        assert_eq!(diff.last().map(|line| line.kind), Some(DiffLineKind::Added));
        assert!(diff.iter().all(|line| line.kind != DiffLineKind::Context));
    }
}
