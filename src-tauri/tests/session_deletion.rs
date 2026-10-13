//! The one exception to the read-only `~/.claude`: moving a session's conversation to the Trash when
//! the user asks for it. Nothing else under `~/.claude` may change.

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use ccm_lib::db::Database;
use ccm_lib::error::AppError;
use ccm_lib::library::deletion::{Discard, delete_conversation};
use ccm_lib::library::indexer::Indexer;
use ccm_lib::library::queries;
use support::fixtures::{self, DEMO_PROJECT_DIR, DEMO_SESSION_ID};

const OTHER_SESSION_ID: &str = "22222222-2222-4222-8222-222222222222";

/// Stands in for the system Trash, so tests never touch the user's real one.
struct FolderTrash(PathBuf);

impl Discard for FolderTrash {
    fn discard(&self, path: &Path) -> std::io::Result<()> {
        fs::create_dir_all(&self.0)?;
        fs::rename(path, self.0.join(path.file_name().expect("named path")))
    }
}

struct Library {
    _root: tempfile::TempDir,
    claude_home: PathBuf,
    project: PathBuf,
    database: Database,
    trash: FolderTrash,
}

impl Library {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let claude_home = root.path().join(".claude");
        let projects = claude_home.join("projects");
        fixtures::install_demo_session(&projects);
        let project = projects.join(DEMO_PROJECT_DIR);
        let other = fs::read_to_string(project.join(format!("{DEMO_SESSION_ID}.jsonl"))).unwrap().replace(DEMO_SESSION_ID, OTHER_SESSION_ID);
        fs::write(project.join(format!("{OTHER_SESSION_ID}.jsonl")), other).unwrap();
        fs::write(claude_home.join("settings.json"), "{}").unwrap();
        let database_file = root.path().join("ccm.sqlite");
        let indexer = Indexer::new(Database::open(&database_file).unwrap(), projects);
        for transcript in indexer.discover() {
            indexer.index_file(&transcript).unwrap();
        }
        let trash = FolderTrash(root.path().join("Trash"));
        Self { claude_home, project, database: Database::open(&database_file).unwrap(), trash, _root: root }
    }

    fn session_ids(&self) -> Vec<String> {
        queries::list_sessions(&self.database.connection()).unwrap().into_iter().map(|item| item.id).collect()
    }

    fn delete(&self, session_id: &str, open: bool) -> Result<(), AppError> {
        delete_conversation(&self.database, &self.claude_home, session_id, open, &self.trash)
    }
}

#[test]
fn deleting_a_conversation_moves_only_its_files_to_the_trash() {
    let library = Library::new();
    queries::rename_session(&library.database.connection(), DEMO_SESSION_ID, Some("nome dado no app")).unwrap();
    assert_eq!(library.session_ids().len(), 2);

    library.delete(DEMO_SESSION_ID, false).unwrap();

    assert!(!library.project.join(format!("{DEMO_SESSION_ID}.jsonl")).exists());
    assert!(!library.project.join(DEMO_SESSION_ID).exists(), "the subagents folder goes too");
    assert!(library.trash.0.join(format!("{DEMO_SESSION_ID}.jsonl")).exists());
    assert!(library.trash.0.join(DEMO_SESSION_ID).join("subagents/agent-a1.jsonl").exists());
    assert!(library.project.join(format!("{OTHER_SESSION_ID}.jsonl")).exists(), "other conversations stay");
    assert!(library.claude_home.join("settings.json").exists());
    assert_eq!(library.session_ids(), vec![OTHER_SESSION_ID.to_owned()]);
}

#[test]
fn an_open_session_is_never_deleted() {
    let library = Library::new();
    assert!(matches!(library.delete(DEMO_SESSION_ID, true), Err(AppError::Invalid(_))));
    assert!(library.project.join(format!("{DEMO_SESSION_ID}.jsonl")).exists());
    assert_eq!(library.session_ids().len(), 2);
}

#[test]
fn transcripts_outside_a_project_folder_are_refused() {
    let library = Library::new();
    let set_path = |path: &Path| {
        library.database.connection().execute("UPDATE sessions SET transcript_path = ?1 WHERE id = ?2", (path.to_string_lossy().into_owned(), DEMO_SESSION_ID)).unwrap();
    };
    let stray = library.claude_home.join(format!("{DEMO_SESSION_ID}.jsonl"));
    fs::write(&stray, "{}").unwrap();
    for hostile in [library.claude_home.join("settings.json"), stray.clone(), library.claude_home.join("projects").join(format!("{DEMO_SESSION_ID}.jsonl"))] {
        set_path(&hostile);
        assert!(library.delete(DEMO_SESSION_ID, false).is_err(), "{}", hostile.display());
    }
    assert!(stray.exists());
    assert!(library.claude_home.join("settings.json").exists());
    assert!(!library.trash.0.exists(), "nothing reached the trash");
}

#[test]
fn symlinks_are_refused() {
    let library = Library::new();
    let transcript = library.project.join(format!("{DEMO_SESSION_ID}.jsonl"));
    let elsewhere = library.claude_home.join("settings.json");
    fs::remove_file(&transcript).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &transcript).unwrap();
    assert!(library.delete(DEMO_SESSION_ID, false).is_err());
    assert!(elsewhere.exists());
    assert!(!library.trash.0.exists());
}
