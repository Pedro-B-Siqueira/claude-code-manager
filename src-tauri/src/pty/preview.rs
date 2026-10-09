//! Text preview of the terminal for session cards, rendered by a headless VT100 emulator so the
//! grid never needs a live xterm instance per session.

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
}
