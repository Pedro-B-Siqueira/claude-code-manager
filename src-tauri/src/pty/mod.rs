//! Embedded terminals: one PTY per session, output kept for replay and summarized for cards.

mod manager;
pub mod preview;
pub mod ring;
mod session;

use std::sync::Arc;

pub use manager::{CommandLine, PtyManager, SpawnRequest};
pub use session::{LaunchMode, LaunchSpec, SessionSnapshot, now_ms};

/// Receives terminal output for the attached view. Returns `false` once the receiver is gone.
pub trait OutputSink: Send {
    fn deliver(&self, bytes: &[u8]) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtyEvent {
    PreviewChanged { key: String, lines: Vec<String> },
    Exited { key: String, code: Option<i32> },
}

pub type PtyNotifier = Arc<dyn Fn(PtyEvent) + Send + Sync>;
