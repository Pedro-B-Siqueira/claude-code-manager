//! The Repouso state machine. Locking engages each step in order and undoes the engaged ones if a
//! later step fails; unlocking asks the platform to authenticate and releases every step in reverse.
//! The state lock is never held while a step runs: steps hop to the main thread, which may need to
//! ask `is_locked` meanwhile.

use std::sync::{Mutex, MutexGuard};

use serde::Serialize;

use crate::error::AppError;

/// Everything Repouso turns on, in the order it turns them on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockStep {
    /// A file saying a lock is in progress; the watchdog acts only while it exists.
    PendingFlag,
    /// Turns the display off if the app dies while locked.
    Watchdog,
    /// Keeps the Mac and the display awake.
    NoSleep,
    /// The main window would give access to the terminals of the sessions.
    HideMainWindow,
    /// Jarvis in front of everything, impossible to close or minimize.
    LockJarvisWindow,
    /// The app menu goes away while locked: its key equivalents (⌘Q, ⌘H, ⌘W, ⌘M) would otherwise
    /// still work with the menu bar hidden, and ⌘Q quits without asking.
    RemoveAppMenu,
    /// Hides Dock and menu bar and blocks app switching, Force Quit, logout and the Apple menu.
    Kiosk,
    /// A lock window on every other display, including ones connected later.
    CoverScreens,
}

pub const LOCK_STEPS: [LockStep; 8] = [
    LockStep::PendingFlag,
    LockStep::Watchdog,
    LockStep::NoSleep,
    LockStep::HideMainWindow,
    LockStep::LockJarvisWindow,
    LockStep::RemoveAppMenu,
    LockStep::Kiosk,
    LockStep::CoverScreens,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockInfo {
    pub since_ms: i64,
}

/// The side effects behind each step; macOS in the app, a recorder in tests.
pub trait LockPlatform: Send + Sync {
    fn engage(&self, step: LockStep, lock: &LockInfo) -> Result<(), AppError>;
    fn release(&self, step: LockStep);
    /// The system authentication prompt must show above the lock windows.
    fn set_prompt_mode(&self, prompting: bool);
    /// Touch ID or the user's password, asked by the system. Blocks until the user answers.
    fn authenticate(&self) -> bool;
}

/// What every window shows (`repouso:changed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LockSnapshot {
    pub active: bool,
    pub since: Option<i64>,
    pub show_details: bool,
    pub authenticating: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlockOutcome {
    Unlocked,
    Refused,
    NotLocked,
    AlreadyAsking,
}

#[derive(Debug, Clone, Copy)]
enum LockState {
    Off { show_details: bool },
    /// Steps are being engaged; already counts as locked for the guards.
    Engaging { since_ms: i64, show_details: bool },
    Locked { since_ms: i64, show_details: bool, authenticating: bool },
}

pub struct Repouso<P: LockPlatform> {
    platform: P,
    state: Mutex<LockState>,
}

impl<P: LockPlatform> Repouso<P> {
    pub fn new(platform: P) -> Self {
        Self { platform, state: Mutex::new(LockState::Off { show_details: true }) }
    }

    fn state(&self) -> MutexGuard<'_, LockState> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn activate(&self, show_details: bool, now_ms: i64) -> Result<LockSnapshot, AppError> {
        {
            let mut state = self.state();
            if !matches!(*state, LockState::Off { .. }) {
                return Ok(snapshot_of(*state));
            }
            *state = LockState::Engaging { since_ms: now_ms, show_details };
        }
        let info = LockInfo { since_ms: now_ms };
        let mut engaged = Vec::with_capacity(LOCK_STEPS.len());
        for step in LOCK_STEPS {
            if let Err(error) = self.platform.engage(step, &info) {
                engaged.iter().rev().for_each(|done| self.platform.release(*done));
                *self.state() = LockState::Off { show_details };
                return Err(error);
            }
            engaged.push(step);
        }
        let mut state = self.state();
        *state = LockState::Locked { since_ms: now_ms, show_details, authenticating: false };
        Ok(snapshot_of(*state))
    }

    pub fn unlock(&self) -> UnlockOutcome {
        {
            let mut state = self.state();
            match *state {
                LockState::Locked { authenticating: true, .. } | LockState::Engaging { .. } => return UnlockOutcome::AlreadyAsking,
                LockState::Off { .. } => return UnlockOutcome::NotLocked,
                LockState::Locked { since_ms, show_details, authenticating: false } => *state = LockState::Locked { since_ms, show_details, authenticating: true },
            }
        }
        self.platform.set_prompt_mode(true);
        let approved = self.platform.authenticate();
        self.platform.set_prompt_mode(false);
        if approved {
            self.release_all();
            return UnlockOutcome::Unlocked;
        }
        let mut state = self.state();
        if let LockState::Locked { since_ms, show_details, .. } = *state {
            *state = LockState::Locked { since_ms, show_details, authenticating: false };
        }
        UnlockOutcome::Refused
    }

    /// Development builds only (CCM_REPOUSO_DEV_TIMEOUT): a bug in the unlock path must not lock the developer out.
    pub fn release_without_authentication(&self) {
        if self.is_locked() {
            self.release_all();
        }
    }

    fn release_all(&self) {
        LOCK_STEPS.iter().rev().for_each(|step| self.platform.release(*step));
        let mut state = self.state();
        let show_details = snapshot_of(*state).show_details;
        *state = LockState::Off { show_details };
    }

    pub fn snapshot(&self) -> LockSnapshot {
        snapshot_of(*self.state())
    }

    pub fn is_locked(&self) -> bool {
        !matches!(*self.state(), LockState::Off { .. })
    }
}

