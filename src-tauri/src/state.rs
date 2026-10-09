use std::sync::OnceLock;
use std::sync::atomic::AtomicBool;

use crate::db::Database;
use crate::library::service::SharedProgress;
use crate::paths::AppPaths;
use crate::pty::PtyManager;
use crate::shell_env::ShellEnvironment;

pub struct AppState {
    pub paths: AppPaths,
    pub database: Database,
    pub library_progress: SharedProgress,
    pub pty: PtyManager,
    pub shell_environment: OnceLock<ShellEnvironment>,
    pub quit_confirmed: AtomicBool,
}

impl AppState {
    /// The login-shell PATH is read once, on first use (a warm-up thread starts it at launch).
    pub fn shell(&self) -> &ShellEnvironment {
        self.shell_environment.get_or_init(ShellEnvironment::detect)
    }
}
