use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::AppError;

const APP_SUPPORT_FOLDER: &str = "ClaudeCodeManager";
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
        let app_support = non_empty_var("CCM_APP_SUPPORT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                home.join("Library/Application Support")
                    .join(APP_SUPPORT_FOLDER)
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

    pub fn ensure_writable(&self, target: &Path) -> Result<(), AppError> {
        let user_config_file = self.home.join(".claude.json");
        if target.starts_with(&self.claude_home) || target == user_config_file {
            return Err(AppError::ForbiddenWrite(target.to_path_buf()));
        }
        Ok(())
    }

    pub fn create_app_support(&self) -> Result<(), AppError> {
        self.ensure_writable(&self.app_support)?;
        fs::create_dir_all(&self.app_support)?;
        Ok(())
    }
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
}
