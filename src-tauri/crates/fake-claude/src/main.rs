//! Stand-in for the real `claude` CLI used by automated tests (selected via `CCM_CLAUDE_BIN`).
//! Stage 1 only echoes its arguments; later stages add transcript lines and hook calls.

use std::env;
use std::io::{self, Write};

fn main() {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.iter().any(|argument| argument == "--version") {
        println!("0.0.0-fake (Claude Code)");
        return;
    }
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "fake-claude started with: {}", arguments.join(" "));
}
