//! Golden rule 1: the app must work with `~/.claude` read-only and never attempt to write there.

mod support;

use ccm_lib::settings::{self, AppSettings, Theme};
use support::SandboxHome;

#[test]
fn app_state_and_settings_never_touch_claude_home() {
    let sandbox = SandboxHome::with_read_only_claude_home();
    let before = sandbox.claude_fingerprint();

    let state =
        ccm_lib::build_state(sandbox.paths()).expect("state builds with read-only ~/.claude");
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
    assert!(ccm_lib::build_state(hostile).is_err());
}
