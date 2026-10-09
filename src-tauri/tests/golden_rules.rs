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
