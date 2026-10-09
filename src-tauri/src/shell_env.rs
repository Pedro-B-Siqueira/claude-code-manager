//! GUI apps launched from the Dock do not inherit the shell's PATH. The login shell is asked for it
//! once, so `claude`, `code`, `gh` and `git` resolve exactly as in the user's terminal.

use std::env;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const PATH_START: &str = "__CCM_PATH_START__";
const PATH_END: &str = "__CCM_PATH_END__";
const SHELL_TIMEOUT: Duration = Duration::from_secs(6);
const POLL_INTERVAL: Duration = Duration::from_millis(40);
const DEFAULT_SHELL: &str = "/bin/zsh";
const FALLBACK_DIRECTORIES: &[&str] = &["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin", "/usr/sbin", "/sbin"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellEnvironment {
    pub shell: PathBuf,
    pub path: String,
}

impl ShellEnvironment {
    pub fn detect() -> Self {
        let shell = env::var_os("SHELL").map(PathBuf::from).filter(|shell| shell.is_file()).unwrap_or_else(|| PathBuf::from(DEFAULT_SHELL));
        let path = login_shell_path(&shell).unwrap_or_else(|| {
            log::warn!("não foi possível ler o PATH do shell de login; usando o PATH padrão");
            fallback_path()
        });
        Self { shell, path }
    }

    pub fn find_binary(&self, name: &str) -> Option<PathBuf> {
        env::split_paths(&self.path).map(|directory| directory.join(name)).find(|candidate| is_executable(candidate))
    }
}

fn login_shell_path(shell: &Path) -> Option<String> {
    let script = format!("printf '%s%s%s' '{PATH_START}' \"$PATH\" '{PATH_END}'");
    let mut child = Command::new(shell)
        .args(["-l", "-i", "-c", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() < SHELL_TIMEOUT => std::thread::sleep(POLL_INTERVAL),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let mut output = String::new();
    child.stdout.take()?.read_to_string(&mut output).ok()?;
    extract_marked_path(&output)
}

/// Interactive shells may print banners; only the text between the markers is the PATH.
pub fn extract_marked_path(output: &str) -> Option<String> {
    let start = output.find(PATH_START)? + PATH_START.len();
    let end = output[start..].find(PATH_END)? + start;
    let path = output[start..end].trim();
    (!path.is_empty()).then(|| path.to_owned())
}

fn fallback_path() -> String {
    let mut directories: Vec<PathBuf> = env::var_os("PATH").map(|path| env::split_paths(&path).collect()).unwrap_or_default();
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        directories.push(home.join(".local/bin"));
    }
    directories.extend(FALLBACK_DIRECTORIES.iter().map(PathBuf::from));
    directories.dedup();
    env::join_paths(directories).map(|joined| joined.to_string_lossy().into_owned()).unwrap_or_default()
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata().is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

/// Single-quotes a value for a POSIX shell command line.
pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_path_between_markers_ignoring_banners() {
        let output = format!("Welcome!\n{PATH_START}/a/bin:/b/bin{PATH_END}\nbye");
        assert_eq!(extract_marked_path(&output).as_deref(), Some("/a/bin:/b/bin"));
        assert_eq!(extract_marked_path("no markers"), None);
        assert_eq!(extract_marked_path(&format!("{PATH_START}{PATH_END}")), None);
    }

    #[test]
    fn quotes_single_quotes_safely() {
        assert_eq!(shell_quote("plain"), "'plain'");
        assert_eq!(shell_quote("it's; rm -rf /"), "'it'\\''s; rm -rf /'");
    }

    #[test]
    fn finds_executables_on_the_resolved_path() {
        let directory = tempfile::tempdir().unwrap();
        let binary = directory.path().join("tool");
        std::fs::write(&binary, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&binary, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
        let environment = ShellEnvironment { shell: PathBuf::from("/bin/sh"), path: directory.path().display().to_string() };
        assert_eq!(environment.find_binary("tool"), Some(binary));
        assert_eq!(environment.find_binary("missing"), None);
    }
}
