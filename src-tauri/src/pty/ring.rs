//! Raw terminal output kept for replay when the terminal view is re-attached.

use std::collections::VecDeque;

/// Rough bytes per terminal line, used to turn the scrollback setting (lines) into a byte budget.
const BYTES_PER_LINE: usize = 240;

pub struct OutputRing {
    bytes: VecDeque<u8>,
    capacity: usize,
}

impl OutputRing {
    pub fn with_scrollback_lines(lines: u32) -> Self {
        let capacity = (lines as usize).saturating_mul(BYTES_PER_LINE).max(BYTES_PER_LINE);
        Self { bytes: VecDeque::with_capacity(capacity.min(1 << 20)), capacity }
    }

    pub fn push(&mut self, chunk: &[u8]) {
        self.bytes.extend(chunk);
        if self.bytes.len() <= self.capacity {
            return;
        }
        let overflow = self.bytes.len() - self.capacity;
        self.bytes.drain(..overflow);
        // Start the replay at a line boundary so it never begins inside an escape sequence.
        if let Some(newline) = self.bytes.iter().position(|byte| *byte == b'\n') {
            self.bytes.drain(..=newline);
        }
    }

    pub fn snapshot(&self) -> Vec<u8> {
        self.bytes.iter().copied().collect()
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_everything_below_capacity() {
        let mut ring = OutputRing::with_scrollback_lines(10);
        ring.push(b"hello\r\nworld");
        assert_eq!(ring.snapshot(), b"hello\r\nworld");
    }

    #[test]
    fn trims_oldest_output_at_a_line_boundary() {
        let mut ring = OutputRing::with_scrollback_lines(1);
        let long_line = vec![b'a'; BYTES_PER_LINE];
        ring.push(&long_line);
        ring.push(b"\nlast line");
        let kept = ring.snapshot();
        assert!(kept.len() <= BYTES_PER_LINE);
        assert_eq!(kept, b"last line");
    }
}
