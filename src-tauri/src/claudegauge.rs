//! Detects ClaudeGauge by reading (never writing) the user's Claude Code settings, so the app can
//! avoid duplicate notifications. Works with or without ClaudeGauge installed.

use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

/// The script ClaudeGauge's installer registers as a `Notification` hook.
const HOOK_MARKER: &str = "claude-notify.sh";
const APP_LOCATIONS: &[&str] = &["/Applications/ClaudeGauge.app"];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeGaugeStatus {
    pub app_installed: bool,
    pub hook_installed: bool,
}

pub fn detect(claude_home: &Path, home: &Path) -> ClaudeGaugeStatus {
    let app_installed = APP_LOCATIONS.iter().any(|location| Path::new(location).exists())
        || home.join("Applications/ClaudeGauge.app").exists();
    ClaudeGaugeStatus { app_installed, hook_installed: has_hook(&claude_home.join("settings.json")) }
}

pub fn has_hook(settings_file: &Path) -> bool {
    let Ok(text) = fs::read_to_string(settings_file) else { return false };
    let Ok(settings) = serde_json::from_str::<Value>(&text) else { return false };
    let Some(events) = settings.get("hooks").and_then(Value::as_object) else { return false };
    events
        .values()
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(|group| group.get("hooks").and_then(Value::as_array))
        .flatten()
        .filter_map(|hook| hook.get("command").and_then(Value::as_str))
        .any(|command| command.contains(HOOK_MARKER))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_with(json: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("settings.json");
        fs::write(&file, json).unwrap();
        (directory, file)
    }

    #[test]
    fn finds_the_claudegauge_notification_hook() {
        let (_directory, file) = settings_with(
            r#"{"hooks":{"Notification":[{"matcher":null,"hooks":[{"type":"command","command":"\"/Applications/ClaudeGauge.app/Contents/Resources/claude-notify.sh\" attention"}]}],
                "PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"/x/other.sh"}]}]}}"#,
        );
        assert!(has_hook(&file));
    }

    #[test]
    fn other_hooks_missing_or_broken_files_are_not_claudegauge() {
        let (_directory, file) = settings_with(r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"say done"}]}]}}"#);
        assert!(!has_hook(&file));
        let (_broken_dir, broken) = settings_with("{not json");
        assert!(!has_hook(&broken));
        assert!(!has_hook(Path::new("/nonexistent/settings.json")));
    }
}
