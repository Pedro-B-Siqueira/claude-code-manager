//! Background library sync: an initial scan, then incremental updates driven by FSEvents.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};

use super::indexer::Indexer;
use super::models::IndexProgress;
use crate::db::Database;
use crate::error::AppError;
use crate::paths::AppPaths;

const DEBOUNCE: Duration = Duration::from_millis(400);
/// How often to check whether `~/.claude/projects` appeared (fresh installs have none yet).
const ROOT_POLL: Duration = Duration::from_secs(30);
const CHANGE_NOTICE_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryNotice {
    Progress(IndexProgress),
    Changed,
}

pub type LibraryNotifier = Arc<dyn Fn(LibraryNotice) + Send + Sync>;

pub type SharedProgress = Arc<Mutex<IndexProgress>>;

pub fn start(paths: &AppPaths, progress: SharedProgress, notify: LibraryNotifier) -> Result<(), AppError> {
    start_with_poll(paths, progress, notify, ROOT_POLL)
}

pub fn start_with_poll(paths: &AppPaths, progress: SharedProgress, notify: LibraryNotifier, root_poll: Duration) -> Result<(), AppError> {
    let indexer = Indexer::new(Database::open(&paths.database_file())?, paths.claude_home().join("projects"));
    thread::Builder::new()
        .name("library-indexer".to_owned())
        .spawn(move || run(indexer, progress, notify, root_poll))?;
    Ok(())
}

fn run(mut indexer: Indexer, progress: SharedProgress, notify: LibraryNotifier, root_poll: Duration) {
    while !indexer.projects_root().is_dir() {
        publish_progress(&progress, &notify, IndexProgress::default());
        thread::sleep(root_poll);
    }
    indexer.use_canonical_root();
    let (sender, receiver) = mpsc::channel::<DebounceEventResult>();
    let watcher = watch(&indexer, sender);
    if let Err(error) = indexer.prune_missing() {
        log::warn!("falha ao limpar sessões removidas: {error}");
    }
    initial_scan(&indexer, &progress, &notify);
    if let Some(_watcher) = watcher {
        follow_changes(&indexer, &receiver, &notify);
    }
}

fn watch(indexer: &Indexer, sender: mpsc::Sender<DebounceEventResult>) -> Option<Debouncer<RecommendedWatcher>> {
    let root = indexer.projects_root();
    let mut debouncer = new_debouncer(DEBOUNCE, sender)
        .inspect_err(|error| log::warn!("não foi possível observar transcripts: {error}"))
        .ok()?;
    debouncer
        .watcher()
        .watch(root, RecursiveMode::Recursive)
        .inspect_err(|error| log::warn!("não foi possível observar {}: {error}", root.display()))
        .ok()?;
    Some(debouncer)
}

fn initial_scan(indexer: &Indexer, progress: &SharedProgress, notify: &LibraryNotifier) {
    let files = indexer.discover();
    let total = u32::try_from(files.len()).unwrap_or(u32::MAX);
    publish_progress(progress, notify, IndexProgress { running: true, files_done: 0, files_total: total });
    let mut last_notice = Instant::now();
    for (index, file) in files.iter().enumerate() {
        if let Err(error) = indexer.index_file(file) {
            log::warn!("falha ao indexar {}: {error}", file.display());
        }
        if last_notice.elapsed() >= CHANGE_NOTICE_INTERVAL {
            let done = u32::try_from(index + 1).unwrap_or(total);
            publish_progress(progress, notify, IndexProgress { running: true, files_done: done, files_total: total });
            notify(LibraryNotice::Changed);
            last_notice = Instant::now();
        }
    }
    publish_progress(progress, notify, IndexProgress { running: false, files_done: total, files_total: total });
    notify(LibraryNotice::Changed);
}

fn follow_changes(indexer: &Indexer, receiver: &Receiver<DebounceEventResult>, notify: &LibraryNotifier) {
    for result in receiver {
        let events = match result {
            Ok(events) => events,
            Err(error) => {
                log::warn!("erro do observador de arquivos: {error}");
                continue;
            }
        };
        let mut paths: Vec<PathBuf> = events.into_iter().map(|event| event.path).collect();
        paths.sort();
        paths.dedup();
        match indexer.index_paths(&paths) {
            Ok(outcome) if outcome.files_changed > 0 => notify(LibraryNotice::Changed),
            Ok(_) => {}
            Err(error) => log::warn!("falha na atualização incremental: {error}"),
        }
    }
}

fn publish_progress(progress: &SharedProgress, notify: &LibraryNotifier, snapshot: IndexProgress) {
    *progress.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = snapshot.clone();
    notify(LibraryNotice::Progress(snapshot));
}
