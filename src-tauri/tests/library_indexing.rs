//! Indexing the synthetic transcripts end to end: totals, edits, search and incremental reads.

mod support;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use ccm_lib::db::Database;
use ccm_lib::library::indexer::Indexer;
use ccm_lib::library::models::MatchSource;
use ccm_lib::library::queries;
use support::fixtures::{self, DEMO_SESSION_ID};
use tempfile::TempDir;

struct Workspace {
    _root: TempDir,
    projects: PathBuf,
    database_file: PathBuf,
}

impl Workspace {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let projects = root.path().join("projects");
        fs::create_dir_all(&projects).unwrap();
        let database_file = root.path().join("ccm.sqlite");
        Self { _root: root, projects, database_file }
    }

    fn indexer(&self) -> Indexer {
        Indexer::new(Database::open(&self.database_file).unwrap(), self.projects.clone())
    }

    fn reader(&self) -> Database {
        Database::open(&self.database_file).unwrap()
    }

    fn index_everything(&self) {
        let indexer = self.indexer();
        indexer.index_paths(&indexer.discover()).unwrap();
    }
}

/// Sonnet 5 main thread ($2/$10, cache read $0.20) plus a Haiku 4.5 subagent ($1/$5).
const EXPECTED_COST_USD: f64 = 0.004_855;

fn assert_demo_totals(database: &Database) {
    let summary = queries::session_summary(&database.connection(), DEMO_SESSION_ID).unwrap().expect("session");
    assert_eq!(summary.usage.input, 31);
    assert_eq!(summary.usage.output, 280);
    assert_eq!(summary.usage.cache_read, 6_750);
    assert_eq!((summary.usage.cache_write_5m, summary.usage.cache_write_1h), (200, 100));
    assert!((summary.item.cost_usd - EXPECTED_COST_USD).abs() < 1e-9, "cost {}", summary.item.cost_usd);
    assert_eq!(summary.context_tokens, Some(1_462));
    assert_eq!(summary.item.files_count, 3);
}

#[test]
fn indexes_summary_fields_from_transcript() {
    let workspace = Workspace::new();
    fixtures::install_demo_session(&workspace.projects);
    workspace.index_everything();

    let database = workspace.reader();
    let summary = queries::session_summary(&database.connection(), DEMO_SESSION_ID).unwrap().unwrap();
    assert_eq!(summary.item.title, "Paginação na listagem de pedidos");
    assert_eq!(summary.item.first_prompt.as_deref(), Some("Adicione paginação na listagem de pedidos"));
    assert_eq!(summary.item.branch.as_deref(), Some("feat-orders-pagination"));
    assert_eq!(summary.item.project, "demo-app");
    assert_eq!(summary.last_assistant.as_deref(), Some("Pronto: paginação feita e testes passando."));
    assert_eq!(summary.pr_url.as_deref(), Some("https://example.com/acme/demo-app/pull/7"));
    assert_eq!(summary.duration_ms, Some(45_000));
    assert!(!summary.cwd_exists);
    assert_demo_totals(&database);
}

#[test]
fn counts_each_response_once_and_skips_rejected_edits() {
    let workspace = Workspace::new();
    fixtures::install_demo_session(&workspace.projects);
    workspace.index_everything();

    let database = workspace.reader();
    let files = queries::files_touched(&database.connection(), DEMO_SESSION_ID).unwrap();
    let by_name = |suffix: &str| files.iter().find(|file| file.path.ends_with(suffix)).cloned();
    let hook = by_name("use-orders.ts").expect("edited hook");
    assert_eq!((hook.added, hook.removed, hook.is_new_file), (3, 1, false));
    let spec = by_name("use-orders.spec.ts").expect("created spec");
    assert_eq!((spec.added, spec.removed, spec.is_new_file), (3, 0, true));
    let guard = by_name("guards.ts").expect("subagent edit");
    assert_eq!((guard.added, guard.removed), (3, 0));
    assert!(by_name("README.md").is_none(), "rejected edit must not count");
}

