mod migrations;

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;
use std::time::Duration;

use crate::error::AppError;

/// The indexer writes on its own connection; a waiting writer retries for this long before failing.
const BUSY_TIMEOUT: Duration = Duration::from_secs(10);

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
        connection.busy_timeout(BUSY_TIMEOUT)?;
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

/// SQLite integers are signed; counters and offsets saturate instead of wrapping.
pub fn sql_int(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// Reads back a value stored by [`sql_int`]; negative values (never written) become zero.
pub fn from_sql_int(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}
