use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, OnceLock};

use crate::attachments::AttachmentStore;
use crate::claudegauge::{self, ClaudeGaugeStatus};
use crate::db::Database;
use crate::hooks::tokens::TokenRegistry;
use crate::library::service::SharedProgress;
use crate::notifications::NotificationCenter;
use crate::paths::AppPaths;
use crate::pty::PtyManager;
use crate::repouso::AppRepouso;
use crate::sessions::action::ActionTracker;
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
    /// What each session opened here is doing now, from the same hook events.
    pub actions: ActionTracker,
    pub notifications: NotificationCenter,
    pub tray: OnceLock<TrayUpdater>,
    pub attachments: AttachmentStore,
    /// Set up once the app handle exists (the platform needs it to manage windows).
    pub repouso: OnceLock<Arc<AppRepouso>>,
}

impl AppState {
    /// The login-shell PATH is read once, on first use (a warm-up thread starts it at launch).
    pub fn shell(&self) -> &ShellEnvironment {
        self.shell_environment.get_or_init(ShellEnvironment::detect)
    }

    pub fn git(&self) -> crate::git::Git {
        crate::git::Git::new(self.shell().find_binary("git").unwrap_or_else(|| PathBuf::from("/usr/bin/git")))
    }

    /// While Repouso is on, nothing may close, quit, hide Jarvis or show the main window.
    pub fn is_locked(&self) -> bool {
        self.repouso.get().is_some_and(|repouso| repouso.is_locked())
    }

    pub fn claudegauge(&self) -> ClaudeGaugeStatus {
        claudegauge::detect(self.paths.claude_home(), self.paths.home())
    }
}
