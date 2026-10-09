//! Builds the command an embedded terminal runs: `claude` through the user's login shell, falling
//! back to that shell when `claude` exits so the terminal stays usable.

use std::env;
use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::pty::{CommandLine, LaunchMode, LaunchSpec};
use crate::shell_env::{ShellEnvironment, shell_quote};

/// Points tests (and anyone debugging) at a stand-in binary instead of the real `claude`.
pub const CLAUDE_BINARY_OVERRIDE: &str = "CCM_CLAUDE_BIN";

/// Identity of whatever terminal launched the app. Inheriting `TERM_SESSION_ID` would make zsh
/// restore and overwrite Terminal.app's saved session history inside the embedded terminals.
const INHERITED_TERMINAL_VARS: &[&str] = &[
    "TERM_SESSION_ID",
    "TERM_PROGRAM",
    "TERM_PROGRAM_VERSION",
    "ITERM_SESSION_ID",
    "ITERM_PROFILE",
    "VSCODE_INJECTION",
    "VSCODE_GIT_IPC_HANDLE",
    "TMUX",
    "TMUX_PANE",
    "CLAUDECODE",
    "CLAUDE_CODE_ENTRYPOINT",
];

pub fn resolve_claude(configured: Option<&str>, shell: &ShellEnvironment, home: &Path) -> Result<PathBuf, AppError> {
    if let Some(override_path) = env::var_os(CLAUDE_BINARY_OVERRIDE).filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(override_path));
    }
    if let Some(configured) = configured.map(str::trim).filter(|value| !value.is_empty()) {
        return Ok(expand_home(configured, home));
    }
    shell.find_binary("claude").ok_or(AppError::ClaudeNotFound)
}

pub fn expand_home(path: &str, home: &Path) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None if path == "~" => home.to_path_buf(),
        None => PathBuf::from(path),
    }
}

pub fn claude_arguments(launch: &LaunchSpec, settings_file: Option<&Path>) -> Vec<String> {
    let mut arguments = match launch.mode {
        LaunchMode::New => vec!["--session-id".to_owned(), launch.session_id.clone()],
        LaunchMode::Resume => vec!["--resume".to_owned(), launch.session_id.clone()],
    };
    if let Some(settings_file) = settings_file {
        arguments.push("--settings".to_owned());
        arguments.push(settings_file.to_string_lossy().into_owned());
    }
    arguments
}

pub fn login_shell_command(shell: &ShellEnvironment, claude: &Path, arguments: &[String], extra_env: Vec<(String, String)>) -> CommandLine {
    let quoted_shell = shell_quote(&shell.shell.to_string_lossy());
    let quoted_arguments: Vec<String> = arguments.iter().map(|argument| shell_quote(argument)).collect();
    let script = format!(
        "{} {}; exec {quoted_shell} -l -i",
        shell_quote(&claude.to_string_lossy()),
        quoted_arguments.join(" ")
    );
    let mut env = terminal_environment(&shell.path);
    env.extend(extra_env);
    CommandLine {
        program: shell.shell.clone(),
        args: vec!["-l".to_owned(), "-i".to_owned(), "-c".to_owned(), script],
        env,
        env_remove: INHERITED_TERMINAL_VARS.iter().map(|name| (*name).to_owned()).collect(),
    }
}

/// Env vars every embedded terminal gets on top of the app's own environment.
pub fn terminal_environment(path: &str) -> Vec<(String, String)> {
    let mut env = vec![
        ("TERM".to_owned(), "xterm-256color".to_owned()),
        ("COLORTERM".to_owned(), "truecolor".to_owned()),
        ("PATH".to_owned(), path.to_owned()),
    ];
    if env::var_os("LANG").is_none() {
        env.push(("LANG".to_owned(), "en_US.UTF-8".to_owned()));
    }
    env
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launch(mode: LaunchMode) -> LaunchSpec {
        LaunchSpec { session_id: "abc".to_owned(), cwd: PathBuf::from("/repo"), mode, worktree: None }
    }

    #[test]
    fn new_sessions_get_a_fixed_id_and_resumed_ones_use_resume() {
        assert_eq!(claude_arguments(&launch(LaunchMode::New), None), vec!["--session-id", "abc"]);
        let resumed = claude_arguments(&launch(LaunchMode::Resume), Some(Path::new("/app/hooks.json")));
        assert_eq!(resumed, vec!["--resume", "abc", "--settings", "/app/hooks.json"]);
    }

    #[test]
    fn script_runs_claude_then_keeps_the_shell_open() {
        let shell = ShellEnvironment { shell: PathBuf::from("/bin/zsh"), path: "/usr/bin".to_owned() };
        let command = login_shell_command(&shell, Path::new("/opt/claude bin/claude"), &["--resume".to_owned(), "x'y".to_owned()], Vec::new());
        assert_eq!(command.program, PathBuf::from("/bin/zsh"));
        assert_eq!(&command.args[..3], ["-l", "-i", "-c"]);
        assert_eq!(command.args[3], "'/opt/claude bin/claude' '--resume' 'x'\\''y'; exec '/bin/zsh' -l -i");
        assert!(command.env.contains(&("PATH".to_owned(), "/usr/bin".to_owned())));
        assert!(command.env_remove.contains(&"TERM_SESSION_ID".to_owned()));
    }

    #[test]
    fn configured_binary_expands_home() {
        let shell = ShellEnvironment { shell: PathBuf::from("/bin/zsh"), path: String::new() };
        if env::var_os(CLAUDE_BINARY_OVERRIDE).is_some() {
            return;
        }
        let resolved = resolve_claude(Some("~/bin/claude"), &shell, Path::new("/Users/someone")).unwrap();
        assert_eq!(resolved, PathBuf::from("/Users/someone/bin/claude"));
        assert!(matches!(resolve_claude(None, &shell, Path::new("/h")), Err(AppError::ClaudeNotFound)));
    }
}
