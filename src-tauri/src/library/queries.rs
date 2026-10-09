//! Read side of the library plus the user-owned metadata (name, pin, tags, category).

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::models::{FileTouched, MatchSource, SearchHit, SessionListItem, SessionSummary};
use crate::context;
use crate::db::from_sql_int;
use crate::error::AppError;
use crate::transcript::tools::truncate_chars;
use crate::transcript::usage::TokenUsage;

const TAG_SEPARATOR: char = '\u{1f}';
const TITLE_MAX_CHARS: usize = 90;
const MAX_TAGS: usize = 12;
const TAG_MAX_CHARS: usize = 32;
const SEARCH_LIMIT: usize = 100;
const UNTITLED: &str = "Sessão sem título";

const LIST_SELECT: &str = "
    SELECT s.id, m.custom_name, s.ai_title, s.first_prompt, COALESCE(p.name, s.project_dir), s.cwd, s.git_branch,
           s.started_at, s.updated_at, COALESCE(m.pinned, 0), m.pin_order, m.category, s.cost_usd,
           (SELECT COUNT(DISTINCT e.file_path) FROM file_edits e WHERE e.session_id = s.id AND e.status <> 'failed'),
           (SELECT group_concat(t.name, char(31)) FROM session_tags st JOIN tags t ON t.id = st.tag_id
             WHERE st.session_id = s.id)
    FROM sessions s
    LEFT JOIN session_meta m ON m.session_id = s.id
    LEFT JOIN projects p ON p.cwd = s.cwd
    WHERE (s.first_prompt IS NOT NULL OR s.ai_title IS NOT NULL OR m.custom_name IS NOT NULL)";

pub fn display_title(custom_name: Option<&str>, ai_title: Option<&str>, first_prompt: Option<&str>) -> String {
    [custom_name, ai_title, first_prompt.and_then(|prompt| prompt.lines().find(|line| !line.trim().is_empty()))]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|candidate| !candidate.is_empty())
        .map(|title| truncate_chars(title, TITLE_MAX_CHARS))
        .unwrap_or_else(|| UNTITLED.to_owned())
}

fn list_item(row: &Row<'_>) -> rusqlite::Result<SessionListItem> {
    let custom_name: Option<String> = row.get(1)?;
    let ai_title: Option<String> = row.get(2)?;
    let first_prompt: Option<String> = row.get(3)?;
    let tags: Option<String> = row.get(14)?;
    Ok(SessionListItem {
        id: row.get(0)?,
        title: display_title(custom_name.as_deref(), ai_title.as_deref(), first_prompt.as_deref()),
        custom_name,
        first_prompt: first_prompt.map(|prompt| truncate_chars(&prompt, 400)),
        project: row.get(4)?,
        cwd: row.get(5)?,
        branch: row.get(6)?,
        started_at: row.get(7)?,
        updated_at: row.get(8)?,
        pinned: row.get::<_, i64>(9)? != 0,
        pin_order: row.get(10)?,
        category: row.get(11)?,
        cost_usd: row.get(12)?,
        files_count: u32::try_from(row.get::<_, i64>(13)?).unwrap_or(0),
        tags: tags.map(|joined| joined.split(TAG_SEPARATOR).map(str::to_owned).collect()).unwrap_or_default(),
    })
}

