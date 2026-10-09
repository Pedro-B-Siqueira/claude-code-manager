//! UI layout the user arranged: card order in the grid and panel widths. Stored only in the app's
//! own database.

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::error::AppError;

const LAYOUT_KEY: &str = "layout";
const SIDEBAR_RANGE: (u32, u32) = (200, 380);
const FOCUS_PANEL_RANGE: (u32, u32) = (240, 560);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PanelLayout {
    pub sidebar_width: u32,
    pub focus_panel_width: u32,
}

impl Default for PanelLayout {
    fn default() -> Self {
        Self { sidebar_width: 248, focus_panel_width: 300 }
    }
}

impl PanelLayout {
    fn clamped(self) -> Self {
        Self {
            sidebar_width: self.sidebar_width.clamp(SIDEBAR_RANGE.0, SIDEBAR_RANGE.1),
            focus_panel_width: self.focus_panel_width.clamp(FOCUS_PANEL_RANGE.0, FOCUS_PANEL_RANGE.1),
        }
    }
}

pub fn load_layout(database: &Database) -> Result<PanelLayout, AppError> {
    let stored: Option<String> = database
        .connection()
        .query_row("SELECT value FROM settings WHERE key = ?1", params![LAYOUT_KEY], |row| row.get(0))
        .optional()?;
    Ok(stored.and_then(|json| serde_json::from_str::<PanelLayout>(&json).ok()).unwrap_or_default().clamped())
}

pub fn save_layout(database: &Database, layout: PanelLayout) -> Result<PanelLayout, AppError> {
    let layout = layout.clamped();
    database.connection().execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![LAYOUT_KEY, serde_json::to_string(&layout)?],
    )?;
    Ok(layout)
}

pub fn load_grid_order(database: &Database) -> Result<Vec<String>, AppError> {
    let connection = database.connection();
    let mut statement = connection.prepare("SELECT session_id FROM grid_order ORDER BY position")?;
    let rows = statement.query_map([], |row| row.get(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn save_grid_order(database: &Database, session_ids: &[String]) -> Result<(), AppError> {
    let mut connection = database.connection();
    let transaction = connection.transaction()?;
    transaction.execute("DELETE FROM grid_order", [])?;
    for (position, session_id) in session_ids.iter().enumerate() {
        transaction.execute(
            "INSERT OR IGNORE INTO grid_order (session_id, position) VALUES (?1, ?2)",
            params![session_id, i64::try_from(position).unwrap_or(i64::MAX)],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_round_trips_and_is_clamped() {
        let database = Database::open_in_memory().unwrap();
        assert_eq!(load_layout(&database).unwrap(), PanelLayout::default());
        let saved = save_layout(&database, PanelLayout { sidebar_width: 5_000, focus_panel_width: 10 }).unwrap();
        assert_eq!(saved, PanelLayout { sidebar_width: 380, focus_panel_width: 240 });
        assert_eq!(load_layout(&database).unwrap(), saved);
    }

    #[test]
    fn grid_order_replaces_the_previous_order() {
        let database = Database::open_in_memory().unwrap();
        save_grid_order(&database, &["b".to_owned(), "a".to_owned()]).unwrap();
        save_grid_order(&database, &["c".to_owned(), "b".to_owned(), "c".to_owned()]).unwrap();
        assert_eq!(load_grid_order(&database).unwrap(), vec!["c".to_owned(), "b".to_owned()]);
    }
}
