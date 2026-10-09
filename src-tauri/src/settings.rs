use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::error::AppError;

const SETTINGS_KEY: &str = "app";
const SCROLLBACK_RANGE: (u32, u32) = (500, 100_000);
const IDLE_MINUTES_RANGE: (u32, u32) = (5, 1_440);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

/// `Auto` turns system notifications off when the ClaudeGauge hook is present, to avoid duplicates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NotificationPreference {
    #[default]
    Auto,
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HibernationSettings {
    pub enabled: bool,
    pub idle_minutes: u32,
}

impl Default for HibernationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            idle_minutes: 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub theme: Theme,
    /// `None` = automatic: follow the folder existing worktrees already use (see `git::infer_worktree_root`).
    pub worktree_root: Option<String>,
    pub branch_prefixes: Vec<String>,
    pub scrollback_lines: u32,
    pub notifications: NotificationPreference,
    pub hibernation: HibernationSettings,
    pub claude_binary: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            worktree_root: None,
            branch_prefixes: vec!["feat-".to_owned(), "fix-".to_owned()],
            scrollback_lines: 5_000,
            notifications: NotificationPreference::default(),
            hibernation: HibernationSettings::default(),
            claude_binary: None,
        }
    }
}

impl AppSettings {
    pub fn sanitized(mut self) -> Self {
        self.scrollback_lines = self
            .scrollback_lines
            .clamp(SCROLLBACK_RANGE.0, SCROLLBACK_RANGE.1);
        self.hibernation.idle_minutes = self
            .hibernation
            .idle_minutes
            .clamp(IDLE_MINUTES_RANGE.0, IDLE_MINUTES_RANGE.1);
        self.claude_binary = self
            .claude_binary
            .filter(|binary| !binary.trim().is_empty());
        self.worktree_root = self.worktree_root.filter(|root| !root.trim().is_empty());
        self.branch_prefixes.retain(|prefix| !prefix.trim().is_empty() && prefix.len() <= 24);
        self.branch_prefixes.dedup();
        if self.branch_prefixes.is_empty() {
            self.branch_prefixes = AppSettings::default().branch_prefixes;
        }
        self
    }
}

pub fn load(database: &Database) -> Result<AppSettings, AppError> {
    let stored: Option<String> = database
        .connection()
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![SETTINGS_KEY],
            |row| row.get(0),
        )
        .optional()?;
    let Some(json) = stored else {
        return Ok(AppSettings::default());
    };
    match serde_json::from_str::<AppSettings>(&json) {
        Ok(settings) => Ok(settings.sanitized()),
        Err(error) => {
            log::warn!("configurações salvas inválidas, usando padrão: {error}");
            Ok(AppSettings::default())
        }
    }
}

pub fn save(database: &Database, settings: AppSettings) -> Result<AppSettings, AppError> {
    let sanitized = settings.sanitized();
    let json = serde_json::to_string(&sanitized)?;
    database.connection().execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![SETTINGS_KEY, json],
    )?;
    Ok(sanitized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_agreed_decisions() {
        let settings = AppSettings::default();
        assert_eq!(settings.worktree_root, None, "worktree folder is inferred unless the user sets one");
        assert_eq!(settings.branch_prefixes, vec!["feat-", "fix-"]);
        assert!(settings.hibernation.enabled);
        assert_eq!(settings.hibernation.idle_minutes, 30);
        assert_eq!(settings.scrollback_lines, 5_000);
    }

    #[test]
    fn missing_row_returns_defaults() {
        let database = Database::open_in_memory().unwrap();
        assert_eq!(load(&database).unwrap(), AppSettings::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let database = Database::open_in_memory().unwrap();
        let light = AppSettings {
            theme: Theme::Light,
            ..AppSettings::default()
        };
        save(&database, light.clone()).unwrap();
        assert_eq!(load(&database).unwrap(), light);
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        let database = Database::open_in_memory().unwrap();
        let extreme = AppSettings {
            scrollback_lines: 1,
            hibernation: HibernationSettings {
                enabled: true,
                idle_minutes: 0,
            },
            ..AppSettings::default()
        };
        let saved = save(&database, extreme).unwrap();
        assert_eq!(saved.scrollback_lines, 500);
        assert_eq!(saved.hibernation.idle_minutes, 5);
    }

    #[test]
    fn unknown_fields_and_missing_fields_are_tolerated() {
        let parsed: AppSettings =
            serde_json::from_str(r#"{"theme":"light","futureOption":true}"#).unwrap();
        assert_eq!(parsed.theme, Theme::Light);
        assert_eq!(parsed.scrollback_lines, 5_000);
    }
}
