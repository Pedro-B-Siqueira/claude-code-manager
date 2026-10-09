use rusqlite::Connection;

use crate::error::AppError;

/// Ordered schema steps; `PRAGMA user_version` records how many were applied.
const MIGRATIONS: &[&str] = &["CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    ) STRICT;"];

pub fn apply(connection: &Connection) -> Result<(), AppError> {
    let applied: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let applied = usize::try_from(applied).unwrap_or(0);
    for (index, statement) in MIGRATIONS.iter().enumerate().skip(applied) {
        connection.execute_batch(statement)?;
        let version = i64::try_from(index + 1).unwrap_or(i64::MAX);
        connection.pragma_update(None, "user_version", version)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applying_twice_is_a_no_op() {
        let connection = Connection::open_in_memory().unwrap();
        apply(&connection).unwrap();
        apply(&connection).unwrap();
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, i64::try_from(MIGRATIONS.len()).unwrap());
    }
}