fn snapshot_of(state: LockState) -> LockSnapshot {
    match state {
        LockState::Off { show_details } => LockSnapshot { active: false, since: None, show_details, authenticating: false },
        LockState::Engaging { since_ms, show_details } => LockSnapshot { active: true, since: Some(since_ms), show_details, authenticating: false },
        LockState::Locked { since_ms, show_details, authenticating } => LockSnapshot { active: true, since: Some(since_ms), show_details, authenticating },
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Default)]
    struct FakePlatform {
        calls: Mutex<Vec<String>>,
        failing_step: Option<LockStep>,
        approves: bool,
        seen_while_asking: Mutex<Option<LockSnapshot>>,
        repouso: Mutex<Option<Arc<Repouso<Arc<FakePlatform>>>>>,
    }

    impl FakePlatform {
        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl LockPlatform for Arc<FakePlatform> {
        fn engage(&self, step: LockStep, _lock: &LockInfo) -> Result<(), AppError> {
            if self.failing_step == Some(step) {
                return Err(AppError::Invalid(format!("{step:?} failed")));
            }
            self.calls.lock().unwrap().push(format!("engage {step:?}"));
            Ok(())
        }

        fn release(&self, step: LockStep) {
            self.calls.lock().unwrap().push(format!("release {step:?}"));
        }

        fn set_prompt_mode(&self, prompting: bool) {
            self.calls.lock().unwrap().push(format!("prompt {prompting}"));
        }

        fn authenticate(&self) -> bool {
            self.calls.lock().unwrap().push("authenticate".to_owned());
            if let Some(repouso) = self.repouso.lock().unwrap().as_ref() {
                *self.seen_while_asking.lock().unwrap() = Some(repouso.snapshot());
            }
            self.approves
        }
    }

    fn repouso(platform: FakePlatform) -> (Arc<FakePlatform>, Arc<Repouso<Arc<FakePlatform>>>) {
        let platform = Arc::new(platform);
        let repouso = Arc::new(Repouso::new(Arc::clone(&platform)));
        *platform.repouso.lock().unwrap() = Some(Arc::clone(&repouso));
        (platform, repouso)
    }

    const ENGAGED_IN_ORDER: [&str; 8] = [
        "engage PendingFlag",
        "engage Watchdog",
        "engage NoSleep",
        "engage HideMainWindow",
        "engage LockJarvisWindow",
        "engage RemoveAppMenu",
        "engage Kiosk",
        "engage CoverScreens",
    ];

    #[test]
    fn locking_engages_every_step_in_order() {
        let (platform, repouso) = repouso(FakePlatform::default());
        let snapshot = repouso.activate(true, 1_000).unwrap();
        assert_eq!(platform.calls(), ENGAGED_IN_ORDER);
        assert_eq!(snapshot, LockSnapshot { active: true, since: Some(1_000), show_details: true, authenticating: false });
        assert!(repouso.is_locked());
    }

    #[test]
    fn a_failing_step_undoes_the_ones_before_it_and_leaves_the_mac_unlocked() {
        let (platform, repouso) = repouso(FakePlatform { failing_step: Some(LockStep::Kiosk), ..FakePlatform::default() });
        assert!(repouso.activate(true, 1_000).is_err());
        assert_eq!(
            platform.calls()[6..],
            ["release RemoveAppMenu", "release LockJarvisWindow", "release HideMainWindow", "release NoSleep", "release Watchdog", "release PendingFlag"]
        );
        assert!(!repouso.is_locked());
    }

    #[test]
    fn locking_twice_keeps_the_first_lock() {
        let (platform, repouso) = repouso(FakePlatform::default());
        repouso.activate(false, 1_000).unwrap();
        let again = repouso.activate(true, 2_000).unwrap();
        assert_eq!(platform.calls().len(), 8);
        assert_eq!(again.since, Some(1_000));
        assert!(!again.show_details);
    }

    #[test]
    fn approved_authentication_releases_everything_in_reverse() {
        let (platform, repouso) = repouso(FakePlatform { approves: true, ..FakePlatform::default() });
        repouso.activate(true, 1_000).unwrap();
        assert_eq!(repouso.unlock(), UnlockOutcome::Unlocked);
        assert_eq!(
            platform.calls()[8..],
            [
                "prompt true",
                "authenticate",
                "prompt false",
                "release CoverScreens",
                "release Kiosk",
                "release RemoveAppMenu",
                "release LockJarvisWindow",
                "release HideMainWindow",
                "release NoSleep",
                "release Watchdog",
                "release PendingFlag",
            ]
        );
        assert_eq!(repouso.snapshot(), LockSnapshot { active: false, since: None, show_details: true, authenticating: false });
    }

    #[test]
    fn refused_or_cancelled_authentication_keeps_the_lock() {
        let (platform, repouso) = repouso(FakePlatform { approves: false, ..FakePlatform::default() });
        repouso.activate(true, 1_000).unwrap();
        assert_eq!(repouso.unlock(), UnlockOutcome::Refused);
        assert_eq!(platform.calls()[8..], ["prompt true", "authenticate", "prompt false"]);
        assert!(repouso.is_locked());
        assert!(!repouso.snapshot().authenticating);
    }

    #[test]
    fn the_screens_know_while_the_system_is_asking() {
        let (platform, repouso) = repouso(FakePlatform::default());
        repouso.activate(true, 1_000).unwrap();
        repouso.unlock();
        assert!(platform.seen_while_asking.lock().unwrap().is_some_and(|snapshot| snapshot.authenticating));
    }

    #[test]
    fn unlocking_what_is_not_locked_asks_nothing() {
        let (platform, repouso) = repouso(FakePlatform::default());
        assert_eq!(repouso.unlock(), UnlockOutcome::NotLocked);
        assert!(platform.calls().is_empty());
    }

    #[test]
    fn the_development_timeout_releases_without_asking() {
        let (platform, repouso) = repouso(FakePlatform::default());
        repouso.activate(true, 1_000).unwrap();
        repouso.release_without_authentication();
        assert!(!repouso.is_locked());
        assert!(!platform.calls().contains(&"authenticate".to_owned()));
        assert_eq!(platform.calls().last().map(String::as_str), Some("release PendingFlag"));
    }
}
