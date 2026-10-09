//! The hibernation sweep against real (fake-claude) sessions: idle ones are ended, busy ones kept.

mod support;

use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use ccm_lib::hibernation::hibernate_idle_sessions;
use ccm_lib::hooks::payload::HookPayload;
use ccm_lib::pty::{CommandLine, LaunchMode, LaunchSpec, SpawnRequest, now_ms};
use ccm_lib::settings::{self, AppSettings, HibernationSettings};
use ccm_lib::state::AppState;
use ccm_lib::status::status_for_hook;
use support::{SandboxHome, fake_claude_binary};

const IDLE_MINUTES: u32 = 5;

fn spawn(state: &AppState, key: &str, cwd: &std::path::Path) {
    let launch = LaunchSpec { session_id: key.to_owned(), cwd: cwd.to_path_buf(), mode: LaunchMode::New, worktree: None };
    let script = format!("'{}' --session-id {key}", fake_claude_binary().display());
    let command = CommandLine { program: PathBuf::from("/bin/sh"), args: vec!["-c".to_owned(), script], env: vec![], env_remove: vec![] };
    state.pty.spawn(SpawnRequest { key: key.to_owned(), launch, command, scrollback_lines: 100 }).unwrap();
}

fn wait_until(description: &str, condition: impl Fn() -> bool) {
    let started = Instant::now();
    while !condition() {
        assert!(started.elapsed() < Duration::from_secs(10), "timed out waiting for {description}");
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn idle_sessions_hibernate_and_working_ones_are_spared() {
    let sandbox = SandboxHome::with_read_only_claude_home();
    let state = ccm_lib::build_state(sandbox.paths(), Arc::new(|_| {})).unwrap();
    let hibernation = HibernationSettings { enabled: true, idle_minutes: IDLE_MINUTES };
    settings::save(&state.database, AppSettings { hibernation, ..AppSettings::default() }).unwrap();
    let workdir = tempfile::tempdir().unwrap();
    spawn(&state, "idle", workdir.path());
    spawn(&state, "busy", workdir.path());
    let working: HookPayload = serde_json::from_str(r#"{"hook_event_name":"UserPromptSubmit"}"#).unwrap();
    state.status.apply("busy", status_for_hook(&working).unwrap(), now_ms());

    let later = now_ms() + i64::from(IDLE_MINUTES) * 60_000 + 1;
    assert_eq!(hibernate_idle_sessions(&state, now_ms()), 0, "nothing is idle yet");
    assert_eq!(hibernate_idle_sessions(&state, later), 1);
    wait_until("idle session ended", || state.pty.snapshot("idle").unwrap().exited);
    assert!(state.pty.snapshot("idle").unwrap().hibernated);
    assert!(!state.pty.snapshot("busy").unwrap().exited, "a working session never hibernates");
    state.pty.shutdown_all(Duration::from_millis(300));
}

#[test]
fn disabled_hibernation_leaves_everything_running() {
    let sandbox = SandboxHome::with_read_only_claude_home();
    let state = ccm_lib::build_state(sandbox.paths(), Arc::new(|_| {})).unwrap();
    let hibernation = HibernationSettings { enabled: false, idle_minutes: IDLE_MINUTES };
    settings::save(&state.database, AppSettings { hibernation, ..AppSettings::default() }).unwrap();
    let workdir = tempfile::tempdir().unwrap();
    spawn(&state, "idle", workdir.path());
    assert_eq!(hibernate_idle_sessions(&state, now_ms() + 24 * 60 * 60_000), 0);
    state.pty.shutdown_all(Duration::from_millis(300));
}
