//! Menu bar icon: how many sessions need the user, plus a quick list to jump to any of them.
//! It never shows quota (that stays with ClaudeGauge).

use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::live::service::live_views;
use crate::sessions::view::LiveSessionView;
use crate::state::AppState;
use crate::status::SessionStatus;

const ICON_SIZE: u32 = 36;
const COALESCE: Duration = Duration::from_millis(500);
const MAX_LISTED: usize = 12;
const SESSION_ITEM_PREFIX: &str = "session:";
const OPEN_ITEM: &str = "open";
const QUIT_ITEM: &str = "quit";
const TITLE_MAX_CHARS: usize = 48;

/// Four rounded tiles in black on transparent: a template image, so macOS tints it for light and dark bars.
pub fn template_icon_rgba() -> Vec<u8> {
    let tile = 15;
    let gap = 2;
    let margin = (ICON_SIZE - 2 * tile - gap) / 2;
    let radius = 4;
    let mut pixels = vec![0u8; (ICON_SIZE * ICON_SIZE * 4) as usize];
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            if inside_tile(x, y, margin, tile, gap, radius) {
                let index = ((y * ICON_SIZE + x) * 4) as usize;
                pixels[index + 3] = 255;
            }
        }
    }
    pixels
}

fn inside_tile(x: u32, y: u32, margin: u32, tile: u32, gap: u32, radius: u32) -> bool {
    let local = |value: u32| -> Option<u32> {
        let offset = value.checked_sub(margin)?;
        let within = offset % (tile + gap);
        (offset < 2 * tile + gap && within < tile).then_some(within)
    };
    let (Some(local_x), Some(local_y)) = (local(x), local(y)) else { return false };
    let corner_x = if local_x < radius { radius - local_x } else if local_x >= tile - radius { local_x + radius + 1 - tile } else { 0 };
    let corner_y = if local_y < radius { radius - local_y } else if local_y >= tile - radius { local_y + radius + 1 - tile } else { 0 };
    corner_x * corner_x + corner_y * corner_y <= radius * radius
}

fn status_label(status: SessionStatus) -> &'static str {
    match status {
        SessionStatus::Working => "Trabalhando",
        SessionStatus::Permission => "Pedindo permissão",
        SessionStatus::Waiting => "Esperando você",
        SessionStatus::Done => "Concluída",
        SessionStatus::Idle => "Ociosa",
    }
}

fn shorten(text: &str) -> String {
    crate::transcript::tools::truncate_chars(text, TITLE_MAX_CHARS)
}

/// Sessions that need the user first, then the rest that are still running.
pub fn menu_entries(views: &[LiveSessionView]) -> Vec<(String, String)> {
    let mut running: Vec<&LiveSessionView> = views.iter().filter(|view| !view.exited).collect();
    running.sort_by_key(|view| (!view.status.needs_user(), view.started_at));
    running
        .into_iter()
        .take(MAX_LISTED)
        .map(|view| (format!("{SESSION_ITEM_PREFIX}{}", view.key), format!("{} · {}", status_label(view.status), shorten(&view.title))))
        .collect()
}

pub fn title_for(needs_you: usize) -> Option<String> {
    (needs_you > 0).then(|| needs_you.to_string())
}

fn build_menu(app: &AppHandle, views: &[LiveSessionView]) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::new(app)?;
    let entries = menu_entries(views);
    if entries.is_empty() {
        menu.append(&MenuItem::with_id(app, "empty", "Nenhuma sessão aberta", false, None::<&str>)?)?;
    }
    for (id, label) in entries {
        menu.append(&MenuItem::with_id(app, id, label, true, None::<&str>)?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, OPEN_ITEM, "Abrir Claude Code Manager", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, QUIT_ITEM, "Sair", true, Some("CmdOrCtrl+Q"))?)?;
    Ok(menu)
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn handle_menu(app: &AppHandle, id: &str, request_quit: fn(&AppHandle)) {
    if let Some(key) = id.strip_prefix(SESSION_ITEM_PREFIX) {
        show_main_window(app);
        if let Err(error) = app.emit("tray:focus-session", key) {
            log::warn!("falha ao focar a sessão pela barra de menus: {error}");
        }
    } else if id == OPEN_ITEM {
        show_main_window(app);
    } else if id == QUIT_ITEM {
        request_quit(app);
    }
}