pub fn list_sessions(connection: &Connection) -> Result<Vec<SessionListItem>, AppError> {
    let mut statement = connection.prepare(&format!("{LIST_SELECT} ORDER BY s.updated_at DESC"))?;
    let rows = statement.query_map([], list_item)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn find_item(connection: &Connection, session_id: &str) -> Result<Option<SessionListItem>, AppError> {
    let item = connection
        .query_row(&format!("{LIST_SELECT} AND s.id = ?1"), params![session_id], list_item)
        .optional()?;
    Ok(item)
}

struct SessionDetails {
    ai_title: Option<String>,
    last_prompt: Option<String>,
    last_assistant: Option<String>,
    repo_root: Option<String>,
    model: Option<String>,
    pr_url: Option<String>,
    usage: TokenUsage,
    unknown_pricing: bool,
    context_tokens: Option<u64>,
    context_model: Option<String>,
}

fn session_details(connection: &Connection, session_id: &str) -> Result<SessionDetails, AppError> {
    let details = connection.query_row(
        "SELECT s.ai_title, s.last_prompt, s.last_assistant, p.repo_root, s.model, s.pr_url,
                s.input_tokens, s.output_tokens, s.cache_read_tokens, s.cache_write_5m_tokens, s.cache_write_1h_tokens,
                s.unknown_pricing, s.context_tokens, s.context_model
         FROM sessions s LEFT JOIN projects p ON p.cwd = s.cwd WHERE s.id = ?1",
        params![session_id],
        |row| {
            Ok(SessionDetails {
                ai_title: row.get(0)?,
                last_prompt: row.get(1)?,
                last_assistant: row.get(2)?,
                repo_root: row.get(3)?,
                model: row.get(4)?,
                pr_url: row.get(5)?,
                usage: TokenUsage {
                    input: from_sql_int(row.get(6)?),
                    output: from_sql_int(row.get(7)?),
                    cache_read: from_sql_int(row.get(8)?),
                    cache_write_5m: from_sql_int(row.get(9)?),
                    cache_write_1h: from_sql_int(row.get(10)?),
                },
                unknown_pricing: row.get::<_, i64>(11)? != 0,
                context_tokens: row.get::<_, Option<i64>>(12)?.map(from_sql_int),
                context_model: row.get(13)?,
            })
        },
    )?;
    Ok(details)
}

pub fn session_summary(connection: &Connection, session_id: &str) -> Result<Option<SessionSummary>, AppError> {
    let Some(item) = find_item(connection, session_id)? else { return Ok(None) };
    let details = session_details(connection, session_id)?;
    let context_window = context::window_for(details.context_model.as_deref(), details.context_tokens.unwrap_or(0));
    Ok(Some(SessionSummary {
        cwd_exists: item.cwd.as_deref().is_some_and(|cwd| Path::new(cwd).is_dir()),
        duration_ms: item.started_at.zip(item.updated_at).map(|(start, end)| (end - start).max(0)),
        files: files_touched(connection, session_id)?,
        context_percent: details.context_tokens.map(|tokens| context::used_percent(tokens, context_window)),
        item,
        ai_title: details.ai_title,
        last_prompt: details.last_prompt,
        last_assistant: details.last_assistant,
        repo_root: details.repo_root,
        model: details.model,
        pr_url: details.pr_url,
        usage: details.usage,
        unknown_pricing: details.unknown_pricing,
        context_tokens: details.context_tokens,
        context_window,
    }))
}

pub fn files_touched(connection: &Connection, session_id: &str) -> Result<Vec<FileTouched>, AppError> {
    let mut statement = connection.prepare(
        "SELECT file_path, SUM(added), SUM(removed), COUNT(*), MAX(is_new_file)
         FROM file_edits WHERE session_id = ?1 AND status <> 'failed'
         GROUP BY file_path ORDER BY MAX(id) DESC",
    )?;
    let rows = statement.query_map(params![session_id], |row| {
        Ok(FileTouched {
            path: row.get(0)?,
            added: u32::try_from(row.get::<_, i64>(1)?).unwrap_or(0),
            removed: u32::try_from(row.get::<_, i64>(2)?).unwrap_or(0),
            edits: u32::try_from(row.get::<_, i64>(3)?).unwrap_or(0),
            is_new_file: row.get::<_, i64>(4)? != 0,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn search(connection: &Connection, query: &str) -> Result<Vec<SearchHit>, AppError> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let mut hits = metadata_hits(connection, query)?;
    hits.extend(text_hits(connection, query)?);
    let mut seen = std::collections::HashSet::new();
    hits.retain(|hit| seen.insert(hit.session_id.clone()));
    hits.truncate(SEARCH_LIMIT);
    Ok(hits)
}

fn metadata_hits(connection: &Connection, query: &str) -> Result<Vec<SearchHit>, AppError> {
    let pattern = format!("%{}%", escape_like(query));
    let checks: [(MatchSource, &str); 5] = [
        (MatchSource::Title, "m.custom_name LIKE ?1 ESCAPE '\\' OR s.ai_title LIKE ?1 ESCAPE '\\' OR s.first_prompt LIKE ?1 ESCAPE '\\'"),
        (MatchSource::Project, "p.name LIKE ?1 ESCAPE '\\' OR s.cwd LIKE ?1 ESCAPE '\\'"),
        (MatchSource::Branch, "s.git_branch LIKE ?1 ESCAPE '\\'"),
        (MatchSource::Tag, "EXISTS (SELECT 1 FROM session_tags st JOIN tags t ON t.id = st.tag_id WHERE st.session_id = s.id AND t.name LIKE ?1 ESCAPE '\\')"),
        (MatchSource::File, "EXISTS (SELECT 1 FROM file_edits e WHERE e.session_id = s.id AND e.file_path LIKE ?1 ESCAPE '\\')"),
    ];
    let mut hits = Vec::new();
    for (source, condition) in checks {
        let sql = format!("SELECT s.id FROM ({LIST_SELECT}) visible JOIN sessions s ON s.id = visible.id
             LEFT JOIN session_meta m ON m.session_id = s.id LEFT JOIN projects p ON p.cwd = s.cwd
             WHERE {condition} ORDER BY s.updated_at DESC LIMIT {SEARCH_LIMIT}");
        let mut statement = connection.prepare(&sql)?;
        let ids = statement.query_map(params![pattern], |row| row.get::<_, String>(0))?;
        for id in ids {
            hits.push(SearchHit { session_id: id?, matched_in: source, snippet: None });
        }
    }
    Ok(hits)
}

fn text_hits(connection: &Connection, query: &str) -> Result<Vec<SearchHit>, AppError> {
    let Some(expression) = fts_expression(query) else { return Ok(Vec::new()) };
    let mut statement = connection.prepare(
        "SELECT session_id, snippet(session_fts, 1, '«', '»', '…', 12) FROM session_fts
         WHERE session_fts MATCH ?1 ORDER BY rank LIMIT 400",
    )?;
    let rows = statement.query_map(params![expression], |row| {
        Ok(SearchHit { session_id: row.get(0)?, matched_in: MatchSource::Text, snippet: row.get(1)? })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Every word must match as a prefix; quoting keeps FTS operators typed by the user inert.
pub fn fts_expression(query: &str) -> Option<String> {
    let terms: Vec<String> = query
        .split(|character: char| !character.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .take(8)
        .map(|term| format!("\"{term}\"*"))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" "))
}

fn escape_like(text: &str) -> String {
    text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

fn ensure_meta_row(connection: &Connection, session_id: &str) -> Result<(), AppError> {
    connection.execute("INSERT OR IGNORE INTO session_meta (session_id) VALUES (?1)", params![session_id])?;
    Ok(())
}

pub fn rename_session(connection: &Connection, session_id: &str, name: Option<&str>) -> Result<(), AppError> {
    ensure_meta_row(connection, session_id)?;
    let name = name.map(str::trim).filter(|name| !name.is_empty());
    connection.execute("UPDATE session_meta SET custom_name = ?2 WHERE session_id = ?1", params![session_id, name])?;
    Ok(())
}

pub fn set_pinned(connection: &Connection, session_id: &str, pinned: bool) -> Result<(), AppError> {
    ensure_meta_row(connection, session_id)?;
    connection.execute(
        "UPDATE session_meta SET pinned = ?2,
             pin_order = CASE WHEN ?2 THEN (SELECT COALESCE(MAX(pin_order), 0) + 1 FROM session_meta) ELSE NULL END
         WHERE session_id = ?1",
        params![session_id, pinned],
    )?;
    Ok(())
}

pub fn set_category(connection: &Connection, session_id: &str, category: Option<&str>) -> Result<(), AppError> {
    ensure_meta_row(connection, session_id)?;
    let category = category.map(str::trim).filter(|category| !category.is_empty());
    connection.execute("UPDATE session_meta SET category = ?2 WHERE session_id = ?1", params![session_id, category])?;
    Ok(())
}

pub fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut normalized: Vec<String> = Vec::new();
    for tag in tags.iter().map(|tag| truncate_chars(tag.trim(), TAG_MAX_CHARS)).filter(|tag| !tag.is_empty()) {
        if !normalized.iter().any(|existing| existing.to_lowercase() == tag.to_lowercase()) {
            normalized.push(tag);
        }
    }
    normalized.truncate(MAX_TAGS);
    normalized
}

pub fn set_tags(connection: &mut Connection, session_id: &str, tags: &[String]) -> Result<Vec<String>, AppError> {
    let tags = normalize_tags(tags);
    let transaction = connection.transaction()?;
    transaction.execute("DELETE FROM session_tags WHERE session_id = ?1", params![session_id])?;
    for tag in &tags {
        transaction.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", params![tag])?;
        transaction.execute(
            "INSERT OR IGNORE INTO session_tags (session_id, tag_id) SELECT ?1, id FROM tags WHERE name = ?2",
            params![session_id, tag],
        )?;
    }
    transaction.execute("DELETE FROM tags WHERE id NOT IN (SELECT tag_id FROM session_tags)", [])?;
    transaction.commit()?;
    Ok(tags)
}

pub fn list_tags(connection: &Connection) -> Result<Vec<String>, AppError> {
    let mut statement = connection.prepare("SELECT name FROM tags ORDER BY name COLLATE NOCASE")?;
    let rows = statement.query_map([], |row| row.get(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn list_categories(connection: &Connection) -> Result<Vec<String>, AppError> {
    let mut statement = connection.prepare(
        "SELECT DISTINCT category FROM session_meta WHERE category IS NOT NULL ORDER BY category COLLATE NOCASE",
    )?;
    let rows = statement.query_map([], |row| row.get(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_prefers_custom_then_ai_then_first_prompt_line() {
        assert_eq!(display_title(Some("Mine"), Some("AI"), Some("prompt")), "Mine");
        assert_eq!(display_title(None, Some("AI"), Some("prompt")), "AI");
        assert_eq!(display_title(Some("  "), None, Some("\nFirst line\nsecond")), "First line");
        assert_eq!(display_title(None, None, None), UNTITLED);
    }

    #[test]
    fn fts_expression_quotes_terms_as_prefixes() {
        assert_eq!(fts_expression("pagina pedidos").as_deref(), Some("\"pagina\"* \"pedidos\"*"));
        assert_eq!(fts_expression("NEAR(\"x\" OR y)").as_deref(), Some("\"NEAR\"* \"x\"* \"OR\"* \"y\"*"));
        assert_eq!(fts_expression("  -- ").as_deref(), None);
    }

    #[test]
    fn tags_are_trimmed_deduplicated_and_capped() {
        let raw: Vec<String> = ["bug", " Bug ", "", "review"].iter().map(|tag| (*tag).to_owned()).collect();
        assert_eq!(normalize_tags(&raw), vec!["bug".to_owned(), "review".to_owned()]);
        let many: Vec<String> = (0..20).map(|index| format!("t{index}")).collect();
        assert_eq!(normalize_tags(&many).len(), MAX_TAGS);
    }

    #[test]
    fn like_wildcards_are_escaped() {
        assert_eq!(escape_like("50%_a\\b"), "50\\%\\_a\\\\b");
    }
}
