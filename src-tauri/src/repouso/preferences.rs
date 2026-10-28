//! The activation dialog remembers whether the locked screen shows what each agent is doing.

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::error::AppError;

const PREFERENCES_KEY: &str = "repouso";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepousoPreferences {
    show_details: bool,
}

/// Unreadable or missing preferences fall back to showing the details, as the dialog does by default.
pub fn load_show_details(database: &Database) -> bool {
    let stored: Option<String> = database
        .connection()
        .query_row("SELECT value FROM settings WHERE key = ?1", params![PREFERENCES_KEY], |row| row.get(0))
        .optional()
        .unwrap_or_else(|error| {
            log::warn!("preferências do Repouso ilegíveis: {error}");
            None
        });
    stored.and_then(|json| serde_json::from_str::<RepousoPreferences>(&json).ok()).is_none_or(|preferences| preferences.show_details)
}

pub fn save_show_details(database: &Database, show_details: bool) -> Result<(), AppError> {
    let json = serde_json::to_string(&RepousoPreferences { show_details })?;
    database.connection().execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![PREFERENCES_KEY, json],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn details_show_by_default() {
        let database = Database::open_in_memory().unwrap();
        assert!(load_show_details(&database));
    }

    #[test]
    fn the_last_choice_is_remembered() {
        let database = Database::open_in_memory().unwrap();
        save_show_details(&database, false).unwrap();
        assert!(!load_show_details(&database));
        save_show_details(&database, true).unwrap();
        assert!(load_show_details(&database));
    }
}