pub struct TrayUpdater {
    trigger: Sender<()>,
}

impl TrayUpdater {
    pub fn schedule(&self) {
        let _ = self.trigger.send(());
    }
}

pub fn install(app: &AppHandle, request_quit: fn(&AppHandle)) -> tauri::Result<TrayUpdater> {
    let icon = Image::new_owned(template_icon_rgba(), ICON_SIZE, ICON_SIZE);
    let tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("Claude Code Manager")
        .menu(&build_menu(app, &[])?)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| handle_menu(app, event.id().as_ref(), request_quit))
        .build(app)?;
    let (trigger, receiver) = mpsc::channel::<()>();
    let handle = app.clone();
    thread::Builder::new().name("tray-updater".to_owned()).spawn(move || {
        while receiver.recv().is_ok() {
            thread::sleep(COALESCE);
            while receiver.try_recv().is_ok() {}
            refresh(&handle, &tray);
        }
    })?;
    Ok(TrayUpdater { trigger })
}

fn refresh(app: &AppHandle, tray: &TrayIcon) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let views = match live_views(&state) {
        Ok(views) => views,
        Err(error) => {
            log::warn!("falha ao atualizar a barra de menus: {error}");
            return;
        }
    };
    let needs_you = views.iter().filter(|view| !view.exited && view.status.needs_user()).count();
    let menu = build_menu(app, &views);
    let tray = tray.clone();
    let _ = app.run_on_main_thread(move || {
        let _ = tray.set_title(title_for(needs_you));
        let _ = tray.set_tooltip(Some(match needs_you {
            0 => "Claude Code Manager".to_owned(),
            1 => "1 sessão precisa de você".to_owned(),
            count => format!("{count} sessões precisam de você"),
        }));
        if let Ok(menu) = menu {
            let _ = tray.set_menu(Some(menu));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sessions::view::SessionOrigin;

    fn view(key: &str, status: SessionStatus, started_at: i64, exited: bool) -> LiveSessionView {
        LiveSessionView {
            key: key.to_owned(),
            session_id: key.to_owned(),
            title: format!("Sessão {key}"),
            repo: "repo".to_owned(),
            cwd: "/repo".to_owned(),
            branch: None,
            is_worktree: false,
            status,
            status_detail: None,
            preview_lines: Vec::new(),
            files: Vec::new(),
            context_percent: None,
            total_tokens: 0,
            cost_usd: 0.0,
            memory_mb: None,
            origin: SessionOrigin::App,
            pid: None,
            hibernated: false,
            pinned: false,
            exited,
            started_at,
        }
    }

    #[test]
    fn sessions_needing_the_user_come_first() {
        let views = [
            view("a", SessionStatus::Working, 1, false),
            view("b", SessionStatus::Waiting, 2, false),
            view("c", SessionStatus::Permission, 3, false),
            view("d", SessionStatus::Idle, 4, true),
        ];
        let entries = menu_entries(&views);
        let ids: Vec<&str> = entries.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(ids, vec!["session:b", "session:c", "session:a"]);
        assert_eq!(entries[1].1, "Pedindo permissão · Sessão c");
    }

    #[test]
    fn title_shows_only_a_positive_count() {
        assert_eq!(title_for(0), None);
        assert_eq!(title_for(3).as_deref(), Some("3"));
    }

    #[test]
    fn template_icon_is_black_with_transparent_background() {
        let pixels = template_icon_rgba();
        assert_eq!(pixels.len(), (ICON_SIZE * ICON_SIZE * 4) as usize);
        let opaque = pixels.chunks(4).filter(|pixel| pixel[3] == 255).count();
        assert!(opaque > 600 && opaque < (ICON_SIZE * ICON_SIZE) as usize);
        assert!(pixels.chunks(4).all(|pixel| pixel[0] == 0 && pixel[1] == 0 && pixel[2] == 0));
        assert_eq!(pixels[3], 0, "corner pixel is transparent");
    }
}
