use rusqlite::Connection;

use crate::error::AppError;

/// Ordered schema steps; `PRAGMA user_version` records how many were applied.
const MIGRATIONS: &[&str] = &[
    "CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    ) STRICT;",
    LIBRARY_SCHEMA,
    "CREATE TABLE grid_order (
        session_id TEXT PRIMARY KEY,
        position   INTEGER NOT NULL
    ) STRICT;",
    EDITS_PER_SESSION,
];

/// Edits become unique per session (a forked session keeps its own copies). Only derived data is
/// rebuilt: the transcripts are re-read on the next start; names, tags and pins are kept.
const EDITS_PER_SESSION: &str = "
    DROP TABLE file_edits;
    CREATE TABLE file_edits (
        id              INTEGER PRIMARY KEY,
        session_id      TEXT NOT NULL,
        tool_use_id     TEXT NOT NULL,
        timestamp       INTEGER,
        tool            TEXT NOT NULL,
        file_path       TEXT NOT NULL,
        new_start       INTEGER,
        new_end         INTEGER,
        added           INTEGER NOT NULL,
        removed         INTEGER NOT NULL,
        is_new_file     INTEGER NOT NULL DEFAULT 0,
        status          TEXT NOT NULL DEFAULT 'pending',
        transcript_path TEXT NOT NULL,
        line_offset     INTEGER NOT NULL,
        line_length     INTEGER NOT NULL,
        why             TEXT,
        from_subagent   INTEGER NOT NULL DEFAULT 0,
        UNIQUE (session_id, tool_use_id)
    ) STRICT;
    CREATE INDEX file_edits_session ON file_edits(session_id, file_path);
    DELETE FROM activity;
    DELETE FROM session_fts;
    DELETE FROM sessions;
    DELETE FROM transcript_files;
";

const LIBRARY_SCHEMA: &str = "
    CREATE TABLE transcript_files (
        path                TEXT PRIMARY KEY,
        session_id          TEXT NOT NULL,
        is_subagent         INTEGER NOT NULL,
        inode               INTEGER NOT NULL,
        size                INTEGER NOT NULL,
        byte_offset         INTEGER NOT NULL,
        modified_ms         INTEGER NOT NULL,
        last_message_id     TEXT,
        last_assistant_text TEXT
    ) STRICT;
    CREATE INDEX transcript_files_session ON transcript_files(session_id);

    CREATE TABLE sessions (
        id                    TEXT PRIMARY KEY,
        project_dir           TEXT NOT NULL,
        transcript_path       TEXT NOT NULL DEFAULT '',
        cwd                   TEXT,
        git_branch            TEXT,
        ai_title              TEXT,
        first_prompt          TEXT,
        last_prompt           TEXT,
        last_assistant        TEXT,
        model                 TEXT,
        pr_url                TEXT,
        started_at            INTEGER,
        updated_at            INTEGER,
        input_tokens          INTEGER NOT NULL DEFAULT 0,
        output_tokens         INTEGER NOT NULL DEFAULT 0,
        cache_read_tokens     INTEGER NOT NULL DEFAULT 0,
        cache_write_5m_tokens INTEGER NOT NULL DEFAULT 0,
        cache_write_1h_tokens INTEGER NOT NULL DEFAULT 0,
        cost_usd              REAL NOT NULL DEFAULT 0,
        unknown_pricing       INTEGER NOT NULL DEFAULT 0,
        context_tokens        INTEGER,
        context_model         TEXT
    ) STRICT;
    CREATE INDEX sessions_updated ON sessions(updated_at DESC);

    CREATE TABLE session_meta (
        session_id  TEXT PRIMARY KEY,
        custom_name TEXT,
        pinned      INTEGER NOT NULL DEFAULT 0,
        pin_order   INTEGER,
        category    TEXT
    ) STRICT;

    CREATE TABLE tags (
        id   INTEGER PRIMARY KEY,
        name TEXT NOT NULL UNIQUE COLLATE NOCASE
    ) STRICT;

    CREATE TABLE session_tags (
        session_id TEXT NOT NULL,
        tag_id     INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
        PRIMARY KEY (session_id, tag_id)
    ) STRICT;

    CREATE TABLE file_edits (
        id              INTEGER PRIMARY KEY,
        session_id      TEXT NOT NULL,
        tool_use_id     TEXT NOT NULL UNIQUE,
        timestamp       INTEGER,
        tool            TEXT NOT NULL,
        file_path       TEXT NOT NULL,
        new_start       INTEGER,
        new_end         INTEGER,
        added           INTEGER NOT NULL,
        removed         INTEGER NOT NULL,
        is_new_file     INTEGER NOT NULL DEFAULT 0,
        status          TEXT NOT NULL DEFAULT 'pending',
        transcript_path TEXT NOT NULL,
        line_offset     INTEGER NOT NULL,
        line_length     INTEGER NOT NULL,
        why             TEXT,
        from_subagent   INTEGER NOT NULL DEFAULT 0
    ) STRICT;
    CREATE INDEX file_edits_session ON file_edits(session_id, file_path);

    CREATE TABLE activity (
        id            INTEGER PRIMARY KEY,
        session_id    TEXT NOT NULL,
        timestamp     INTEGER,
        kind          TEXT NOT NULL,
        tool          TEXT,
        target        TEXT,
        edit_id       INTEGER,
        from_subagent INTEGER NOT NULL DEFAULT 0
    ) STRICT;
    CREATE INDEX activity_session ON activity(session_id, id);

    CREATE TABLE projects (
        cwd       TEXT PRIMARY KEY,
        name      TEXT NOT NULL,
        repo_root TEXT
    ) STRICT;

    CREATE VIRTUAL TABLE session_fts USING fts5(
        session_id UNINDEXED,
        body,
        tokenize = 'unicode61 remove_diacritics 2'
    );
