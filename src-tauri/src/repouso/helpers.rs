//! Process checks for cleaning up after a crash during Repouso.

use std::process::Command;

/// The watchdog's `sh` carries this name as its `$0`, so a reused pid is not mistaken for it.
pub const WATCHDOG_NAME: &str = "ccm-repouso-watchdog";

pub fn is_watchdog(pid: u32) -> bool {
    Command::new("/bin/ps")
        .args(["-o", "args=", "-p", &pid.to_string()])
        .output()
        .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains(WATCHDOG_NAME))
}

pub fn stop_process(pid: u32) {
    let Ok(pid) = i32::try_from(pid) else { return };
    // SAFETY: plain kill(2) on a pid just checked to be our own watchdog.
    unsafe {
        libc::kill(pid, libc::SIGTERM);
    }
}
