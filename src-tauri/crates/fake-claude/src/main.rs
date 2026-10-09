//! Stand-in for the real `claude` CLI, selected through `CCM_CLAUDE_BIN`. Automated tests use it so
//! they never spend quota. It echoes what is typed and, when given `--settings`, calls the HTTP
//! hooks declared there the way Claude Code does:
//! - start → `SessionStart`; a line → `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `Stop`;
//! - `perm <command>` → `PermissionRequest` + `Notification` (permission_prompt);
//! - `/exit` or end of input → `SessionEnd`.

use std::env;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::Path;

use serde_json::{Value, json};

const PASTE_START: &[u8] = b"\x1b[200~";
const PASTE_END: &[u8] = b"\x1b[201~";
const IMAGE_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "gif", "webp"];

/// Like the real CLI, keys are read one byte at a time, so a paste is seen before Enter.
struct RawTerminal {
    original: Option<libc::termios>,
}

impl RawTerminal {
    fn enable() -> Self {
        // SAFETY: termios is plain data filled by tcgetattr; both calls only change stdin's settings.
        unsafe {
            let mut settings: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(libc::STDIN_FILENO, &mut settings) != 0 {
                return Self { original: None };
            }
            let original = settings;
            settings.c_lflag &= !(libc::ICANON | libc::ECHO);
            settings.c_cc[libc::VMIN] = 1;
            settings.c_cc[libc::VTIME] = 0;
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &settings);
            Self { original: Some(original) }
        }
    }
}

impl Drop for RawTerminal {
    fn drop(&mut self) {
        if let Some(original) = self.original {
            // SAFETY: restores exactly what `enable` read, so the shell after us gets a normal terminal.
            unsafe {
                libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &original);
            }
        }
    }
}

enum Input {
    Line(String),
    Paste(String),
}

#[derive(Default)]
struct InputReader {
    line: Vec<u8>,
    paste: Option<Vec<u8>>,
}

impl InputReader {
    fn push(&mut self, byte: u8) -> Option<Input> {
        if let Some(paste) = &mut self.paste {
            paste.push(byte);
            if !paste.ends_with(PASTE_END) {
                return None;
            }
            paste.truncate(paste.len() - PASTE_END.len());
            let text = String::from_utf8_lossy(paste).into_owned();
            self.paste = None;
            return Some(Input::Paste(text));
        }
        if byte == b'\n' || byte == b'\r' {
            let line = String::from_utf8_lossy(&self.line).into_owned();
            self.line.clear();
            return Some(Input::Line(line));
        }
        self.line.push(byte);
        if self.line.ends_with(PASTE_START) {
            self.line.truncate(self.line.len() - PASTE_START.len());
            self.paste = Some(Vec::new());
        }
        None
    }

    fn append(&mut self, text: &str) {
        self.line.extend_from_slice(text.as_bytes());
    }
}

/// Claude Code splits a paste at line breaks and at spaces followed by `/`.
fn split_paste(text: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut start = 0;
    for (index, _) in text.match_indices(" /") {
        pieces.push(&text[start..index]);
        start = index + 1;
    }
    pieces.push(&text[start..]);
    pieces.into_iter().flat_map(str::lines).filter(|piece| !piece.trim().is_empty()).collect()
}

fn is_image_path(piece: &str) -> bool {
    let path = Path::new(piece);
    let extension = path.extension().and_then(|extension| extension.to_str()).map(str::to_ascii_lowercase);
    extension.is_some_and(|extension| IMAGE_EXTENSIONS.contains(&extension.as_str())) && path.is_file()
}

