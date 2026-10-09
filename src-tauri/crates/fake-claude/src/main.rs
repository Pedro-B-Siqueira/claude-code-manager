//! Stand-in for the real `claude` CLI, selected through `CCM_CLAUDE_BIN`. Automated tests use it so
//! they never spend quota. It echoes what is typed and, when given `--settings`, calls the HTTP
//! hooks declared there the way Claude Code does:
//! - start → `SessionStart`; a line → `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `Stop`;
//! - `perm <command>` → `PermissionRequest` + `Notification` (permission_prompt);
//! - `/exit` or end of input → `SessionEnd`.

use std::env;
use std::io::{self, BufRead, Read, Write};
use std::net::TcpStream;

use serde_json::{Value, json};

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
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "fake-claude ready args={}", arguments.join(" "));
    let _ = stdout.flush();
    hooks.send("SessionStart", json!({ "source": "startup" }));
    for line in io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim().to_owned();
        if line == "/exit" {
            let _ = writeln!(stdout, "fake-claude bye");
            break;
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
    hooks.send("SessionEnd", json!({ "reason": "prompt_input_exit" }));
}
