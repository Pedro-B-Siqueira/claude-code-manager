mod migrations;

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

use crate::error::AppError;

/// Single SQLite connection for the app's own data (never Claude Code's files).
pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn open(file: &Path) -> Result<Self, AppError> {
        let connection = Connection::open(file)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        Self::initialize(connection)
    }

    pub fn open_in_memory() -> Result<Self, AppError> {
        Self::initialize(Connection::open_in_memory()?)
    }

    fn initialize(connection: Connection) -> Result<Self, AppError> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        migrations::apply(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn connection(&self) -> MutexGuard<'_, Connection> {
        // A panic while holding the lock cannot leave SQLite half-written, so poisoning is safe to ignore.
        self.connection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
