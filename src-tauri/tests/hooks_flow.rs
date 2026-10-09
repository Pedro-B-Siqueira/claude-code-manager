//! Hook server security checks and the full chain: fake `claude` reads `--settings`, calls the HTTP
//! hooks with its session token, and the tracker derives the session status.

mod support;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use ccm_lib::hooks::server::{HookEvent, HookServer};
use ccm_lib::hooks::settings_file::{SESSION_ENV, TOKEN_ENV, hook_settings};
use ccm_lib::hooks::tokens::TokenRegistry;
use ccm_lib::pty::{CommandLine, LaunchMode, LaunchSpec, OutputSink, PtyManager, SpawnRequest};
use ccm_lib::shell_env::shell_quote;
use ccm_lib::status::{SessionStatus, StatusTracker, status_for_hook};
use support::fake_claude_binary;

const WAIT: Duration = Duration::from_secs(10);

struct Recorder {
    events: Arc<Mutex<Vec<HookEvent>>>,
    server: HookServer,
    tokens: Arc<TokenRegistry>,
}

fn start_server() -> Recorder {
    let tokens = Arc::new(TokenRegistry::default());
    let events: Arc<Mutex<Vec<HookEvent>>> = Arc::default();
    let sink = Arc::clone(&events);
    let server = HookServer::start(Arc::clone(&tokens), Arc::new(move |event| sink.lock().unwrap().push(event))).unwrap();
    Recorder { events, server, tokens }
}

fn raw_request(port: u16, request: &str) -> u16 {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response.split_whitespace().nth(1).and_then(|code| code.parse().ok()).unwrap_or(0)
}

fn post(port: u16, path: &str, session: &str, token: &str, body: &str) -> u16 {
    raw_request(
        port,
        &format!("POST {path} HTTP/1.1\r\nHost: x\r\nX-CCM-Session: {session}\r\nX-CCM-Token: {token}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()),
    )
}

fn wait_until(description: &str, condition: impl Fn() -> bool) {
    let started = Instant::now();
    while !condition() {
        assert!(started.elapsed() < WAIT, "timed out waiting for {description}");
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn server_accepts_only_authenticated_hook_posts() {
    let recorder = start_server();
    let port = recorder.server.port();
    let token = recorder.tokens.issue("session-a");
    let body = r#"{"hook_event_name":"Stop","session_id":"x"}"#;

    assert_eq!(post(port, "/hook", "session-a", &token, body), 200);
    assert_eq!(post(port, "/hook", "session-a", "wrong-token", body), 401);
    assert_eq!(post(port, "/hook", "session-b", &token, body), 401);
    assert_eq!(post(port, "/other", "session-a", &token, body), 404);
    assert_eq!(post(port, "/hook", "session-a", &token, "{not json"), 400);
    assert_eq!(raw_request(port, "GET /hook HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n"), 405);

    // Requests are handled in order, so once this sentinel arrives every earlier one was processed.
    assert_eq!(post(port, "/hook", "session-a", &token, r#"{"hook_event_name":"SessionEnd"}"#), 200);
    wait_until("sentinel", || recorder.events.lock().unwrap().iter().any(|event| event.payload.event() == "SessionEnd"));
    let events = recorder.events.lock().unwrap();
    let delivered: Vec<&str> = events.iter().map(|event| event.payload.event()).collect();
    assert_eq!(delivered, vec!["Stop", "SessionEnd"], "only authenticated requests are delivered");
    assert!(events.iter().all(|event| event.session_key == "session-a"));
}

#[test]
fn server_listens_on_loopback_only() {
    let recorder = start_server();
    assert!(recorder.server.address().ip().is_loopback());
    assert!(TcpStream::connect(("127.0.0.1", recorder.server.port())).is_ok());
}

#[test]
fn oversized_unauthenticated_and_revoked_requests_are_rejected() {
    let recorder = start_server();
    let port = recorder.server.port();
    let token = recorder.tokens.issue("session-a");
    let huge = format!(r#"{{"hook_event_name":"Stop","pad":"{}"}}"#, "x".repeat(2 * 1024 * 1024));
    assert_eq!(post(port, "/hook", "session-a", &token, &huge), 413);
    assert_eq!(raw_request(port, "POST /hook HTTP/1.1\r\nHost: x\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}"), 401);
    recorder.tokens.revoke("session-a");
    assert_eq!(post(port, "/hook", "session-a", &token, r#"{"hook_event_name":"Stop"}"#), 401);
    let preflight = raw_full_response(port, "OPTIONS /hook HTTP/1.1\r\nHost: x\r\nOrigin: https://example.com\r\nConnection: close\r\n\r\n");
    assert!(preflight.starts_with("HTTP/1.1 405"));
    assert!(!preflight.to_lowercase().contains("access-control-allow"), "no CORS: browsers cannot call the hook endpoint");
    assert!(recorder.events.lock().unwrap().is_empty());
}

fn raw_full_response(port: u16, request: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

#[derive(Clone, Default)]
struct CollectingSink(Arc<Mutex<Vec<u8>>>);

impl OutputSink for CollectingSink {
    fn deliver(&self, bytes: &[u8]) -> bool {
        self.0.lock().unwrap().extend_from_slice(bytes);
        true
    }
}

#[test]
fn fake_claude_reports_status_through_injected_hooks() {
    let recorder = start_server();
    let workdir = tempfile::tempdir().unwrap();
    let settings_file = workdir.path().join("hooks.json");
    std::fs::write(&settings_file, hook_settings(recorder.server.port()).to_string()).unwrap();
    let key = "hooked-session";
    let token = recorder.tokens.issue(key);

    let tracker = Arc::new(StatusTracker::default());
    let script = format!(
        "{} --session-id {key} --settings {}; exec /bin/sh",
        shell_quote(&fake_claude_binary().to_string_lossy()),
        shell_quote(&settings_file.to_string_lossy())
    );
    let manager = PtyManager::new(Arc::new(|_| {}));
    let launch = LaunchSpec { session_id: key.to_owned(), cwd: workdir.path().to_path_buf(), mode: LaunchMode::New, worktree: None };
    let env = vec![(TOKEN_ENV.to_owned(), token), (SESSION_ENV.to_owned(), key.to_owned())];
    let command = CommandLine { program: PathBuf::from("/bin/sh"), args: vec!["-c".to_owned(), script], env, env_remove: vec![] };
    manager.spawn(SpawnRequest { key: key.to_owned(), launch, command, scrollback_lines: 500 }).unwrap();
    let output = CollectingSink::default();
    manager.attach(key, Box::new(output.clone())).unwrap();

    let events = Arc::clone(&recorder.events);
    let statuses = |tracker: &StatusTracker| {
        for event in events.lock().unwrap().drain(..) {
            if let Some(update) = status_for_hook(&event.payload) {
                tracker.apply(&event.session_key, update, 0);
            }
        }
        tracker.current(key, 0)
    };

    wait_until("SessionStart", || statuses(&tracker).is_some_and(|current| current.status == SessionStatus::Idle));
    manager.write(key, b"perm npm publish\n").unwrap();
    wait_until("permission", || {
        statuses(&tracker).is_some_and(|current| current.status == SessionStatus::Permission && current.detail.as_deref() == Some("$ npm publish"))
    });
    manager.write(key, b"hello\n").unwrap();
    wait_until("turn finished", || statuses(&tracker).is_some_and(|current| current.status == SessionStatus::Done));
    let text = String::from_utf8_lossy(&output.0.lock().unwrap()).into_owned();
    assert!(text.contains("> hello"));
    manager.close(key, Duration::from_millis(300)).unwrap();
}