#[test]
fn stores_the_assistant_text_before_each_edit_as_why() {
    let workspace = Workspace::new();
    fixtures::install_demo_session(&workspace.projects);
    workspace.index_everything();

    let database = workspace.reader();
    let connection = database.connection();
    let why = |tool_use_id: &str| -> Option<String> {
        connection
            .query_row("SELECT why FROM file_edits WHERE tool_use_id = ?1", [tool_use_id], |row| row.get(0))
            .unwrap()
    };
    assert_eq!(why("toolu_1").as_deref(), Some("Vou adicionar a paginação no hook de pedidos."));
    assert_eq!(why("toolu_2").as_deref(), Some("Agora crio o teste do hook."));
    assert_eq!(why("toolu_9").as_deref(), Some("Adicionando guarda para página negativa."));
    let range: (i64, i64) = connection
        .query_row("SELECT new_start, new_end FROM file_edits WHERE tool_use_id = 'toolu_1'", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert_eq!(range, (20, 25));
}

#[test]
fn search_matches_text_without_accents_and_metadata() {
    let workspace = Workspace::new();
    fixtures::install_demo_session(&workspace.projects);
    workspace.index_everything();

    let database = workspace.reader();
    let connection = database.connection();
    let text = queries::search(&connection, "testes passando").unwrap();
    assert_eq!(text.first().map(|hit| hit.matched_in), Some(MatchSource::Text));
    let unaccented = queries::search(&connection, "paginacao").unwrap();
    assert_eq!(unaccented.first().map(|hit| hit.session_id.as_str()), Some(DEMO_SESSION_ID));
    let branch = queries::search(&connection, "orders-pagination").unwrap();
    assert!(branch.iter().any(|hit| hit.matched_in == MatchSource::Branch));
    let file = queries::search(&connection, "guards.ts").unwrap();
    assert!(file.iter().any(|hit| hit.matched_in == MatchSource::File));
    assert!(queries::search(&connection, "inexistente").unwrap().is_empty());
}

#[test]
fn incremental_reads_match_a_single_full_pass() {
    let workspace = Workspace::new();
    let demo = fixtures::install_demo_session(&workspace.projects);
    let lines = fixtures::main_fixture_lines();
    let (first_half, second_half) = lines.split_at(lines.len() / 2);
    fs::write(&demo.main_transcript, format!("{}\n{}", first_half.join("\n"), "{\"type\":\"assi")).unwrap();
    workspace.index_everything();

    fs::write(&demo.main_transcript, format!("{}\n", first_half.join("\n"))).unwrap();
    let mut file = fs::OpenOptions::new().append(true).open(&demo.main_transcript).unwrap();
    file.write_all(format!("{}\n", second_half.join("\n")).as_bytes()).unwrap();
    drop(file);
    let indexer = workspace.indexer();
    assert!(indexer.index_file(&demo.main_transcript).unwrap());
    assert!(!indexer.index_file(&demo.main_transcript).unwrap(), "unchanged file is skipped");

    assert_demo_totals(&workspace.reader());
}

#[test]
fn truncated_transcript_is_reindexed_without_double_counting() {
    let workspace = Workspace::new();
    let demo = fixtures::install_demo_session(&workspace.projects);
    workspace.index_everything();

    let original = fs::read_to_string(&demo.main_transcript).unwrap();
    fs::write(&demo.main_transcript, original.lines().take(3).map(|line| format!("{line}\n")).collect::<String>()).unwrap();
    workspace.indexer().index_file(&demo.main_transcript).unwrap();
    fs::write(&demo.main_transcript, &original).unwrap();
    workspace.indexer().index_file(&demo.main_transcript).unwrap();

    assert_demo_totals(&workspace.reader());
}

#[test]
fn user_metadata_survives_reindex_and_deleted_sessions_leave() {
    let workspace = Workspace::new();
    let demo = fixtures::install_demo_session(&workspace.projects);
    workspace.index_everything();
    {
        let database = workspace.reader();
        let mut connection = database.connection();
        queries::rename_session(&connection, DEMO_SESSION_ID, Some("Minha sessão")).unwrap();
        queries::set_pinned(&connection, DEMO_SESSION_ID, true).unwrap();
        queries::set_tags(&mut connection, DEMO_SESSION_ID, &["review".to_owned(), "Review".to_owned()]).unwrap();
    }
    let indexer = workspace.indexer();
    indexer.reset_session(DEMO_SESSION_ID).unwrap();
    indexer.index_paths(&indexer.discover()).unwrap();

    let database = workspace.reader();
    let items = queries::list_sessions(&database.connection()).unwrap();
    let item = items.iter().find(|item| item.id == DEMO_SESSION_ID).expect("listed");
    assert_eq!((item.title.as_str(), item.pinned), ("Minha sessão", true));
    assert_eq!(item.tags, vec!["review".to_owned()]);

    fs::remove_file(&demo.main_transcript).unwrap();
    assert_eq!(indexer.prune_missing().unwrap(), 1);
    assert!(queries::list_sessions(&database.connection()).unwrap().is_empty());
}

#[test]
fn ignores_files_outside_the_known_layout() {
    let workspace = Workspace::new();
    let stray: &Path = &workspace.projects.join("notes.jsonl");
    fs::write(stray, "{}\n").unwrap();
    assert!(!workspace.indexer().index_file(stray).unwrap());
}

#[test]
fn edit_details_rebuild_the_diff_from_the_transcript_line() {
    use ccm_lib::library::details;
    use ccm_lib::library::diff::DiffLineKind;

    let workspace = Workspace::new();
    fixtures::install_demo_session(&workspace.projects);
    workspace.index_everything();
    let database = workspace.reader();
    let connection = database.connection();

    let hook = "/tmp/ccm-fixture/demo-app/src/orders/use-orders.ts";
    let edits = details::file_edits(&connection, DEMO_SESSION_ID, hook).unwrap();
    assert_eq!(edits.len(), 1);
    let edit = &edits[0];
    assert_eq!(edit.why.as_deref(), Some("Vou adicionar a paginação no hook de pedidos."));
    assert_eq!((edit.new_start, edit.new_end), (Some(20), Some(25)));
    let removed: Vec<&str> = edit.lines.iter().filter(|line| line.kind == DiffLineKind::Removed).map(|line| line.text.as_str()).collect();
    let added: Vec<&str> = edit.lines.iter().filter(|line| line.kind == DiffLineKind::Added).map(|line| line.text.as_str()).collect();
    assert_eq!(removed, vec!["const page = 1;"]);
    assert_eq!(added, vec!["const page = params.page;", "const size = 20;", "const offset = page * size;"]);

    let spec = details::file_edits(&connection, DEMO_SESSION_ID, "/tmp/ccm-fixture/demo-app/src/orders/use-orders.spec.ts").unwrap();
    assert!(spec[0].is_new_file);
    assert!(spec[0].lines.iter().all(|line| line.kind == DiffLineKind::Added));
    assert_eq!(details::edit_by_id(&connection, edit.id).unwrap().map(|detail| detail.lines.len()), Some(edit.lines.len()));

    let subagent = details::file_edits(&connection, DEMO_SESSION_ID, "/tmp/ccm-fixture/demo-app/src/orders/guards.ts").unwrap();
    assert_eq!(subagent[0].lines.iter().filter(|line| line.kind == DiffLineKind::Added).count(), 3);
}

#[test]
fn activity_feed_lists_prompts_tools_and_edits_newest_first() {
    use ccm_lib::library::details;

    let workspace = Workspace::new();
    fixtures::install_demo_session(&workspace.projects);
    workspace.index_everything();
    let database = workspace.reader();
    let activity = details::activity(&database.connection(), DEMO_SESSION_ID, 50).unwrap();
    let kinds: Vec<(&str, Option<&str>)> = activity.iter().map(|item| (item.kind.as_str(), item.target.as_deref())).collect();
    assert!(kinds.contains(&("prompt", Some("Adicione paginação na listagem de pedidos"))));
    assert!(kinds.contains(&("tool", Some("npm test -- orders"))));
    assert!(activity.iter().any(|item| item.kind == "edit" && item.edit_id.is_some()));
    assert!(activity.iter().any(|item| item.from_subagent));
    assert!(activity.windows(2).all(|pair| pair[0].id > pair[1].id));
}
