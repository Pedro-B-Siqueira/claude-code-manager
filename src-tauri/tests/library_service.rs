//! The background library service end to end: initial scan, live updates through FSEvents, and
//! a projects folder that only appears after the app started.

mod support;

use std::fs;
use std::io::Write;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ccm_lib::library::models::IndexProgress;
use ccm_lib::library::service::{self, LibraryNotice};
use ccm_lib::paths::AppPaths;
use support::fixtures;

const WAIT: Duration = Duration::from_secs(15);

struct Running {
    _root: tempfile::TempDir,
    claude_home: std::path::PathBuf,
    progress: Arc<Mutex<IndexProgress>>,
    notices: mpsc::Receiver<LibraryNotice>,
}

fn start(create_projects: bool) -> Running {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let claude_home = home.join(".claude");
    fs::create_dir_all(&claude_home).unwrap();
    if create_projects {
        fs::create_dir_all(claude_home.join("projects")).unwrap();
    }
    let paths = AppPaths::new(home.clone(), claude_home.clone(), home.join("app"));
    paths.create_app_support().unwrap();
    ccm_lib::db::Database::open(&paths.database_file()).unwrap();
    let (sender, notices) = mpsc::channel();
    let sender = Mutex::new(sender);
    let progress: Arc<Mutex<IndexProgress>> = Arc::default();
    service::start_with_poll(&paths, Arc::clone(&progress), Arc::new(move |notice| {
        let _ = sender.lock().unwrap().send(notice);
    }), Duration::from_millis(200))
    .unwrap();
    Running { _root: root, claude_home, progress, notices }
}

fn wait_for_change(running: &Running) {
    loop {
        match running.notices.recv_timeout(WAIT).expect("library notice") {
            LibraryNotice::Changed => return,
            LibraryNotice::Progress(_) => {}
        }
    }
}

#[test]
fn scans_then_follows_appended_lines() {
    let running = start(true);
    let demo = fixtures::install_demo_session(&running.claude_home.join("projects"));
    wait_for_change(&running);
    std::thread::sleep(Duration::from_millis(500));
    while running.notices.try_recv().is_ok() {}
    assert!(!running.progress.lock().unwrap().running);

    let mut file = fs::OpenOptions::new().append(true).open(&demo.main_transcript).unwrap();
    writeln!(file, r#"{{"type":"ai-title","aiTitle":"Novo título","sessionId":"x"}}"#).unwrap();
    drop(file);
    wait_for_change(&running);
}

#[test]
fn starts_following_once_the_projects_folder_appears() {
    let running = start(false);
    std::thread::sleep(Duration::from_millis(300));
    fixtures::install_demo_session(&running.claude_home.join("projects"));
    wait_for_change(&running);
}
