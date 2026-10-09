//! Golden rule 1: the app must work with `~/.claude` read-only and never attempt to write there.

mod support;

use ccm_lib::db::Database;
use ccm_lib::library::{indexer::Indexer, queries};
use ccm_lib::settings::{self, AppSettings, Theme};
use support::SandboxHome;

#[test]
fn app_state_and_settings_never_touch_claude_home() {
    let sandbox = SandboxHome::with_read_only_claude_home();
    let before = sandbox.claude_fingerprint();

    let state =
        ccm_lib::build_state(sandbox.paths(), std::sync::Arc::new(|_| {})).expect("state builds with read-only ~/.claude");
    let light = AppSettings {
        theme: Theme::Light,
        ..AppSettings::default()
    };
    settings::save(&state.database, light.clone()).expect("settings saved in app support");

    assert_eq!(settings::load(&state.database).expect("load"), light);
    assert!(sandbox.app_support.join("ccm.sqlite").exists());
    assert_eq!(
        sandbox.claude_fingerprint(),
        before,
        "~/.claude must stay byte-for-byte identical"
    );
}

#[test]
fn app_support_inside_claude_home_is_refused() {
    let sandbox = SandboxHome::with_read_only_claude_home();
    let hostile = ccm_lib::paths::AppPaths::new(
        sandbox.home.clone(),
        sandbox.claude_home.clone(),
        sandbox.claude_home.join("ccm"),
    );
    assert!(ccm_lib::build_state(hostile, std::sync::Arc::new(|_| {})).is_err());
}

#[test]
fn indexing_transcripts_never_touches_claude_home() {
    let sandbox = SandboxHome::with_read_only_claude_home();
    let before = sandbox.claude_fingerprint();

    let state = ccm_lib::build_state(sandbox.paths(), std::sync::Arc::new(|_| {})).expect("state");
    let indexer = Indexer::new(
        Database::open(&state.paths.database_file()).expect("indexer connection"),
        sandbox.claude_home.join("projects"),
    );
    let discovered = indexer.discover();
    assert_eq!(discovered.len(), 2, "main transcript and subagent are discovered");
    for transcript in &discovered {
        indexer.index_file(transcript).expect("indexing works on a read-only ~/.claude");
    }

    assert_eq!(queries::list_sessions(&state.database.connection()).expect("list").len(), 1);
    assert_eq!(sandbox.claude_fingerprint(), before, "~/.claude must stay byte-for-byte identical");
}

#[test]
fn hooks_registry_and_claudegauge_detection_stay_read_only() {
    let sandbox = SandboxHome::with_read_only_claude_home();
    let before = sandbox.claude_fingerprint();

    let state = ccm_lib::build_state(sandbox.paths(), std::sync::Arc::new(|_| {})).expect("state");
    let settings_file = ccm_lib::hooks::settings_file::write_settings(&state.paths, 4242).expect("hook settings written");
    assert!(settings_file.starts_with(&sandbox.app_support), "hook settings live in the app folder");
    assert!(ccm_lib::claudegauge::detect(&sandbox.claude_home, &sandbox.home).hook_installed);
    assert_eq!(ccm_lib::live::registry::read_registry(&sandbox.claude_home).len(), 1);
    let refused = ccm_lib::hooks::settings_file::write_settings(
        &ccm_lib::paths::AppPaths::new(sandbox.home.clone(), sandbox.claude_home.clone(), sandbox.claude_home.join("app")),
        1,
    );
    assert!(refused.is_err(), "hook settings can never be written inside ~/.claude");

    assert_eq!(sandbox.claude_fingerprint(), before, "~/.claude must stay byte-for-byte identical");
}

#[test]
fn app_support_reached_through_a_symlink_into_claude_home_is_refused() {
    let sandbox = SandboxHome::with_read_only_claude_home();
    let before = sandbox.claude_fingerprint();
    std::os::unix::fs::symlink(&sandbox.claude_home, sandbox.home.join("link")).unwrap();
    let sneaky = ccm_lib::paths::AppPaths::new(sandbox.home.clone(), sandbox.claude_home.clone(), sandbox.home.join("link/ccm"));
    assert!(ccm_lib::build_state(sneaky, std::sync::Arc::new(|_| {})).is_err());
    let dotted = ccm_lib::paths::AppPaths::new(sandbox.home.clone(), sandbox.claude_home.clone(), sandbox.home.join("x/../.claude/ccm"));
    assert!(ccm_lib::build_state(dotted, std::sync::Arc::new(|_| {})).is_err());
    assert_eq!(sandbox.claude_fingerprint(), before);
}
