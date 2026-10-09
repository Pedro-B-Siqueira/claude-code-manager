//! Terminal sessions end to end with the fake `claude` binary (never the real one).

use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use ccm_lib::error::AppError;
use ccm_lib::pty::{CommandLine, LaunchMode, LaunchSpec, OutputSink, PtyEvent, PtyManager, SpawnRequest};
use ccm_lib::shell_env::shell_quote;

const WAIT: Duration = Duration::from_secs(10);

/// Always rebuilt (a no-op when up to date) so edits to the fake binary are never tested stale.
fn fake_claude_binary() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let status = Command::new(cargo)
        .args(["build", "--quiet", "-p", "fake-claude", "--manifest-path"])
        .arg(manifest.join("Cargo.toml"))
        .status()
        .expect("cargo build fake-claude");
    assert!(status.success());
    manifest.join("target/debug/fake-claude")
}

#[derive(Clone, Default)]
struct CollectingSink(Arc<Mutex<Vec<u8>>>);

impl CollectingSink {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

impl OutputSink for CollectingSink {
    fn deliver(&self, bytes: &[u8]) -> bool {
        self.0.lock().unwrap().extend_from_slice(bytes);
        true
    }
}

struct Harness {
    manager: PtyManager,
    events: Arc<Mutex<Vec<PtyEvent>>>,
    workdir: tempfile::TempDir,
}

impl Harness {
    fn new() -> Self {
        let events: Arc<Mutex<Vec<PtyEvent>>> = Arc::default();
        let recorder = Arc::clone(&events);
        let manager = PtyManager::new(Arc::new(move |event| recorder.lock().unwrap().push(event)));
        Self { manager, events, workdir: tempfile::tempdir().unwrap() }
    }

    /// Mirrors the app's launch script, with `/bin/sh` instead of the user's login shell.
    fn spawn(&self, key: &str) -> CollectingSink {
        let script = format!("{} --session-id {key}; exec /bin/sh", shell_quote(&fake_claude_binary().to_string_lossy()));
        let launch = LaunchSpec { session_id: key.to_owned(), cwd: self.workdir.path().to_path_buf(), mode: LaunchMode::New, worktree: None };
        let command = CommandLine { program: PathBuf::from("/bin/sh"), args: vec!["-c".to_owned(), script], env: vec![("PS1".to_owned(), "$ ".to_owned())], env_remove: vec![] };
        self.manager.spawn(SpawnRequest { key: key.to_owned(), launch, command, scrollback_lines: 1_000 }).expect("spawn");
        let sink = CollectingSink::default();
        self.manager.attach(key, Box::new(sink.clone())).unwrap();
        sink
    }

    /// Runs an arbitrary script in the PTY (for lifecycle tests that need stubborn processes).
    fn spawn_script(&self, key: &str, script: &str) -> CollectingSink {
        let launch = LaunchSpec { session_id: key.to_owned(), cwd: self.workdir.path().to_path_buf(), mode: LaunchMode::New, worktree: None };
        let command = CommandLine { program: PathBuf::from("/bin/sh"), args: vec!["-c".to_owned(), script.to_owned()], env: vec![], env_remove: vec![] };
        self.manager.spawn(SpawnRequest { key: key.to_owned(), launch, command, scrollback_lines: 500 }).expect("spawn");
        let sink = CollectingSink::default();
        self.manager.attach(key, Box::new(sink.clone())).unwrap();
        sink
    }

