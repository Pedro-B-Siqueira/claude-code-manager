use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use portable_pty::{Child, MasterPty, PtySize};
use serde::Serialize;

use super::preview::TerminalScreen;
use super::ring::OutputRing;
use super::{OutputSink, PtyEvent, PtyNotifier};

const READ_CHUNK_BYTES: usize = 32 * 1024;

static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LaunchMode {
    New,
    Resume,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchSpec {
    pub session_id: String,
    pub cwd: PathBuf,
    pub mode: LaunchMode,
    pub worktree: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub key: String,
    pub instance: u64,
    pub launch: LaunchSpec,
    pub started_at: i64,
    pub last_output_at: i64,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
    pub exited: bool,
    pub hibernated: bool,
    pub preview_lines: Vec<String>,
}

struct OutputState {
    ring: OutputRing,
    screen: TerminalScreen,
    subscriber: Option<Box<dyn OutputSink>>,
    preview_dirty: bool,
    last_output_at: i64,
}

#[derive(Default)]
struct ExitState {
    exited: bool,
    code: Option<i32>,
}

pub struct PtySession {
    pub key: String,
    pub instance: u64,
    pub launch: LaunchSpec,
    started_at: i64,
    pid: Option<u32>,
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    output: Mutex<OutputState>,
    exit: Mutex<ExitState>,
    hibernated: AtomicBool,
}

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_millis() as i64).unwrap_or(0)
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub struct SessionParts {
    pub key: String,
    pub launch: LaunchSpec,
    pub master: Box<dyn MasterPty + Send>,
    pub writer: Box<dyn Write + Send>,
    pub pid: Option<u32>,
    pub size: PtySize,
    pub scrollback_lines: u32,
}

impl PtySession {
    pub fn new(parts: SessionParts) -> Self {
        let started_at = now_ms();
        Self {
            key: parts.key,
            instance: NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed),
            launch: parts.launch,
            started_at,
            pid: parts.pid,
            master: Mutex::new(parts.master),
            writer: Mutex::new(parts.writer),
            output: Mutex::new(OutputState {
                ring: OutputRing::with_scrollback_lines(parts.scrollback_lines),
                screen: TerminalScreen::new(parts.size.rows, parts.size.cols),
                subscriber: None,
                preview_dirty: false,
                last_output_at: started_at,
            }),
            exit: Mutex::new(ExitState::default()),
            hibernated: AtomicBool::new(false),
        }
    }

    pub fn mark_hibernated(&self) {
        self.hibernated.store(true, Ordering::SeqCst);
    }

    pub fn is_hibernated(&self) -> bool {
        self.hibernated.load(Ordering::SeqCst)
    }

    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    pub fn is_running(&self) -> bool {
        !lock(&self.exit).exited
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        let output = lock(&self.output);
        let exit = lock(&self.exit);
        SessionSnapshot {
            key: self.key.clone(),
            instance: self.instance,
            launch: self.launch.clone(),
            started_at: self.started_at,
            last_output_at: output.last_output_at,
            pid: self.pid,
            exit_code: exit.code,
            exited: exit.exited,
            hibernated: self.is_hibernated(),
            preview_lines: output.screen.preview_lines(),
        }
    }

    pub fn write(&self, bytes: &[u8]) -> std::io::Result<()> {
        let mut writer = lock(&self.writer);
        writer.write_all(bytes)?;
        writer.flush()
    }

    pub fn resize(&self, size: PtySize) -> Result<(), String> {
        lock(&self.master).resize(size).map_err(|error| error.to_string())?;
        lock(&self.output).screen.resize(size.rows, size.cols);
        Ok(())
    }

    /// Sends everything kept so far, then streams new output to `sink`.
    pub fn attach(&self, sink: Box<dyn OutputSink>) {
        let mut output = lock(&self.output);
        let replay = output.ring.snapshot();
        if replay.is_empty() || sink.deliver(&replay) {
            output.subscriber = Some(sink);
        }
    }

    pub fn detach(&self) {
        lock(&self.output).subscriber = None;
    }

    fn receive(&self, chunk: &[u8]) {
        let mut output = lock(&self.output);
        output.ring.push(chunk);
        output.screen.process(chunk);
        output.preview_dirty = true;
        output.last_output_at = now_ms();
        let delivered = output.subscriber.as_ref().is_none_or(|subscriber| subscriber.deliver(chunk));
        if !delivered {
            output.subscriber = None;
        }
    }

    /// Returns the new preview when output arrived since the last call.
    pub fn take_preview_if_changed(&self) -> Option<Vec<String>> {
        let mut output = lock(&self.output);
        if !output.preview_dirty {
            return None;
        }
        output.preview_dirty = false;
        Some(output.screen.preview_lines())
    }

    fn mark_exited(&self, code: Option<i32>) {
        let mut exit = lock(&self.exit);
        exit.exited = true;
        exit.code = code;
    }
}

/// Pumps PTY output into the session until the terminal closes, then records the exit code.
pub fn spawn_reader(session: Arc<PtySession>, mut reader: Box<dyn Read + Send>, mut child: Box<dyn Child + Send + Sync>, notify: PtyNotifier) {
    let name = format!("pty-{}", &session.key[..session.key.len().min(8)]);
    let spawned = thread::Builder::new().name(name).spawn(move || {
        let mut buffer = vec![0u8; READ_CHUNK_BYTES];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => session.receive(&buffer[..read]),
            }
        }
        let code = child.wait().ok().map(|status| i32::try_from(status.exit_code()).unwrap_or(i32::MAX));
        session.mark_exited(code);
        notify(PtyEvent::Exited { key: session.key.clone(), code, instance: session.instance });
    });
    if let Err(error) = spawned {
        log::error!("falha ao iniciar a leitura do terminal: {error}");
    }
}
