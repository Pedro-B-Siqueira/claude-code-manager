//! Hook server security checks and the full chain: fake `claude` reads `--settings`, calls the HTTP
//! hooks with its session token, and the tracker derives the session status.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use ccm_lib::hooks::server::{HookEvent, HookServer};
use ccm_lib::hooks::settings_file::{SESSION_ENV, TOKEN_ENV, hook_settings};
use ccm_lib::hooks::tokens::TokenRegistry;
use ccm_lib::pty::{CommandLine, LaunchMode, LaunchSpec, OutputSink, PtyManager, SpawnRequest};
use ccm_lib::shell_env::shell_quote;
use ccm_lib::status::{SessionStatus, StatusTracker, status_for_hook};

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

    wait_until("accepted event", || !recorder.events.lock().unwrap().is_empty());
    thread::sleep(Duration::from_millis(100));
    let events = recorder.events.lock().unwrap();
    assert_eq!(events.len(), 1, "only the authenticated request is delivered");
    assert_eq!((events[0].session_key.as_str(), events[0].payload.event()), ("session-a", "Stop"));
}

#[test]
fn server_listens_on_loopback_only() {
    let recorder = start_server();
    let port = recorder.server.port();
    assert!(TcpStream::connect(("127.0.0.1", port)).is_ok());
    let non_loopback = std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|socket| socket.connect("192.0.2.1:9").map(|()| socket))
        .and_then(|socket| socket.local_addr())
        .map(|address| address.ip());
    if let Ok(ip) = non_loopback {
        if !ip.is_loopback() && !ip.is_unspecified() {
            assert!(TcpStream::connect_timeout(&(ip, port).into(), Duration::from_millis(300)).is_err());
        }
    }
}

#[derive(Clone, Default)]
struct CollectingSink(Arc<Mutex<Vec<u8>>>);

impl OutputSink for CollectingSink {
    fn deliver(&self, bytes: &[u8]) -> bool {
        self.0.lock().unwrap().extend_from_slice(bytes);
        true
    }
}

fn fake_claude_binary() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let status = Command::new(cargo).args(["build", "--quiet", "-p", "fake-claude", "--manifest-path"]).arg(manifest.join("Cargo.toml")).status().unwrap();
    assert!(status.success());
    manifest.join("target/debug/fake-claude")
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