";

pub fn apply(connection: &Connection) -> Result<(), AppError> {
    apply_steps(connection, MIGRATIONS)
}

/// Each step runs in its own transaction together with the version bump, so a failure leaves
/// neither a half-built schema nor a version that lies about it.
fn apply_steps(connection: &Connection, steps: &[&str]) -> Result<(), AppError> {
    let applied: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let applied = usize::try_from(applied).unwrap_or(0);
    for (index, statement) in steps.iter().enumerate().skip(applied) {
        let version = i64::try_from(index + 1).unwrap_or(i64::MAX);
        connection.execute_batch(&format!("BEGIN; {statement}; PRAGMA user_version = {version}; COMMIT;")).inspect_err(|_| {
            let _ = connection.execute_batch("ROLLBACK;");
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_exists(connection: &Connection, name: &str) -> bool {
        connection
            .query_row("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1", [name], |row| row.get::<_, i64>(0))
            .unwrap()
            > 0
    }

    #[test]
    fn a_failing_step_leaves_no_partial_schema() {
        let connection = Connection::open_in_memory().unwrap();
        let steps = ["CREATE TABLE first (id INTEGER) STRICT;", "CREATE TABLE second (id INTEGER) STRICT; CREATE TABLE broken (;"];
        assert!(apply_steps(&connection, &steps).is_err());
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
        assert_eq!(version, 1);
        assert!(table_exists(&connection, "first"));
        assert!(!table_exists(&connection, "second"), "the failed step rolled back entirely");
        let fixed = ["CREATE TABLE first (id INTEGER) STRICT;", "CREATE TABLE second (id INTEGER) STRICT;"];
        apply_steps(&connection, &fixed).unwrap();
        assert!(table_exists(&connection, "second"));
    }

    #[test]
    fn upgrading_keeps_user_data_and_rebuilds_derived_data() {
        let connection = Connection::open_in_memory().unwrap();
        apply_steps(&connection, &MIGRATIONS[..3]).unwrap();
        connection.execute_batch(
            "INSERT INTO session_meta (session_id, custom_name, pinned) VALUES ('s', 'Minha', 1);
             INSERT INTO tags (name) VALUES ('review');
             INSERT INTO sessions (id, project_dir) VALUES ('s', 'p');
             INSERT INTO transcript_files (path, session_id, is_subagent, inode, size, byte_offset, modified_ms) VALUES ('/t', 's', 0, 1, 1, 1, 1);",
        )
        .unwrap();
        apply(&connection).unwrap();
        let name: String = connection.query_row("SELECT custom_name FROM session_meta WHERE session_id = 's'", [], |row| row.get(0)).unwrap();
        assert_eq!(name, "Minha");
        assert_eq!(connection.query_row("SELECT COUNT(*) FROM tags", [], |row| row.get::<_, i64>(0)).unwrap(), 1);
        assert_eq!(connection.query_row("SELECT COUNT(*) FROM transcript_files", [], |row| row.get::<_, i64>(0)).unwrap(), 0, "re-read on next start");
        assert!(table_exists(&connection, "grid_order"));
    }

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
