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

pub fn write_settings(paths: &AppPaths, port: u16) -> Result<PathBuf, AppError> {
    let directory = paths.app_support().join("hooks");
    let file = directory.join("claude-settings.json");
    paths.ensure_writable(&file)?;
    fs::create_dir_all(&directory)?;
    fs::write(&file, serde_json::to_vec_pretty(&hook_settings(port))?)?;
    Ok(file)
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
    fn settings_reference_tokens_only_through_env_vars() {
        let text = hook_settings(1).to_string();
        let header_values: Vec<&str> = text.match_indices("X-CCM-Token\":\"").map(|(index, marker)| &text[index + marker.len()..]).collect();
        assert!(header_values.iter().all(|rest| rest.starts_with("$CCM_HOOK_TOKEN")));
    }
}
