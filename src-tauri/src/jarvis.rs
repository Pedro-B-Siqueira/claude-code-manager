//! The Jarvis mode: a full-screen window created on demand and destroyed on exit, so nothing of it
//! stays in memory while it is closed. Also reads the user's shape rules and, in development builds,
//! mock sessions for the scene.

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::error::AppError;

pub const WINDOW_LABEL: &str = "jarvis";
pub const SHAPE_RULES_FILE: &str = "jarvis-shapes.json";
const MOCK_ENV: &str = "CCM_JARVIS_MOCK";
const PIXEL_RATIO_ENV: &str = "CCM_JARVIS_PIXEL_RATIO";
const MEASURE_ENV: &str = "CCM_JARVIS_MEASURE";
const BACKGROUND: tauri::window::Color = tauri::window::Color(10, 12, 16, 255);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShapeKind {
    Tower,
    Eye,
    Fortress,
    Orb,
    Crystal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShapeRule {
    #[serde(rename = "match")]
    pub pattern: String,
    pub shape: ShapeKind,
}

/// As written in the file: a shape name the app does not know is skipped, not fatal.
#[derive(Debug, Deserialize)]
struct RawRule {
    #[serde(rename = "match")]
    pattern: String,
    shape: String,
}

#[derive(Debug, Deserialize)]
struct RulesFile {
    #[serde(default)]
    rules: Vec<RawRule>,
}

fn valid_rules(raw: Vec<RawRule>) -> Vec<ShapeRule> {
    raw.into_iter()
        .filter_map(|rule| {
            let pattern = rule.pattern.trim().to_owned();
            match serde_json::from_value::<ShapeKind>(serde_json::Value::String(rule.shape.clone())) {
                Ok(shape) if !pattern.is_empty() => Some(ShapeRule { pattern, shape }),
                Ok(_) => None,
                Err(_) => {
                    log::warn!("{SHAPE_RULES_FILE}: forma desconhecida \"{}\" ignorada", rule.shape);
                    None
                }
            }
        })
        .collect()
}

/// Invalid JSON yields no rules: every repository becomes a crystal instead of breaking the scene.
pub fn parse_shape_rules(json: &str) -> Vec<ShapeRule> {
    match serde_json::from_str::<RulesFile>(json) {
        Ok(file) => valid_rules(file.rules),
        Err(error) => {
            log::warn!("{SHAPE_RULES_FILE} inválido, usando cristais: {error}");
            Vec::new()
        }
    }
}

pub fn load_shape_rules(app_support: &Path) -> Vec<ShapeRule> {
    match std::fs::read_to_string(app_support.join(SHAPE_RULES_FILE)) {
        Ok(json) => parse_shape_rules(&json),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => {
            log::warn!("não foi possível ler {SHAPE_RULES_FILE}: {error}");
            Vec::new()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MockStatus {
    Working,
    Waiting,
    Done,
    Idle,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MockFileSession {
    id: String,
    repo: String,
    name: String,
    status: MockStatus,
    action: Option<String>,
    branch: Option<String>,
    #[serde(default)]
    idle_seconds: u64,
    #[serde(default)]
    events: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MockFile {
    sessions: Vec<MockFileSession>,
    #[serde(default)]
    rules: Vec<RawRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MockSession {
    pub id: String,
    pub repo: String,
    pub name: String,
    pub status: MockStatus,
    pub action: Option<String>,
    pub branch: Option<String>,
    pub last_activity_at: i64,
    pub events: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MockData {
    pub sessions: Vec<MockSession>,
    pub rules: Vec<ShapeRule>,
}

/// The file says how long ago each session was active (`idleSeconds`); the scene wants a timestamp.
pub fn parse_mock_data(json: &str, now_ms: i64) -> Result<MockData, AppError> {
    let file: MockFile = serde_json::from_str(json)?;
    let sessions = file
        .sessions
        .into_iter()
        .map(|session| MockSession {
            id: session.id,
            repo: session.repo,
            name: session.name,
            status: session.status,
            action: session.action,
            branch: session.branch,
            last_activity_at: now_ms - i64::try_from(session.idle_seconds).unwrap_or(0).saturating_mul(1000),
            events: session.events,
        })
        .collect();
    Ok(MockData { sessions, rules: valid_rules(file.rules) })
}

/// Development builds only: `CCM_JARVIS_MOCK=1` uses the built-in mocks, a path loads that file.
pub fn mock_source() -> Option<String> {
    if !cfg!(debug_assertions) {
        return None;
    }
    std::env::var(MOCK_ENV).ok().filter(|value| !value.trim().is_empty())
}

pub fn load_mock_data(now_ms: i64) -> Option<MockData> {
    let source = mock_source().filter(|value| value != "1")?;
    match std::fs::read_to_string(&source).map_err(AppError::from).and_then(|json| parse_mock_data(&json, now_ms)) {
        Ok(data) => Some(data),
        Err(error) => {
            log::warn!("mocks do Jarvis em {source} indisponíveis: {error}");
            None
        }
    }
}

/// What the Jarvis page is told through its URL. Both only exist in development builds.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PageOptions {
    pub mock: bool,
    /// Canvas backing-scale cap, to compare memory and CPU across values.
    pub pixel_ratio: Option<f64>,
}

impl PageOptions {
    pub fn from_environment() -> Self {
        if !cfg!(debug_assertions) {
            return Self::default();
        }
        let pixel_ratio = std::env::var(PIXEL_RATIO_ENV).ok().and_then(|value| value.trim().parse::<f64>().ok()).filter(|ratio| ratio.is_finite() && *ratio > 0.0);
        Self { mock: mock_source().is_some(), pixel_ratio }
    }
}

pub fn page_url(options: &PageOptions) -> String {
    let mut parameters = Vec::new();
    if options.mock {
        parameters.push("source=mock".to_owned());
    }
    if let Some(ratio) = options.pixel_ratio {
        parameters.push(format!("pixelRatio={ratio}"));
    }
    if parameters.is_empty() { "jarvis.html".to_owned() } else { format!("jarvis.html?{}", parameters.join("&")) }
}

/// `CCM_JARVIS_MEASURE="20,110,140,230"`: open at 20 s, close at 110 s, open again at 140 s, close at 230 s.
pub fn parse_measure_schedule(value: &str) -> Option<Vec<(Duration, Duration)>> {
    let seconds = value.split(',').map(|part| part.trim().parse::<u64>().ok()).collect::<Option<Vec<_>>>()?;
    let increasing = seconds.windows(2).all(|pair| pair[0] < pair[1]);
    if seconds.is_empty() || seconds.len() % 2 != 0 || !increasing {
        return None;
    }
    Some(seconds.chunks(2).map(|pair| (Duration::from_secs(pair[0]), Duration::from_secs(pair[1]))).collect())
}

/// Development builds only: opens and closes the Jarvis mode on a timer so memory and CPU can be
/// measured without anyone at the keyboard (see scripts/measure-jarvis.sh).
pub fn start_measure_schedule(app: &AppHandle) {
    if !cfg!(debug_assertions) {
        return;
    }
    let Some(schedule) = std::env::var(MEASURE_ENV).ok().as_deref().and_then(parse_measure_schedule) else {
        return;
    };
    let app = app.clone();
    let started = std::time::Instant::now();
    std::thread::spawn(move || {
        for (open_at, close_at) in schedule {
            std::thread::sleep(open_at.saturating_sub(started.elapsed()));
            log::info!("medição do Jarvis: abrindo");
            if let Err(error) = open(&app) {
                log::warn!("medição do Jarvis: {error}");
            }
            std::thread::sleep(close_at.saturating_sub(started.elapsed()));
            log::info!("medição do Jarvis: fechando");
            close(&app);
        }
    });
}

/// Opens the Jarvis mode full screen on the main window's monitor, or brings it forward if open.
pub fn open(app: &AppHandle) -> Result<(), AppError> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        let _ = window.set_focus();
        return Ok(());
    }
    let main = app.get_webview_window("main");
    let monitor = main.as_ref().and_then(|window| window.current_monitor().ok().flatten());
    let mut builder = WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::App(page_url(&PageOptions::from_environment()).into()))
        .title("Modo Jarvis")
        .decorations(false)
        .background_color(BACKGROUND)
        .incognito(true);
    if let Some(monitor) = monitor {
        let scale = monitor.scale_factor();
        let position = monitor.position().to_logical::<f64>(scale);
        let size = monitor.size().to_logical::<f64>(scale);
        builder = builder.position(position.x, position.y).inner_size(size.width, size.height);
    }
    #[cfg(not(target_os = "macos"))]
    {
        builder = builder.fullscreen(true);
    }
    let window = builder.build().map_err(|error| AppError::Window(error.to_string()))?;
    // Simple full screen covers the screen at once, without the animation into a new Space.
    #[cfg(target_os = "macos")]
    window.set_simple_fullscreen(true).map_err(|error| AppError::Window(error.to_string()))?;
    let _ = window.set_focus();
    Ok(())
}

/// Destroys the window (its web process ends with it) and gives the focus back to the main window.
pub fn close(app: &AppHandle) {
    if app.try_state::<crate::state::AppState>().is_some_and(|state| state.is_locked()) {
        return;
    }
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        // Simple full screen changes app-wide presentation options; undo them before the window goes.
        #[cfg(target_os = "macos")]
        let _ = window.set_simple_fullscreen(false);
        if let Err(error) = window.destroy() {
            log::warn!("falha ao fechar o Modo Jarvis: {error}");
        }
    }
    crate::tray::show_main_window(app);
}

/// Leaves the Jarvis mode and asks the main window to show something: a session or the new-session dialog.
pub fn hand_over(app: &AppHandle, event: &str, payload: Option<&str>) {
    if app.try_state::<crate::state::AppState>().is_some_and(|state| state.is_locked()) {
        return;
    }
    close(app);
    let result = match payload {
        Some(key) => app.emit(event, key),
        None => app.emit(event, ()),
    };
    if let Err(error) = result {
        log::warn!("falha ao avisar a janela principal ({event}): {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_rules_the_user_wrote() {
        let rules = parse_shape_rules(r#"{"rules":[{"match":"api*","shape":"tower"},{"match":"*eye*","shape":"eye"}]}"#);
        assert_eq!(
            rules,
            vec![
                ShapeRule { pattern: "api*".to_owned(), shape: ShapeKind::Tower },
                ShapeRule { pattern: "*eye*".to_owned(), shape: ShapeKind::Eye },
            ]
        );
    }

    #[test]
    fn skips_unknown_shapes_and_empty_patterns_but_keeps_the_rest() {
        let rules = parse_shape_rules(r#"{"rules":[{"match":"a*","shape":"dragon"},{"match":"  ","shape":"orb"},{"match":"b*","shape":"fortress"}]}"#);
        assert_eq!(rules, vec![ShapeRule { pattern: "b*".to_owned(), shape: ShapeKind::Fortress }]);
    }

    #[test]
    fn broken_json_means_no_rules() {
        assert!(parse_shape_rules("{rules: [").is_empty());
        assert!(parse_shape_rules("{}").is_empty());
    }

    #[test]
    fn a_missing_rules_file_is_not_an_error() {
        let folder = tempfile::tempdir().unwrap();
        assert!(load_shape_rules(folder.path()).is_empty());
        std::fs::write(folder.path().join(SHAPE_RULES_FILE), r#"{"rules":[{"match":"x","shape":"crystal"}]}"#).unwrap();
        assert_eq!(load_shape_rules(folder.path()).len(), 1);
    }

    #[test]
    fn mock_sessions_turn_idle_seconds_into_a_timestamp() {
        let data = parse_mock_data(
            r#"{"sessions":[{"id":"s1","repo":"web","name":"login","status":"waiting","action":"Posso seguir?","branch":null,"idleSeconds":99,"events":["Leu a.ts"]}],"rules":[{"match":"web","shape":"orb"}]}"#,
            1_000_000,
        )
        .unwrap();
        assert_eq!(data.sessions[0].last_activity_at, 901_000);
        assert_eq!(data.sessions[0].status, MockStatus::Waiting);
        assert_eq!(data.rules, vec![ShapeRule { pattern: "web".to_owned(), shape: ShapeKind::Orb }]);
        let json = serde_json::to_value(&data).unwrap();
        assert_eq!(json["sessions"][0]["lastActivityAt"], 901_000);
        assert_eq!(json["rules"][0]["match"], "web");
    }

    #[test]
    fn the_page_asks_for_mocks_and_a_pixel_ratio_only_when_told_to() {
        assert_eq!(page_url(&PageOptions::default()), "jarvis.html");
        assert_eq!(page_url(&PageOptions { mock: true, pixel_ratio: None }), "jarvis.html?source=mock");
        assert_eq!(page_url(&PageOptions { mock: true, pixel_ratio: Some(2.0) }), "jarvis.html?source=mock&pixelRatio=2");
        assert_eq!(page_url(&PageOptions { mock: false, pixel_ratio: Some(1.5) }), "jarvis.html?pixelRatio=1.5");
    }

    #[test]
    fn the_measure_schedule_is_a_list_of_open_and_close_seconds() {
        let seconds = |pairs: &[(u64, u64)]| pairs.iter().map(|(open, close)| (Duration::from_secs(*open), Duration::from_secs(*close))).collect::<Vec<_>>();
        assert_eq!(parse_measure_schedule("20,110"), Some(seconds(&[(20, 110)])));
        assert_eq!(parse_measure_schedule(" 20 , 110 ,140, 230"), Some(seconds(&[(20, 110), (140, 230)])));
        assert_eq!(parse_measure_schedule("30,10"), None);
        assert_eq!(parse_measure_schedule("20,110,100,200"), None);
        assert_eq!(parse_measure_schedule("20,110,140"), None);
        assert_eq!(parse_measure_schedule("a,b"), None);
        assert_eq!(parse_measure_schedule(""), None);
    }
}
