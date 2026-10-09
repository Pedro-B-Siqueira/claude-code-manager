use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::error::AppError;

const DATABASE_FILE: &str = "ccm.sqlite";

/// Directories the app reads from and writes to. `claude_home` and `~/.claude.json`
/// are read-only by contract: every write goes through [`AppPaths::ensure_writable`].
#[derive(Debug, Clone)]
pub struct AppPaths {
    home: PathBuf,
    claude_home: PathBuf,
    app_support: PathBuf,
}

impl AppPaths {
    pub fn new(home: PathBuf, claude_home: PathBuf, app_support: PathBuf) -> Self {
        Self {
            home,
            claude_home,
            app_support,
        }
    }

    /// `CLAUDE_CONFIG_DIR` mirrors Claude Code's own override; `CCM_APP_SUPPORT_DIR` exists for tests.
    pub fn from_environment() -> Result<Self, AppError> {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or(AppError::MissingHome)?;
        let claude_home = non_empty_var("CLAUDE_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".claude"));
        let app_support = non_empty_var("CCM_APP_SUPPORT_DIR").map(PathBuf::from).unwrap_or_else(|| {
            let xdg_data_home = non_empty_var("XDG_DATA_HOME").map(PathBuf::from);
            crate::platform::default_app_support(&home, crate::platform::CURRENT, xdg_data_home.as_deref())
        });
        Ok(Self::new(home, claude_home, app_support))
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn claude_home(&self) -> &Path {
        &self.claude_home
    }

    pub fn app_support(&self) -> &Path {
        &self.app_support
    }

    pub fn database_file(&self) -> PathBuf {
        self.app_support.join(DATABASE_FILE)
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.app_support.join("logs")
    }

    /// Refuses relative paths and anything that resolves (after `..` and symlinks) into Claude
    /// Code's own files.
    pub fn ensure_writable(&self, target: &Path) -> Result<(), AppError> {
        let forbidden = || Err(AppError::ForbiddenWrite(target.to_path_buf()));
        if !target.is_absolute() {
            return forbidden();
        }
        let resolved = resolve_path(target);
        let claude_home = resolve_path(&self.claude_home);
        let user_config_file = resolve_path(&self.home.join(".claude.json"));
        if resolved.starts_with(&claude_home) || resolved == user_config_file {
            return forbidden();
        }
        Ok(())
    }

    pub fn create_app_support(&self) -> Result<(), AppError> {
        self.ensure_writable(&self.app_support)?;
        fs::create_dir_all(&self.app_support)?;
        Ok(())
    }
}

/// Removes `.` and `..` without touching the filesystem.
fn normalize_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }
    normalized
}

/// Canonical form of a path that may not exist yet: the longest existing ancestor is resolved
/// (following symlinks) and the missing tail is appended.
fn resolve_path(path: &Path) -> PathBuf {
    let normalized = normalize_lexically(path);
    let mut existing = normalized.clone();
    let mut missing = Vec::new();
    while !existing.exists() {
        match existing.file_name() {
            Some(name) => {
                missing.push(name.to_owned());
                existing.pop();
            }
            None => break,
        }
    }
    let mut resolved = existing.canonicalize().unwrap_or(existing);
    for part in missing.iter().rev() {
        resolved.push(part);
    }
    resolved
}

fn non_empty_var(name: &str) -> Option<OsString> {
    env::var_os(name).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_paths() -> AppPaths {
        let home = PathBuf::from("/Users/someone");
        AppPaths::new(
            home.clone(),
            home.join(".claude"),
            home.join("Library/Application Support/ClaudeCodeManager"),
        )
    }

    #[test]
    fn rejects_writes_inside_claude_home() {
        let paths = sample_paths();
        let transcript = paths.claude_home().join("projects/x/session.jsonl");
        assert!(matches!(
            paths.ensure_writable(&transcript),
            Err(AppError::ForbiddenWrite(_))
        ));
    }

    #[test]
    fn rejects_writes_to_user_config_file() {
        let paths = sample_paths();
        let user_config = paths.home().join(".claude.json");
        assert!(paths.ensure_writable(&user_config).is_err());
    }

    #[test]
    fn allows_writes_inside_app_support() {
        let paths = sample_paths();
        assert!(paths.ensure_writable(&paths.database_file()).is_ok());
    }

    #[test]
    fn rejects_relative_paths_and_dot_dot_escapes() {
        let paths = sample_paths();
        assert!(paths.ensure_writable(Path::new("relative/dir")).is_err());
        assert!(paths.ensure_writable(Path::new("/Users/someone/projects/../.claude/wt")).is_err());
        assert!(paths.ensure_writable(Path::new("/Users/someone/projects/./app")).is_ok());
    }

    #[test]
    fn rejects_symlinks_that_point_into_claude_home() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().to_path_buf();
        let claude_home = home.join(".claude");
        std::fs::create_dir_all(&claude_home).unwrap();
        std::os::unix::fs::symlink(&claude_home, home.join("shortcut")).unwrap();
        let paths = AppPaths::new(home.clone(), claude_home, home.join("app"));
        assert!(paths.ensure_writable(&home.join("shortcut/worktrees/x")).is_err());
        assert!(paths.ensure_writable(&home.join("app/ccm.sqlite")).is_ok());
    }
}
