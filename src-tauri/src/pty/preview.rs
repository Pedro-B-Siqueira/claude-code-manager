//! Text preview of the terminal for session cards, rendered by a headless VT100 emulator so the
//! grid never needs a live xterm instance per session.

use super::paste::IMAGE_PLACEHOLDER;

const PREVIEW_LINES: usize = 4;

pub struct TerminalScreen {
    parser: vt100::Parser,
}

impl TerminalScreen {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self { parser: vt100::Parser::new(rows, cols, 0) }
    }

    pub fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.parser.screen_mut().set_size(rows, cols);
    }

    pub fn bracketed_paste(&self) -> bool {
        self.parser.screen().bracketed_paste()
    }

    pub fn image_placeholder_count(&self) -> usize {
        count_image_placeholders(&self.parser.screen().contents())
    }

    /// The last non-blank lines on screen, trailing spaces trimmed. Claude Code keeps its prompt box
    /// and status lines at the bottom, so the preview shows what is above them.
    pub fn preview_lines(&self) -> Vec<String> {
        let contents = self.parser.screen().contents();
        let screen_lines: Vec<&str> = contents.lines().map(str::trim_end).collect();
        let lines = &screen_lines[..prompt_box_start(&screen_lines).unwrap_or(screen_lines.len())];
        let last_content = lines.iter().rposition(|line| !line.is_empty());
        let Some(end) = last_content else { return Vec::new() };
        let start = end.saturating_sub(PREVIEW_LINES - 1);
        lines[start..=end].iter().map(|line| (*line).to_owned()).collect()
    }
}

const MIN_RULE_WIDTH: usize = 10;

/// Where Claude Code's prompt box starts: the last pair of horizontal rules with the `❯` input between.
fn prompt_box_start(lines: &[&str]) -> Option<usize> {
    let rules: Vec<usize> = lines.iter().enumerate().filter(|(_, line)| is_rule(line)).map(|(index, _)| index).collect();
    rules
        .windows(2)
        .rev()
        .find(|pair| lines[pair[0] + 1..pair[1]].iter().any(|line| is_prompt(line)))
        .map(|pair| pair[0])
}

/// Mostly `─`: Claude Code may write a label into the border (Ink's `borderText`).
fn is_rule(line: &str) -> bool {
    let visible = line.chars().filter(|character| !character.is_whitespace()).count();
    let dashes = line.chars().filter(|character| *character == '─').count();
    dashes >= MIN_RULE_WIDTH && dashes * 2 >= visible
}

fn is_prompt(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with('❯') || trimmed.starts_with('>')
}

