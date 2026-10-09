use rusqlite::Connection;

use crate::error::AppError;

/// Ordered schema steps; `PRAGMA user_version` records how many were applied.
const MIGRATIONS: &[&str] = &[
    "CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    ) STRICT;",
    LIBRARY_SCHEMA,
];

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
