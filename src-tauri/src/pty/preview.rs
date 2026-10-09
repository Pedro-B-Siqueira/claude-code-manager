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

    /// The last non-blank lines on screen, trailing spaces trimmed.
    pub fn preview_lines(&self) -> Vec<String> {
        let contents = self.parser.screen().contents();
        let lines: Vec<&str> = contents.lines().map(str::trim_end).collect();
        let last_content = lines.iter().rposition(|line| !line.is_empty());
        let Some(end) = last_content else { return Vec::new() };
        let start = end.saturating_sub(PREVIEW_LINES - 1);
        lines[start..=end].iter().map(|line| (*line).to_owned()).collect()
    }
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
