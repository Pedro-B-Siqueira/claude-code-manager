//! The pending-lock file and the watchdog. While Repouso is on, `repouso.json` in the app data folder
//! says so. A small `/bin/sh` process, outside the app's process group, waits for the app to die;
//! if the file is still there then (a crash, not an unlock), it turns the display off so macOS asks
//! for the password. On the next launch the app removes a file left behind.

use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::repouso::helpers::WATCHDOG_NAME;

pub const PENDING_FILE: &str = "repouso.json";

/// Turns the display off; with "require password immediately" that is a real lock.
pub const DISPLAY_OFF: (&str, &str) = ("/usr/bin/pmset", "displaysleepnow");

/// Polls twice a second: `kill -0` cannot wait on a process that is not its child.
const WATCHDOG_SCRIPT: &str = r#"while kill -0 "$1" 2>/dev/null; do sleep 0.5; done; [ -f "$2" ] && "$3" "$4""#;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingLock {
    pub app_pid: u32,
    pub since: i64,
    pub watchdog_pid: Option<u32>,
}

pub fn write_pending(app_support: &Path, pending: &PendingLock) -> Result<(), AppError> {
    std::fs::write(app_support.join(PENDING_FILE), serde_json::to_vec(pending)?)?;
    Ok(())
}

pub fn read_pending(app_support: &Path) -> Option<PendingLock> {
    let bytes = std::fs::read(app_support.join(PENDING_FILE)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn remove_pending(app_support: &Path) {
    if let Err(error) = std::fs::remove_file(app_support.join(PENDING_FILE)) {
        if error.kind() != std::io::ErrorKind::NotFound {
            log::warn!("não foi possível apagar {PENDING_FILE}: {error}");
        }
    }
}

/// `sh` in its own process group, so killing the app's group does not take the watchdog with it.
pub fn watchdog_command(app_pid: u32, flag: &Path, action: (&str, &str)) -> Command {
    let mut command = Command::new("/bin/sh");
    command
        .args(["-c", WATCHDOG_SCRIPT, WATCHDOG_NAME, &app_pid.to_string(), &flag.to_string_lossy(), action.0, action.1])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    command
}

/// A file left by a crash: the system already undid presentation options and power assertions with the
/// process; only the file and, if still running, its watchdog remain.
pub fn clean_up_after_crash(app_support: &Path, is_watchdog: impl Fn(u32) -> bool, stop: impl FnMut(u32)) -> Option<PendingLock> {
    let pending = read_pending(app_support)?;
    pending.watchdog_pid.filter(|pid| is_watchdog(*pid)).into_iter().for_each(stop);
    remove_pending(app_support);
    Some(pending)
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::time::{Duration, Instant};

    use super::*;

    fn wait_for(condition: impl Fn() -> bool, limit: Duration) -> bool {
        let started = Instant::now();
        while started.elapsed() < limit {
            if condition() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        condition()
    }

    #[test]
    fn the_pending_file_round_trips_and_goes_away() {
        let folder = tempfile::tempdir().unwrap();
        assert_eq!(read_pending(folder.path()), None);
        let pending = PendingLock { app_pid: 42, since: 1_000, watchdog_pid: Some(43) };
        write_pending(folder.path(), &pending).unwrap();
        assert_eq!(read_pending(folder.path()), Some(pending));
        remove_pending(folder.path());
        assert_eq!(read_pending(folder.path()), None);
    }

    #[test]
    fn the_watchdog_acts_when_the_app_dies_with_the_lock_pending() {
        let folder = tempfile::tempdir().unwrap();
        let flag = folder.path().join(PENDING_FILE);
        std::fs::write(&flag, "{}").unwrap();
        let marker = folder.path().join("display-off");
        let mut app = Command::new("/bin/sleep").arg("0.3").spawn().unwrap();
        let mut watchdog = watchdog_command(app.id(), &flag, ("/usr/bin/touch", &marker.to_string_lossy())).spawn().unwrap();
        app.wait().unwrap();
        assert!(wait_for(|| marker.exists(), Duration::from_secs(3)), "the watchdog should turn the display off");
        watchdog.wait().unwrap();
    }

    #[test]
    fn the_watchdog_stays_quiet_after_a_normal_unlock() {
        let folder = tempfile::tempdir().unwrap();
        let flag = folder.path().join(PENDING_FILE);
        let marker = folder.path().join("display-off");
        let mut app = Command::new("/bin/sleep").arg("0.3").spawn().unwrap();
        let mut watchdog = watchdog_command(app.id(), &flag, ("/usr/bin/touch", &marker.to_string_lossy())).spawn().unwrap();
        app.wait().unwrap();
        watchdog.wait().unwrap();
        assert!(!marker.exists(), "without the pending file the watchdog must do nothing");
    }

    #[test]
    fn a_launch_after_a_crash_removes_the_leftover_and_stops_its_watchdog() {
        let folder = tempfile::tempdir().unwrap();
        write_pending(folder.path(), &PendingLock { app_pid: 1, since: 5, watchdog_pid: Some(77) }).unwrap();
        let mut stopped = Vec::new();
        let leftover = clean_up_after_crash(folder.path(), |pid| pid == 77, |pid| stopped.push(pid));
        assert_eq!(leftover.map(|pending| pending.since), Some(5));
        assert_eq!(stopped, vec![77]);
        assert_eq!(read_pending(folder.path()), None);
        assert_eq!(clean_up_after_crash(folder.path(), |_| true, |_| panic!("nothing to stop")), None);
    }
}