fn render_paste(text: &str, images: &mut usize) -> String {
    split_paste(text)
        .into_iter()
        .map(|piece| {
            if is_image_path(piece) {
                *images += 1;
                format!("[Image #{images}]")
            } else {
                piece.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

struct HookTarget {
    host_port: String,
    path: String,
    headers: Vec<(String, String)>,
}

struct Hooks {
    session_id: String,
    settings: Value,
}

impl Hooks {
    fn target(&self, event: &str) -> Option<HookTarget> {
        let hook = self.settings.get("hooks")?.get(event)?.get(0)?.get("hooks")?.get(0)?;
        let url = hook.get("url")?.as_str()?.strip_prefix("http://")?;
        let (host_port, path) = url.split_once('/').map(|(host, path)| (host.to_owned(), format!("/{path}")))?;
        let headers = hook
            .get("headers")
            .and_then(Value::as_object)
            .map(|headers| headers.iter().filter_map(|(name, value)| Some((name.clone(), interpolate(value.as_str()?)))).collect())
            .unwrap_or_default();
        Some(HookTarget { host_port, path, headers })
    }

    fn send(&self, event: &str, extra: Value) {
        let Some(target) = self.target(event) else { return };
        let mut body = json!({ "hook_event_name": event, "session_id": self.session_id, "cwd": env::current_dir().map(|dir| dir.display().to_string()).unwrap_or_default() });
        if let (Some(body), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
            body.extend(extra.clone());
        }
        let _ = post(&target, &body.to_string());
    }
}

fn interpolate(value: &str) -> String {
    match value.strip_prefix('$') {
        Some(name) => env::var(name.trim_start_matches('{').trim_end_matches('}')).unwrap_or_default(),
        None => value.to_owned(),
    }
}

fn post(target: &HookTarget, body: &str) -> io::Result<()> {
    let mut stream = TcpStream::connect(&target.host_port)?;
    let mut request = format!("POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n", target.path, target.host_port, body.len());
    for (name, value) in &target.headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    request.push_str("\r\n");
    request.push_str(body);
    stream.write_all(request.as_bytes())?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(())
}

fn argument_after(arguments: &[String], flag: &str) -> Option<String> {
    arguments.iter().position(|argument| argument == flag).and_then(|index| arguments.get(index + 1)).cloned()
}

fn main() {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.iter().any(|argument| argument == "--version") {
        println!("0.0.0-fake (Claude Code)");
        return;
    }
    let session_id = argument_after(&arguments, "--session-id").or_else(|| argument_after(&arguments, "--resume")).unwrap_or_default();
    let settings = argument_after(&arguments, "--settings")
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null);
    let hooks = Hooks { session_id, settings };
    let _raw = RawTerminal::enable();
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "fake-claude ready args={}", arguments.join(" "));
    let _ = write!(stdout, "\x1b[?2004h");
    let _ = stdout.flush();
    hooks.send("SessionStart", json!({ "source": "startup" }));
    let mut reader = InputReader::default();
    let mut images = 0;
    for byte in io::stdin().lock().bytes() {
        let Ok(byte) = byte else { break };
        match reader.push(byte) {
            None => {}
            Some(Input::Paste(text)) => {
                let rendered = render_paste(&text, &mut images);
                reader.append(&rendered);
                let _ = write!(stdout, "{rendered}");
                let _ = stdout.flush();
            }
            Some(Input::Line(line)) => {
                let line = line.trim().to_owned();
                if line.is_empty() {
                    continue;
                }
                if line == "/exit" {
                    let _ = writeln!(stdout, "fake-claude bye");
                    break;
                }
                if line == "nopaste" {
                    let _ = writeln!(stdout, "\x1b[?2004lpaste off");
                    let _ = stdout.flush();
                    continue;
                }
                if let Some(command) = line.strip_prefix("perm ") {
                    hooks.send("PermissionRequest", json!({ "tool_name": "Bash", "tool_input": { "command": command } }));
                    hooks.send("Notification", json!({ "notification_type": "permission_prompt", "message": "Claude needs your permission to use Bash" }));
                    let _ = writeln!(stdout, "permission requested: {command}");
                    let _ = stdout.flush();
                    continue;
                }
                hooks.send("UserPromptSubmit", json!({ "prompt": line }));
                hooks.send("PreToolUse", json!({ "tool_name": "Read", "tool_input": { "file_path": "README.md" } }));
                hooks.send("PostToolUse", json!({ "tool_name": "Read" }));
                let _ = writeln!(stdout, "> {line}");
                let _ = stdout.flush();
                hooks.send("Stop", json!({}));
            }
        }
    }
    hooks.send("SessionEnd", json!({ "reason": "prompt_input_exit" }));
}
