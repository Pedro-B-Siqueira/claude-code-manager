//! Stand-in for the real `claude` CLI, selected through `CCM_CLAUDE_BIN`. Automated tests use it so
//! they never spend quota: it echoes what is typed and exits on `/exit` or end of input.

use std::env;
use std::io::{self, BufRead, Write};

fn main() {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.iter().any(|argument| argument == "--version") {
        println!("0.0.0-fake (Claude Code)");
        return;
    }
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "fake-claude ready args={}", arguments.join(" "));
    let _ = stdout.flush();
    for line in io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.trim() == "/exit" {
            let _ = writeln!(stdout, "fake-claude bye");
            break;
        }
        let _ = writeln!(stdout, "> {line}");
        let _ = stdout.flush();
    }
}
