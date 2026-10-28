//! How long macOS waits before asking for the password once the screen goes dark. The watchdog
//! turns the display off when the app dies during Repouso; only "immediately" makes that a real lock.

use std::process::Command;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "seconds", rename_all = "lowercase")]
pub enum LockDelay {
    Immediate,
    Seconds(u32),
    Off,
    Unknown,
}

impl LockDelay {
    pub fn locks_at_once(self) -> bool {
        self == Self::Immediate
    }
}

pub fn parse_lock_delay(output: &str) -> LockDelay {
    if output.contains("screenLock is off") {
        return LockDelay::Off;
    }
    let Some(delay) = output.split("screenLock delay is ").nth(1) else { return LockDelay::Unknown };
    if delay.trim_start().starts_with("immediate") {
        return LockDelay::Immediate;
    }
    delay.split_whitespace().next().and_then(|seconds| seconds.parse().ok()).map_or(LockDelay::Unknown, LockDelay::Seconds)
}

/// Reads the setting without changing it; `sysadminctl` prints it on stderr and needs no admin rights.
pub fn read_lock_delay() -> LockDelay {
    match Command::new("/usr/sbin/sysadminctl").args(["-screenLock", "status"]).output() {
        Ok(output) => parse_lock_delay(&format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr))),
        Err(error) => {
            log::warn!("não foi possível ler o atraso da senha da tela: {error}");
            LockDelay::Unknown
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_delays_sysadminctl_reports() {
        assert_eq!(parse_lock_delay("2026-10-27 16:00:51.345 sysadminctl[90368:747992] screenLock delay is 60 seconds"), LockDelay::Seconds(60));
        assert_eq!(parse_lock_delay("2026-10-27 16:00:51.345 sysadminctl[1:2] screenLock delay is immediate"), LockDelay::Immediate);
        assert_eq!(parse_lock_delay("sysadminctl[1:2] screenLock is off"), LockDelay::Off);
        assert_eq!(parse_lock_delay("something else entirely"), LockDelay::Unknown);
        assert_eq!(parse_lock_delay(""), LockDelay::Unknown);
    }

    #[test]
    fn only_an_immediate_lock_is_safe_for_the_watchdog() {
        assert!(LockDelay::Immediate.locks_at_once());
        assert!(!LockDelay::Seconds(5).locks_at_once());
        assert!(!LockDelay::Off.locks_at_once());
        assert!(!LockDelay::Unknown.locks_at_once());
    }

    #[test]
    fn serializes_for_the_activation_dialog() {
        assert_eq!(serde_json::to_value(LockDelay::Seconds(60)).unwrap(), serde_json::json!({ "kind": "seconds", "seconds": 60 }));
        assert_eq!(serde_json::to_value(LockDelay::Immediate).unwrap(), serde_json::json!({ "kind": "immediate" }));
    }
}
