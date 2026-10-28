//! The real side effects of each Repouso step. Only macOS can lock the screen; elsewhere activation
//! is refused before anything changes.

#[cfg(target_os = "macos")]
pub use macos::AppLockPlatform;
#[cfg(not(target_os = "macos"))]
pub use unsupported::AppLockPlatform;

/// Windows covering the displays the Jarvis window is not on.
pub const LOCK_WINDOW_PREFIX: &str = "lock-";

#[cfg(target_os = "macos")]
mod macos {
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, mpsc};
    use std::time::Duration;

    use objc2::MainThreadMarker;
    use tauri::menu::Menu;
    use tauri::{AppHandle, Manager, Monitor, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Wry};

    use super::LOCK_WINDOW_PREFIX;
    use crate::error::AppError;
    use crate::jarvis;
    use crate::repouso::lock::{LockInfo, LockPlatform, LockStep};
    use crate::repouso::native::{self, LOCK_WINDOW_LEVEL, NORMAL_WINDOW_LEVEL};
    use crate::repouso::pending::{DISPLAY_OFF, PENDING_FILE, PendingLock, remove_pending, watchdog_command, write_pending};

    const MAIN_THREAD_WAIT: Duration = Duration::from_secs(5);
    const SCREEN_CHECK: Duration = Duration::from_secs(1);
    const LOCK_BACKGROUND: tauri::window::Color = tauri::window::Color(10, 10, 10, 255);

    #[derive(Default)]
    struct Helpers {
        watchdog: Option<Child>,
        caffeinate: Option<Child>,
    }

    pub struct AppLockPlatform {
        app: AppHandle,
        app_support: PathBuf,
        helpers: Mutex<Helpers>,
        saved_presentation: Mutex<Option<usize>>,
        saved_menu: Mutex<Option<Menu<Wry>>>,
        covering: Arc<AtomicBool>,
    }

    impl AppLockPlatform {
        pub fn new(app: AppHandle, app_support: PathBuf) -> Self {
            Self {
                app,
                app_support,
                helpers: Mutex::default(),
                saved_presentation: Mutex::default(),
                saved_menu: Mutex::default(),
                covering: Arc::default(),
            }
        }

        fn remove_app_menu(&self) -> Result<(), AppError> {
            let removed = self.app.remove_menu().map_err(window_error)?;
            *self.saved_menu.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = removed;
            Ok(())
        }

        fn restore_app_menu(&self) {
            let saved = self.saved_menu.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take();
            let menu = match saved {
                Some(menu) => Ok(menu),
                None => Menu::default(&self.app),
            };
            if let Err(error) = menu.and_then(|menu| self.app.set_menu(menu)) {
                log::warn!("Repouso: falha ao devolver o menu do app: {error}");
            }
        }

        fn helpers(&self) -> std::sync::MutexGuard<'_, Helpers> {
            self.helpers.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
        }

        fn start_watchdog(&self, lock: &LockInfo) -> Result<(), AppError> {
            let child = watchdog_command(std::process::id(), &self.app_support.join(PENDING_FILE), DISPLAY_OFF).spawn()?;
            write_pending(&self.app_support, &PendingLock { app_pid: std::process::id(), since: lock.since_ms, watchdog_pid: Some(child.id()) })?;
            self.helpers().watchdog = Some(child);
            Ok(())
        }

        /// `-w`: caffeinate quits on its own when the app does, even after `kill -9`.
        fn start_caffeinate(&self) -> Result<(), AppError> {
            let child = Command::new("/usr/bin/caffeinate")
                .args(["-d", "-i", "-w", &std::process::id().to_string()])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            self.helpers().caffeinate = Some(child);
            Ok(())
        }

        fn lock_jarvis_window(&self) -> Result<(), AppError> {
            jarvis::open(&self.app)?;
            let window = jarvis_window(&self.app)?;
            window.set_minimizable(false).map_err(window_error)?;
            window.set_visible_on_all_workspaces(true).map_err(window_error)?;
            set_level(&self.app, window.clone(), LOCK_WINDOW_LEVEL)?;
            window.set_focus().map_err(window_error)
        }

        fn unlock_jarvis_window(&self) {
            let Ok(window) = jarvis_window(&self.app) else { return };
            let _ = set_level(&self.app, window.clone(), NORMAL_WINDOW_LEVEL);
            let _ = window.set_minimizable(true);
            let _ = window.set_visible_on_all_workspaces(false);
            let _ = window.set_focus();
        }

        fn enter_kiosk(&self) -> Result<(), AppError> {
            let previous = on_main_thread(&self.app, native::enter_kiosk)?;
            *self.saved_presentation.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(previous);
            Ok(())
        }

        fn leave_kiosk(&self) {
            let previous = self.saved_presentation.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take();
            if let Some(bits) = previous {
                if let Err(error) = on_main_thread(&self.app, move |main_thread| native::restore_presentation(main_thread, bits)) {
                    log::warn!("Repouso: falha ao restaurar Dock e barra de menus: {error}");
                }
            }
        }

        /// Covers every other display now, then keeps checking once a second for displays that come or go.
        fn cover_screens(&self) -> Result<(), AppError> {
            self.covering.store(true, Ordering::SeqCst);
            let mut covered = cover_other_displays(&self.app, &[])?;
            let app = self.app.clone();
            let covering = Arc::clone(&self.covering);
            std::thread::Builder::new().name("repouso-screens".to_owned()).spawn(move || {
                while covering.load(Ordering::SeqCst) {
                    std::thread::sleep(SCREEN_CHECK);
                    if !covering.load(Ordering::SeqCst) {
                        break;
                    }
                    match cover_other_displays(&app, &covered) {
                        Ok(now_covered) => covered = now_covered,
                        Err(error) => log::warn!("Repouso: falha ao cobrir as telas: {error}"),
                    }
                }
            })?;
            Ok(())
        }

        fn uncover_screens(&self) {
            self.covering.store(false, Ordering::SeqCst);
            close_lock_windows(&self.app);
        }
    }

    impl LockPlatform for AppLockPlatform {
        fn engage(&self, step: LockStep, lock: &LockInfo) -> Result<(), AppError> {
            match step {
                LockStep::PendingFlag => write_pending(&self.app_support, &PendingLock { app_pid: std::process::id(), since: lock.since_ms, watchdog_pid: None }),
                LockStep::Watchdog => self.start_watchdog(lock),
                LockStep::NoSleep => self.start_caffeinate(),
                LockStep::HideMainWindow => self.app.get_webview_window("main").map_or(Ok(()), |main| main.hide().map_err(window_error)),
                LockStep::LockJarvisWindow => self.lock_jarvis_window(),
                LockStep::RemoveAppMenu => self.remove_app_menu(),
                LockStep::Kiosk => self.enter_kiosk(),
                LockStep::CoverScreens => self.cover_screens(),
            }
        }

        fn release(&self, step: LockStep) {
            match step {
                LockStep::PendingFlag => remove_pending(&self.app_support),
                LockStep::Watchdog => stop(self.helpers().watchdog.take()),
                LockStep::NoSleep => stop(self.helpers().caffeinate.take()),
                LockStep::HideMainWindow => {
                    if let Some(main) = self.app.get_webview_window("main") {
                        let _ = main.show();
                    }
                }
                LockStep::LockJarvisWindow => self.unlock_jarvis_window(),
                LockStep::RemoveAppMenu => self.restore_app_menu(),
                LockStep::Kiosk => self.leave_kiosk(),
                LockStep::CoverScreens => self.uncover_screens(),
            }
        }

        fn set_prompt_mode(&self, prompting: bool) {
            let level = if prompting { NORMAL_WINDOW_LEVEL } else { LOCK_WINDOW_LEVEL };
            for (label, window) in self.app.webview_windows() {
                if label == jarvis::WINDOW_LABEL || label.starts_with(LOCK_WINDOW_PREFIX) {
                    if let Err(error) = set_level(&self.app, window, level) {
                        log::warn!("Repouso: falha ao ajustar o nível da janela {label}: {error}");
                    }
                }
            }
        }

        fn authenticate(&self) -> bool {
            native::authenticate()
        }
    }

    fn window_error(error: tauri::Error) -> AppError {
        AppError::Window(error.to_string())
    }

    fn jarvis_window(app: &AppHandle) -> Result<WebviewWindow, AppError> {
        app.get_webview_window(jarvis::WINDOW_LABEL).ok_or_else(|| AppError::Window("a janela do Modo Jarvis não está aberta".to_owned()))
    }

    fn stop(child: Option<Child>) {
        if let Some(mut child) = child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// AppKit must be called on the main thread; this runs `work` there and waits for its result.
    /// Callers are command and helper threads, never the main thread itself.
    fn on_main_thread<T: Send + 'static>(app: &AppHandle, work: impl FnOnce(MainThreadMarker) -> T + Send + 'static) -> Result<T, AppError> {
        let (sender, receiver) = mpsc::channel();
        app.run_on_main_thread(move || {
            if let Some(main_thread) = MainThreadMarker::new() {
                let _ = sender.send(work(main_thread));
            }
        })
        .map_err(window_error)?;
        receiver.recv_timeout(MAIN_THREAD_WAIT).map_err(|_| AppError::Window("a thread principal não respondeu".to_owned()))
    }

    fn set_level(app: &AppHandle, window: WebviewWindow, level: isize) -> Result<(), AppError> {
        on_main_thread(app, move |_| match window.ns_window() {
            // SAFETY: the pointer is this live window's own NSWindow, used here on the main thread.
            Ok(ns_window) => unsafe { native::set_window_level(ns_window, level) },
            Err(error) => log::warn!("Repouso: janela sem NSWindow: {error}"),
        })
    }

    fn display_key(monitor: &Monitor) -> (i32, i32, u32, u32) {
        let position = monitor.position();
        let size = monitor.size();
        (position.x, position.y, size.width, size.height)
    }

    /// Recreates the lock windows when the set of displays to cover changed. Returns the covered displays.
    fn cover_other_displays(app: &AppHandle, covered: &[(i32, i32, u32, u32)]) -> Result<Vec<(i32, i32, u32, u32)>, AppError> {
        let jarvis = jarvis_window(app)?;
        let jarvis_display = jarvis.current_monitor().map_err(window_error)?.map(|monitor| display_key(&monitor));
        let others: Vec<Monitor> = jarvis.available_monitors().map_err(window_error)?.into_iter().filter(|monitor| Some(display_key(monitor)) != jarvis_display).collect();
        let keys: Vec<_> = others.iter().map(display_key).collect();
        if keys == covered {
            return Ok(keys);
        }
        close_lock_windows(app);
        for (index, monitor) in others.iter().enumerate() {
            open_lock_window(app, index, monitor)?;
        }
        Ok(keys)
    }

    fn open_lock_window(app: &AppHandle, index: usize, monitor: &Monitor) -> Result<(), AppError> {
        let scale = monitor.scale_factor();
        let position: PhysicalPosition<i32> = *monitor.position();
        let logical_position = position.to_logical::<f64>(scale);
        let size = monitor.size().to_logical::<f64>(scale);
        let window = WebviewWindowBuilder::new(app, format!("{LOCK_WINDOW_PREFIX}{index}"), WebviewUrl::App("lock.html".into()))
            .title("Bloqueado")
            .decorations(false)
            .background_color(LOCK_BACKGROUND)
            .incognito(true)
            .focused(false)
            .minimizable(false)
            .visible_on_all_workspaces(true)
            .position(logical_position.x, logical_position.y)
            .inner_size(size.width, size.height)
            .build()
            .map_err(window_error)?;
        set_level(app, window, LOCK_WINDOW_LEVEL)
    }

    fn close_lock_windows(app: &AppHandle) {
        for (label, window) in app.webview_windows() {
            if label.starts_with(LOCK_WINDOW_PREFIX) {
                if let Err(error) = window.destroy() {
                    log::warn!("Repouso: falha ao fechar a janela {label}: {error}");
                }
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod unsupported {
    use std::path::PathBuf;

    use tauri::AppHandle;

    use crate::error::AppError;
    use crate::repouso::lock::{LockInfo, LockPlatform, LockStep};

    pub struct AppLockPlatform;

    impl AppLockPlatform {
        pub fn new(_app: AppHandle, _app_support: PathBuf) -> Self {
            Self
        }
    }

    impl LockPlatform for AppLockPlatform {
        fn engage(&self, _step: LockStep, _lock: &LockInfo) -> Result<(), AppError> {
            Err(AppError::Invalid("O Repouso só existe no macOS.".to_owned()))
        }

        fn release(&self, _step: LockStep) {}

        fn set_prompt_mode(&self, _prompting: bool) {}

        fn authenticate(&self) -> bool {
            false
        }
    }
}