    fn exited(&self, key: &str) -> Option<Option<i32>> {
        self.events.lock().unwrap().iter().find_map(|event| match event {
            PtyEvent::Exited { key: exited_key, code, .. } if exited_key == key => Some(*code),
            _ => None,
        })
    }
}

fn wait_until(description: &str, condition: impl Fn() -> bool) {
    let started = Instant::now();
    while !condition() {
        assert!(started.elapsed() < WAIT, "timed out waiting for {description}");
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn new_session_runs_claude_with_its_session_id_and_echoes_input() {
    let harness = Harness::new();
    let sink = harness.spawn("s1");
    wait_until("fake claude banner", || sink.text().contains("fake-claude ready args=--session-id s1"));
    harness.manager.write("s1", b"hello\n").unwrap();
    wait_until("echo", || sink.text().contains("> hello"));
}

#[test]
fn shell_stays_open_after_claude_exits_and_exit_is_reported() {
    let harness = Harness::new();
    let sink = harness.spawn("s2");
    wait_until("banner", || sink.text().contains("fake-claude ready"));
    harness.manager.write("s2", b"/exit\n").unwrap();
    wait_until("claude exit", || sink.text().contains("fake-claude bye"));
    harness.manager.write("s2", b"echo still-$((40+2))\n").unwrap();
    wait_until("shell output", || sink.text().contains("still-42"));
    assert!(harness.exited("s2").is_none(), "the shell keeps the terminal alive");
    harness.manager.write("s2", b"exit 0\n").unwrap();
    wait_until("exit event", || harness.exited("s2").is_some());
    assert_eq!(harness.exited("s2"), Some(Some(0)));
}

#[test]
fn reattaching_replays_previous_output() {
    let harness = Harness::new();
    let sink = harness.spawn("s3");
    harness.manager.write("s3", b"first line\n").unwrap();
    wait_until("echo", || sink.text().contains("> first line"));
    harness.manager.detach("s3");
    let second_view = CollectingSink::default();
    harness.manager.attach("s3", Box::new(second_view.clone())).unwrap();
    assert!(second_view.text().contains("> first line"));
}

#[test]
fn closing_a_running_session_ends_the_whole_process_group() {
    let harness = Harness::new();
    let sink = harness.spawn("s4");
    wait_until("banner", || sink.text().contains("fake-claude ready"));
    harness.manager.close("s4", Duration::from_millis(500)).unwrap();
    wait_until("exit after close", || harness.exited("s4").is_some());
    assert!(harness.manager.running_keys().is_empty());
}

#[test]
fn preview_reflects_the_terminal_screen() {
    let harness = Harness::new();
    let sink = harness.spawn("s5");
    harness.manager.write("s5", b"preview me\n").unwrap();
    wait_until("echo", || sink.text().contains("> preview me"));
    wait_until("preview event", || {
        harness.events.lock().unwrap().iter().any(|event| matches!(event, PtyEvent::PreviewChanged { key, lines } if key == "s5" && lines.iter().any(|line| line.contains("> preview me"))))
    });
}

#[test]
fn missing_working_directory_is_rejected() {
    let harness = Harness::new();
    let launch = LaunchSpec { session_id: "x".to_owned(), cwd: PathBuf::from("/nonexistent/ccm"), mode: LaunchMode::Resume, worktree: None };
    let command = CommandLine { program: PathBuf::from("/bin/sh"), args: vec![], env: vec![], env_remove: vec![] };
    let result = harness.manager.spawn(SpawnRequest { key: "x".to_owned(), launch, command, scrollback_lines: 100 });
    assert!(matches!(result, Err(AppError::MissingDirectory(_))));
}

#[test]
fn hibernating_ends_the_process_but_keeps_the_card() {
    let harness = Harness::new();
    let sink = harness.spawn("s6");
    wait_until("banner", || sink.text().contains("fake-claude ready"));
    harness.manager.hibernate("s6", Duration::from_millis(300)).unwrap();
    wait_until("exit after hibernation", || harness.exited("s6").is_some());
    let snapshot = harness.manager.snapshot("s6").unwrap();
    assert!(snapshot.exited && snapshot.hibernated);
    assert!(harness.manager.running_keys().is_empty());
}

fn group_alive(pid: i32) -> bool {
    unsafe { libc::killpg(pid, 0) == 0 }
}

#[test]
fn close_escalates_to_sigkill_when_a_child_ignores_sighup() {
    let harness = Harness::new();
    let sink = harness.spawn_script("stubborn", "(trap '' HUP; echo child-ready; exec sleep 60) & wait");
    wait_until("child running", || sink.text().contains("child-ready"));
    let pid = harness.manager.snapshot("stubborn").unwrap().pid.unwrap() as i32;
    harness.manager.close("stubborn", Duration::from_millis(300)).unwrap();
    wait_until("whole process group gone", || !group_alive(pid));
}

#[test]
fn shutdown_kills_groups_whose_leader_already_left() {
    let harness = Harness::new();
    let sink = harness.spawn_script("orphans", "(trap '' HUP; echo child-ready; exec sleep 60) & wait");
    wait_until("child running", || sink.text().contains("child-ready"));
    let pid = harness.manager.snapshot("orphans").unwrap().pid.unwrap() as i32;
    harness.manager.shutdown_all(Duration::from_millis(300));
    wait_until("whole process group gone", || !group_alive(pid));
}

#[test]
fn a_reused_key_keeps_old_and_new_processes_apart() {
    let harness = Harness::new();
    let first = harness.spawn("reused");
    wait_until("first banner", || first.text().contains("fake-claude ready"));
    let old_instance = harness.manager.snapshot("reused").unwrap().instance;
    harness.manager.hibernate("reused", Duration::from_millis(300)).unwrap();
    harness.manager.wait_for_exit("reused", Duration::from_secs(5));
    let second = harness.spawn("reused");
    wait_until("second banner", || second.text().contains("fake-claude ready"));
    let new_instance = harness.manager.snapshot("reused").unwrap().instance;
    assert_ne!(old_instance, new_instance);
    assert!(!harness.manager.is_current("reused", old_instance));
    assert!(harness.manager.is_current("reused", new_instance));
    assert!(!harness.manager.snapshot("reused").unwrap().hibernated, "the new process starts awake");
    harness.manager.close("reused", Duration::from_millis(300)).unwrap();
}
