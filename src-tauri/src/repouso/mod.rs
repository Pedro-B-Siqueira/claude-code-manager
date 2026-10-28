//! Repouso: locks the Mac in the Jarvis screen until Touch ID or the user's password unlocks it.

pub mod lock_delay;
pub mod lock;
pub mod pending;
pub mod preferences;
#[cfg(target_os = "macos")]
pub mod native;
pub mod platform;

/// The Repouso of the running app.
pub type AppRepouso = lock::Repouso<platform::AppLockPlatform>;
pub mod helpers;
