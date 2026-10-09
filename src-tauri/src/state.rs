use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, OnceLock};

use crate::claudegauge::{self, ClaudeGaugeStatus};
use crate::db::Database;
use crate::hooks::tokens::TokenRegistry;
use crate::library::service::SharedProgress;
use crate::notifications::NotificationCenter;
use crate::paths::AppPaths;
use crate::pty::PtyManager;
use crate::shell_env::ShellEnvironment;
use crate::status::StatusTracker;
use crate::tray::TrayUpdater;

pub struct AppState {
    pub paths: AppPaths,
    pub database: Database,
    pub library_progress: SharedProgress,
    pub pty: PtyManager,
    pub shell_environment: OnceLock<ShellEnvironment>,
    pub quit_confirmed: AtomicBool,
    pub hook_tokens: Arc<TokenRegistry>,
    /// Set once the local hook server is up; without it sessions run without `--settings`.
    pub hook_settings_file: OnceLock<PathBuf>,
    pub status: StatusTracker,
    pub notifications: NotificationCenter,
    pub tray: OnceLock<TrayUpdater>,
}

impl AppState {
    /// The login-shell PATH is read once, on first use (a warm-up thread starts it at launch).
    pub fn shell(&self) -> &ShellEnvironment {
        self.shell_environment.get_or_init(ShellEnvironment::detect)
    }

    pub fn git(&self) -> crate::git::Git {
        crate::git::Git::new(self.shell().find_binary("git").unwrap_or_else(|| PathBuf::from("/usr/bin/git")))
    }

    pub fn claudegauge(&self) -> ClaudeGaugeStatus {
        claudegauge::detect(self.paths.claude_home(), self.paths.home())
    }
}
