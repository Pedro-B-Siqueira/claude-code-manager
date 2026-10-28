//! Notices external sessions starting, changing status or ending: FSEvents on Claude Code's session
//! registry (read-only) plus a slow liveness check for processes that died without cleanup.

use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use notify_debouncer_mini::new_debouncer;
use notify_debouncer_mini::notify::RecursiveMode;

use super::registry::read_registry;

const DEBOUNCE: Duration = Duration::from_millis(300);
const LIVENESS_INTERVAL: Duration = Duration::from_secs(10);

pub type ChangeNotifier = Arc<dyn Fn() + Send + Sync>;

fn fingerprint(claude_home: &Path) -> Vec<(u32, String, i64)> {
    read_registry(claude_home)
        .into_iter()
        .map(|session| (session.pid, format!("{:?}", session.status), session.status_updated_at))
        .collect()
}

/// Without a file watcher (the registry folder does not exist yet) the channel is disconnected and
/// would return at once; sleeping keeps the liveness check at its normal pace instead of spinning.
fn wait_for_change<T>(receiver: &mpsc::Receiver<T>, interval: Duration) {
    if let Err(mpsc::RecvTimeoutError::Disconnected) = receiver.recv_timeout(interval) {
        thread::sleep(interval);
    }
}

pub fn start(claude_home: &Path, notify: ChangeNotifier) {
    let claude_home = claude_home.to_path_buf();
    let spawned = thread::Builder::new().name("registry-watcher".to_owned()).spawn(move || {
        let (sender, receiver) = mpsc::channel();
        let registry = claude_home.join("sessions");
        let _debouncer = new_debouncer(DEBOUNCE, sender).ok().and_then(|mut debouncer| {
            debouncer.watcher().watch(&registry, RecursiveMode::NonRecursive).ok()?;
            Some(debouncer)
        });
        let mut last = fingerprint(&claude_home);
        loop {
            wait_for_change(&receiver, LIVENESS_INTERVAL);
            let current = fingerprint(&claude_home);
            if current != last {
                last = current;
                notify();
            }
        }
    });
    if let Err(error) = spawned {
        log::warn!("falha ao observar sessões externas: {error}");
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    #[test]
    fn waits_the_whole_interval_when_nothing_can_signal_a_change() {
        let (sender, receiver) = mpsc::channel::<()>();
        drop(sender);
        let started = Instant::now();
        wait_for_change(&receiver, Duration::from_millis(60));
        assert!(started.elapsed() >= Duration::from_millis(60), "a folder that cannot be watched must not turn into a busy loop");
    }

    #[test]
    fn returns_as_soon_as_a_change_arrives() {
        let (sender, receiver) = mpsc::channel::<()>();
        sender.send(()).unwrap();
        let started = Instant::now();
        wait_for_change(&receiver, Duration::from_secs(5));
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
