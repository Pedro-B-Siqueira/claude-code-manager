//! Incremental reading: only complete lines appended after a saved byte offset are processed.
//! A trailing line without `\n` is still being written and is left for the next pass.

use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

const READ_BUFFER_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileIdentity {
    pub inode: u64,
    pub size: u64,
    pub modified_ms: i64,
}

pub fn identity(path: &Path) -> io::Result<FileIdentity> {
    let metadata = fs::metadata(path)?;
    let modified_ms = metadata.mtime() * 1_000 + metadata.mtime_nsec() / 1_000_000;
    Ok(FileIdentity { inode: metadata.ino(), size: metadata.len(), modified_ms })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadProgress {
    /// Offset just past the last complete line consumed.
    pub end_offset: u64,
    /// `false` when `max_lines` stopped the pass before the end of the file.
    pub reached_end: bool,
}

/// Calls `on_line(offset, bytes)` for up to `max_lines` complete lines starting at `start`.
pub fn read_complete_lines(
    path: &Path,
    start: u64,
    max_lines: usize,
    mut on_line: impl FnMut(u64, &[u8]),
) -> io::Result<ReadProgress> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(start))?;
    let mut reader = BufReader::with_capacity(READ_BUFFER_BYTES, file);
    let mut line = Vec::new();
    let mut offset = start;
    for _ in 0..max_lines {
        line.clear();
        let read = reader.read_until(b'\n', &mut line)?;
        if read == 0 || line.last() != Some(&b'\n') {
            return Ok(ReadProgress { end_offset: offset, reached_end: true });
        }
        let content = &line[..line.len() - 1];
        if !content.iter().all(u8::is_ascii_whitespace) {
            on_line(offset, content);
        }
        offset += read as u64;
    }
    Ok(ReadProgress { end_offset: offset, reached_end: false })
}

/// Reads one line back by its offset and length (used to load a diff on demand).
pub fn read_line_at(path: &Path, offset: u64, length: u64) -> io::Result<Vec<u8>> {
    use std::io::Read;
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = vec![0; usize::try_from(length).unwrap_or(0)];
    file.read_exact(&mut bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn collect(path: &Path, start: u64) -> (Vec<(u64, String)>, u64) {
        let mut lines = Vec::new();
        let progress = read_complete_lines(path, start, usize::MAX, |offset, bytes| {
            lines.push((offset, String::from_utf8_lossy(bytes).into_owned()));
        })
        .unwrap();
        (lines, progress.end_offset)
    }

    #[test]
    fn stops_after_max_lines() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("s.jsonl");
        fs::write(&path, "a\nb\nc\n").unwrap();
        let progress = read_complete_lines(&path, 0, 2, |_, _| {}).unwrap();
        assert_eq!(progress, ReadProgress { end_offset: 4, reached_end: false });
        let rest = read_complete_lines(&path, 4, 2, |_, _| {}).unwrap();
        assert_eq!(rest, ReadProgress { end_offset: 6, reached_end: true });
    }

    #[test]
    fn leaves_partial_trailing_line_for_later() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("s.jsonl");
        fs::write(&path, "{\"a\":1}\n{\"b\":").unwrap();
        let (lines, end) = collect(&path, 0);
        assert_eq!(lines, vec![(0, "{\"a\":1}".to_owned())]);
        assert_eq!(end, 8);

        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"2}\n").unwrap();
        let (lines, end) = collect(&path, end);
        assert_eq!(lines, vec![(8, "{\"b\":2}".to_owned())]);
        assert_eq!(end, 16);
    }

    #[test]
    fn reads_a_line_back_by_offset() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("s.jsonl");
        fs::write(&path, "first\nsecond\n").unwrap();
        assert_eq!(read_line_at(&path, 6, 6).unwrap(), b"second");
    }

    #[test]
    fn blank_lines_are_skipped_but_counted() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("s.jsonl");
        fs::write(&path, "\n  \nx\n").unwrap();
        let (lines, end) = collect(&path, 0);
        assert_eq!(lines, vec![(4, "x".to_owned())]);
        assert_eq!(end, 6);
    }
}
