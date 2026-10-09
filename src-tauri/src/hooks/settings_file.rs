//! The settings file passed to `claude --settings`: HTTP hooks pointing at the local server. It
//! holds no secret; each session's token reaches Claude Code through an environment variable.

use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};

use super::server::{HOOK_PATH, SESSION_HEADER, TOKEN_HEADER};
use crate::error::AppError;
use crate::paths::AppPaths;

pub const TOKEN_ENV: &str = "CCM_HOOK_TOKEN";
pub const SESSION_ENV: &str = "CCM_SESSION_KEY";
const HOOK_TIMEOUT_SECONDS: u64 = 5;
const TOOL_EVENTS: &[&str] = &["PreToolUse", "PostToolUse", "PermissionRequest"];
const SESSION_EVENTS: &[&str] = &["SessionStart", "UserPromptSubmit", "Notification", "Stop", "StopFailure", "SessionEnd"];

fn http_hook(port: u16) -> Value {
    json!({
        "type": "http",
        "url": format!("http://127.0.0.1:{port}{HOOK_PATH}"),
        "timeout": HOOK_TIMEOUT_SECONDS,
        "headers": {
            TOKEN_HEADER: format!("${TOKEN_ENV}"),
            SESSION_HEADER: format!("${SESSION_ENV}"),
        },
        "allowedEnvVars": [TOKEN_ENV, SESSION_ENV],
    })
}

pub fn hook_settings(port: u16) -> Value {
    let mut hooks = serde_json::Map::new();
    for event in TOOL_EVENTS {
        hooks.insert((*event).to_owned(), json!([{ "matcher": "*", "hooks": [http_hook(port)] }]));
    }
    for event in SESSION_EVENTS {
        hooks.insert((*event).to_owned(), json!([{ "hooks": [http_hook(port)] }]));
    }
    json!({ "hooks": hooks })
}

const FILE_PREFIX: &str = "claude-settings-";

/// One file per app process (named after its pid), written atomically, so a second running copy
/// of the app (dev and release side by side) never redirects the other's sessions.
pub fn write_settings(paths: &AppPaths, port: u16) -> Result<PathBuf, AppError> {
    let directory = paths.app_support().join("hooks");
    let file = directory.join(format!("{FILE_PREFIX}{}.json", std::process::id()));
    let temporary = file.with_extension("json.tmp");
    paths.ensure_writable(&file)?;
    fs::create_dir_all(&directory)?;
    remove_stale_files(&directory);
    fs::write(&temporary, serde_json::to_vec_pretty(&hook_settings(port))?)?;
    fs::rename(&temporary, &file)?;
    Ok(file)
}

/// Files left by app processes that are no longer running.
fn remove_stale_files(directory: &std::path::Path) {
    let Ok(entries) = fs::read_dir(directory) else { return };
    for path in entries.flatten().map(|entry| entry.path()) {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else { continue };
        let owner = name.strip_prefix(FILE_PREFIX).and_then(|rest| rest.split('.').next()).and_then(|pid| pid.parse::<u32>().ok());
        let stale = match owner {
            Some(pid) => pid != std::process::id() && !crate::live::registry::process_alive(pid),
            None => name == "claude-settings.json",
        };
        if stale {
            let _ = fs::remove_file(&path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_event_points_at_the_local_server_with_templated_headers() {
        let settings = hook_settings(4321);
        for event in TOOL_EVENTS.iter().chain(SESSION_EVENTS) {
            let hook = &settings["hooks"][event][0]["hooks"][0];
            assert_eq!(hook["type"], "http", "{event}");
            assert_eq!(hook["url"], "http://127.0.0.1:4321/hook");
            assert_eq!(hook["headers"]["X-CCM-Token"], "$CCM_HOOK_TOKEN");
            assert_eq!(hook["allowedEnvVars"], json!(["CCM_HOOK_TOKEN", "CCM_SESSION_KEY"]));
        }
        assert_eq!(settings["hooks"]["PreToolUse"][0]["matcher"], "*");
        assert!(settings["hooks"]["Stop"][0].get("matcher").is_none());
    }

    #[test]
    fn written_file_never_contains_a_session_token() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().to_path_buf();
        let paths = AppPaths::new(home.clone(), home.join(".claude"), home.join("app"));
        let token = crate::hooks::tokens::TokenRegistry::default().issue("session");
        let file = write_settings(&paths, 4321).unwrap();
        let text = fs::read_to_string(&file).unwrap();
        assert!(!text.contains(&token));
        assert!(text.contains("$CCM_HOOK_TOKEN"));
        assert!(file.file_name().unwrap().to_string_lossy().contains(&std::process::id().to_string()));
    }

    #[test]
    fn stale_files_from_dead_instances_are_removed_and_live_ones_kept() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().to_path_buf();
        let paths = AppPaths::new(home.clone(), home.join(".claude"), home.join("app"));
        let directory = paths.app_support().join("hooks");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("claude-settings-999999.json"), "{}").unwrap();
        fs::write(directory.join("claude-settings.json"), "{}").unwrap();
        let mine = write_settings(&paths, 1).unwrap();
        let again = write_settings(&paths, 2).unwrap();
        assert_eq!(mine, again);
        let names: Vec<String> = fs::read_dir(&directory).unwrap().flatten().map(|entry| entry.file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(names, vec![mine.file_name().unwrap().to_string_lossy().into_owned()]);
        assert!(fs::read_to_string(&again).unwrap().contains("127.0.0.1:2"));
    }
}