/// Counts `[Image #<digits>]`. Claude Code wraps long prompts itself, so the placeholder can be split
/// across lines, with the input box border (`│`) in between.
fn count_image_placeholders(text: &str) -> usize {
    text.match_indices(IMAGE_PLACEHOLDER)
        .filter(|(start, _)| {
            let rest = text[start + IMAGE_PLACEHOLDER.len()..].trim_start_matches(|character: char| character.is_whitespace() || character == '│');
            let Some(after_hash) = rest.strip_prefix('#') else { return false };
            let digits = after_hash.chars().take_while(char::is_ascii_digit).count();
            digits > 0 && after_hash[digits..].starts_with(']')
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_escape_sequences_into_plain_lines() {
        let mut screen = TerminalScreen::new(10, 40);
        screen.process(b"\x1b[1;32mok\x1b[0m line one\r\nline two\r\n\r\n");
        assert_eq!(screen.preview_lines(), vec!["ok line one".to_owned(), "line two".to_owned()]);
    }

    #[test]
    fn keeps_only_the_last_lines() {
        let mut screen = TerminalScreen::new(20, 40);
        for index in 0..10 {
            screen.process(format!("line {index}\r\n").as_bytes());
        }
        assert_eq!(screen.preview_lines(), vec!["line 6", "line 7", "line 8", "line 9"]);
    }

    #[test]
    fn redraws_replace_previous_content() {
        let mut screen = TerminalScreen::new(5, 40);
        screen.process(b"working...");
        screen.process(b"\r\x1b[2Kdone");
        assert_eq!(screen.preview_lines(), vec!["done"]);
    }

    #[test]
    fn empty_screen_has_no_preview() {
        assert!(TerminalScreen::new(5, 20).preview_lines().is_empty());
    }

    #[test]
    fn tracks_bracketed_paste_mode() {
        let mut screen = TerminalScreen::new(5, 40);
        assert!(!screen.bracketed_paste());
        screen.process(b"\x1b[?2004h");
        assert!(screen.bracketed_paste());
        screen.process(b"\x1b[?2004l");
        assert!(!screen.bracketed_paste());
    }

    /// Claude Code 2.x: activity, then its prompt box (rule, `❯`, rule) and status lines at the bottom.
    const CLAUDE_SCREEN: &[u8] = "\u{23fa} Update(src/orders/table.tsx)\r\n  \u{23bf} Updated with 12 additions\r\n\r\n\u{273b} Working\u{2026} (esc to interrupt)\r\n\
\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\r\n\
\u{276f} \r\n\
\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\r\n\
  \u{25b8}\u{25b8} auto mode on (shift+tab to cycle)".as_bytes();

    #[test]
    fn skips_claude_codes_prompt_box_and_status_lines() {
        let mut screen = TerminalScreen::new(12, 60);
        screen.process(CLAUDE_SCREEN);
        assert_eq!(
            screen.preview_lines(),
            vec!["\u{23fa} Update(src/orders/table.tsx)", "  \u{23bf} Updated with 12 additions", "", "\u{273b} Working\u{2026} (esc to interrupt)"]
        );
    }

    /// Claude Code can write a label into the prompt box border (Ink's `borderText`), at either end.
    #[test]
    fn finds_the_prompt_box_when_its_border_carries_a_label() {
        let rule = "\u{2500}".repeat(60);
        for top in [format!("{} Projetos (meta outubro) \u{2500}\u{2500}", "\u{2500}".repeat(35)), format!("Projetos {}", "\u{2500}".repeat(51))] {
            let mut screen = TerminalScreen::new(10, 60);
            let text = format!("\u{23fa} Update(src/a.ts)\r\n  \u{23bf} Updated\r\n\r\n{top}\r\n\u{276f} \r\n{rule}\r\n  \u{25b8}\u{25b8} auto mode on");
            screen.process(text.as_bytes());
            assert_eq!(screen.preview_lines(), vec!["\u{23fa} Update(src/a.ts)", "  \u{23bf} Updated"], "top border: {top}");
        }
    }

    #[test]
    fn rules_without_a_prompt_between_them_are_ordinary_output() {
        let mut screen = TerminalScreen::new(8, 40);
        screen.process("intro\r\n\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\r\nsection text\r\n\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\r\nend".as_bytes());
        assert_eq!(screen.preview_lines().last().map(String::as_str), Some("end"));
    }

    #[test]
    fn counts_placeholders_split_across_wrapped_prompt_lines() {
        let mut screen = TerminalScreen::new(6, 40);
        screen.process(b"\xe2\x94\x82 > fix the modal [Image\r\n\xe2\x94\x82 #1] and [Image #2]\r\n");
        assert_eq!(screen.image_placeholder_count(), 2);
        screen.process(b"[Image without number] [Images #3]");
        assert_eq!(screen.image_placeholder_count(), 2, "only `[Image #<digits>]` counts");
    }

    #[test]
    fn counts_image_placeholders_on_screen() {
        let mut screen = TerminalScreen::new(5, 60);
        assert_eq!(screen.image_placeholder_count(), 0);
        screen.process(b"> [Image #1] [Image #2] fix the modal");
        assert_eq!(screen.image_placeholder_count(), 2);
    }
}
