use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

use super::session::{LaunchSpec, PtySession, SessionParts, SessionSnapshot, spawn_reader};
use super::{OutputSink, PtyEvent, PtyNotifier};
use crate::error::AppError;

const PREVIEW_INTERVAL: Duration = Duration::from_millis(400);
const DEFAULT_SIZE: PtySize = PtySize { rows: 32, cols: 120, pixel_width: 0, pixel_height: 0 };

/// Program, arguments and environment for the process that runs inside the PTY.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLine {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub env_remove: Vec<String>,
}

pub struct SpawnRequest {
    pub key: String,
    pub launch: LaunchSpec,
    pub command: CommandLine,
    pub scrollback_lines: u32,
}

pub struct PtyManager {
    sessions: Arc<Mutex<HashMap<String, Arc<PtySession>>>>,
    notify: PtyNotifier,
    /// Set while the window is hidden: previews are not sent to a UI nobody is looking at.
    previews_paused: Arc<AtomicBool>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl PtyManager {
    pub fn new(notify: PtyNotifier) -> Self {
        let manager = Self { sessions: Arc::default(), notify, previews_paused: Arc::default() };
        manager.start_preview_ticker();
        manager
    }

    fn start_preview_ticker(&self) {
        let sessions = Arc::clone(&self.sessions);
        let notify = Arc::clone(&self.notify);
        let paused = Arc::clone(&self.previews_paused);
        let spawned = thread::Builder::new().name("pty-preview".to_owned()).spawn(move || loop {
            thread::sleep(PREVIEW_INTERVAL);
            if paused.load(Ordering::Relaxed) {
                continue;
            }
            let current: Vec<Arc<PtySession>> = lock(&sessions).values().cloned().collect();
            for session in current {
                if let Some(lines) = session.take_preview_if_changed() {
                    notify(PtyEvent::PreviewChanged { key: session.key.clone(), lines });
                }
            }
        });
        if let Err(error) = spawned {
            log::error!("falha ao iniciar a prévia dos terminais: {error}");
        }
    }

    pub fn spawn(&self, request: SpawnRequest) -> Result<SessionSnapshot, AppError> {
        if !request.launch.cwd.is_dir() {
            return Err(AppError::MissingDirectory(request.launch.cwd));
        }
        let pair = native_pty_system().openpty(DEFAULT_SIZE).map_err(terminal_error)?;
        let mut builder = CommandBuilder::new(&request.command.program);
        builder.args(&request.command.args);
        builder.cwd(&request.launch.cwd);
        for name in &request.command.env_remove {
            builder.env_remove(name);
        }
        for (name, value) in &request.command.env {
            builder.env(name, value);
        }
        let child = pair.slave.spawn_command(builder).map_err(terminal_error)?;
        drop(pair.slave);
        let reader = pair.master.try_clone_reader().map_err(terminal_error)?;
        let writer = pair.master.take_writer().map_err(terminal_error)?;
        let session = Arc::new(PtySession::new(SessionParts {
            key: request.key.clone(),
            launch: request.launch,
            pid: child.process_id(),
            master: pair.master,
            writer,
            size: DEFAULT_SIZE,
            scrollback_lines: request.scrollback_lines,
        }));
        spawn_reader(Arc::clone(&session), reader, child, Arc::clone(&self.notify));
        let snapshot = session.snapshot();
        lock(&self.sessions).insert(request.key, session);
        Ok(snapshot)
    }

    fn session(&self, key: &str) -> Result<Arc<PtySession>, AppError> {
        lock(&self.sessions).get(key).cloned().ok_or_else(|| AppError::UnknownSession(key.to_owned()))
    }

    pub fn attach(&self, key: &str, sink: Box<dyn OutputSink>) -> Result<(), AppError> {
        self.session(key)?.attach(sink);
        Ok(())
    }

    pub fn detach(&self, key: &str) {
        if let Ok(session) = self.session(key) {
            session.detach();
        }
    }

    pub fn write(&self, key: &str, bytes: &[u8]) -> Result<(), AppError> {
        Ok(self.session(key)?.write(bytes)?)
    }

    pub fn resize(&self, key: &str, cols: u16, rows: u16) -> Result<(), AppError> {
        let size = PtySize { rows: rows.max(2), cols: cols.max(10), pixel_width: 0, pixel_height: 0 };
        self.session(key)?.resize(size).map_err(AppError::Terminal)
    }

    pub fn snapshots(&self) -> Vec<SessionSnapshot> {
        let mut snapshots: Vec<SessionSnapshot> = lock(&self.sessions).values().map(|session| session.snapshot()).collect();
        snapshots.sort_by_key(|snapshot| snapshot.started_at);
        snapshots
    }

    pub fn snapshot(&self, key: &str) -> Result<SessionSnapshot, AppError> {
        Ok(self.session(key)?.snapshot())
    }

    pub fn running_keys(&self) -> Vec<String> {
        lock(&self.sessions).values().filter(|session| session.is_running()).map(|session| session.key.clone()).collect()
    }

    /// Ends a running session (SIGHUP to its process group, SIGKILL after `grace`) or forgets an
    /// exited one. The conversation stays resumable through `claude --resume`.
    pub fn close(&self, key: &str, grace: Duration) -> Result<(), AppError> {
        let session = self.session(key)?;
        if session.is_running() {
            terminate_process_group(session.pid(), grace);
        } else {
            lock(&self.sessions).remove(key);
        }
        Ok(())
    }

    pub fn set_previews_paused(&self, paused: bool) {
        self.previews_paused.store(paused, Ordering::Relaxed);
    }

    /// Ends the process to free memory; the card stays, marked hibernated, until it is woken.
    pub fn hibernate(&self, key: &str, grace: Duration) -> Result<(), AppError> {
        let session = self.session(key)?;
        if !session.is_running() {
            return Ok(());
        }
        session.mark_hibernated();
        terminate_process_group(session.pid(), grace);
        Ok(())
    }

    pub fn forget(&self, key: &str) {
        lock(&self.sessions).remove(key);
    }

    pub fn shutdown_all(&self, grace: Duration) {
        let running: Vec<Arc<PtySession>> = lock(&self.sessions).values().filter(|session| session.is_running()).cloned().collect();
        for session in &running {
            signal_group(session.pid(), libc::SIGHUP);
        }
        let deadline = std::time::Instant::now() + grace;
        while running.iter().any(|session| session.is_running()) && std::time::Instant::now() < deadline {
            thread::sleep(Duration::from_millis(50));
        }
        for session in running.iter().filter(|session| session.is_running()) {
            signal_group(session.pid(), libc::SIGKILL);
        }
    }
}

fn terminal_error(error: impl std::fmt::Display) -> AppError {
    AppError::Terminal(error.to_string())
}

fn signal_group(pid: Option<u32>, signal: libc::c_int) {
    let Some(pid) = pid.and_then(|pid| libc::pid_t::try_from(pid).ok()).filter(|pid| *pid > 1) else { return };
    // The shell is a session leader inside the PTY, so its pid is also the process group id.
    let result = unsafe { libc::killpg(pid, signal) };
    if result != 0 {
        log::debug!("sinal {signal} para o grupo {pid} falhou: {}", std::io::Error::last_os_error());
    }
}

fn terminate_process_group(pid: Option<u32>, grace: Duration) {
    signal_group(pid, libc::SIGHUP);
    let spawned = thread::Builder::new().name("pty-terminate".to_owned()).spawn(move || {
        thread::sleep(grace);
        let Some(raw_pid) = pid.and_then(|pid| libc::pid_t::try_from(pid).ok()) else { return };
        let still_alive = unsafe { libc::kill(raw_pid, 0) } == 0;
        if still_alive {
            signal_group(pid, libc::SIGKILL);
        }
    });
    if let Err(error) = spawned {
        log::warn!("falha ao agendar o encerramento forçado: {error}");
    }
}

impl Drop for PtyManager {
    fn drop(&mut self) {
        self.shutdown_all(Duration::from_millis(500));
    }
}
