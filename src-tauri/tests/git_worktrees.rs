//! Worktree operations against a throwaway repository (never the user's repositories).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use ccm_lib::error::AppError;
use ccm_lib::git::{Git, infer_worktree_root};

fn git_command(directory: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "-c", "commit.gpgsign=false"])
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

struct Sandbox {
    _root: tempfile::TempDir,
    repo: PathBuf,
    worktrees: PathBuf,
}

fn sandbox() -> Sandbox {
    let root = tempfile::tempdir().unwrap();
    let repo = root.path().join("app");
    fs::create_dir_all(&repo).unwrap();
    git_command(&repo, &["init", "-q", "-b", "master"]);
    fs::write(repo.join("README.md"), "hello\n").unwrap();
    git_command(&repo, &["add", "."]);
    git_command(&repo, &["commit", "-q", "-m", "init"]);
    let worktrees = root.path().join("my-worktrees");
    Sandbox { _root: root, repo, worktrees }
}

#[test]
fn creates_lists_and_infers_the_worktree_folder() {
    let sandbox = sandbox();
    let git = Git::new(PathBuf::from("git"));
    assert_eq!(git.default_branch(&sandbox.repo).as_deref(), Some("master"));

    let first = sandbox.worktrees.join("app-orders");
    fs::create_dir_all(&sandbox.worktrees).unwrap();
    git.add_worktree(&sandbox.repo, &first, "feat-orders", "master").unwrap();

    let list = git.worktrees(&sandbox.repo).unwrap();
    assert_eq!(list.len(), 2);
    assert!(list[0].is_main);
    assert_eq!(list[1].branch.as_deref(), Some("feat-orders"));
    assert_eq!(git.main_repository(&first).map(|path| path.canonicalize().unwrap()), Some(sandbox.repo.canonicalize().unwrap()));
    assert_eq!(infer_worktree_root(&[list]).map(|path| path.canonicalize().unwrap()), Some(sandbox.worktrees.canonicalize().unwrap()));
    assert!(git.branch_exists(&sandbox.repo, "feat-orders"));
    assert!(!git.is_valid_branch_name(&sandbox.repo, "bad..name"));
}

#[test]
fn refuses_to_remove_dirty_or_main_worktrees() {
    let sandbox = sandbox();
    let git = Git::new(PathBuf::from("git"));
    let path = sandbox.worktrees.join("app-dirty");
    git.add_worktree(&sandbox.repo, &path, "fix-dirty", "master").unwrap();
    fs::write(path.join("notes.txt"), "pending\n").unwrap();

    assert!(matches!(git.remove_worktree(&sandbox.repo, &path), Err(AppError::Git(message)) if message.contains("pendentes")));
    assert!(path.exists(), "dirty worktree stays on disk");
    assert!(matches!(git.remove_worktree(&sandbox.repo, &sandbox.repo), Err(AppError::Git(_))));

    fs::remove_file(path.join("notes.txt")).unwrap();
    git.remove_worktree(&sandbox.repo, &path).unwrap();
    assert!(!path.exists());
    assert!(git.branch_exists(&sandbox.repo, "fix-dirty"), "removing a worktree keeps its branch");
}

#[test]
fn diff_stat_counts_working_tree_changes() {
    let sandbox = sandbox();
    let git = Git::new(PathBuf::from("git"));
    fs::write(sandbox.repo.join("README.md"), "hello\nworld\nagain\n").unwrap();
    let stat = git.diff_stat(&sandbox.repo).unwrap();
    assert_eq!((stat.added, stat.removed, stat.files.len()), (2, 0, 1));
    assert_eq!(git.current_branch(&sandbox.repo).as_deref(), Some("master"));
}

#[test]
fn read_only_queries_leave_the_git_index_untouched() {
    let sandbox = sandbox();
    let git = Git::new(PathBuf::from("/usr/bin/git"));
    thread::sleep(Duration::from_millis(1_100));
    fs::write(sandbox.repo.join("README.md"), "hello\n").unwrap();
    let before = fs::read(sandbox.repo.join(".git/index")).unwrap();

    let stat = git.diff_stat(&sandbox.repo).unwrap();
    assert!(git.is_clean(&sandbox.repo).unwrap());
    git.current_branch(&sandbox.repo);
    git.main_repository(&sandbox.repo);
    git.worktrees(&sandbox.repo).unwrap();
    git.default_branch(&sandbox.repo);

    assert_eq!(stat.files.len(), 0, "touching a file without changing it is not a change");
    assert_eq!(fs::read(sandbox.repo.join(".git/index")).unwrap(), before, "the index file must not be rewritten");
}

#[test]
fn worktree_operations_never_move_the_main_checkout() {
    let sandbox = sandbox();
    let git = Git::new(PathBuf::from("git"));
    fs::write(sandbox.repo.join("wip.txt"), "work in progress\n").unwrap();
    let status_before = git_output(&sandbox.repo, &["status", "--porcelain"]);
    let head_before = git_output(&sandbox.repo, &["rev-parse", "HEAD"]);

    let path = sandbox.worktrees.join("app-feature");
    git.add_worktree(&sandbox.repo, &path, "feat-feature", "master").unwrap();
    git.remove_worktree(&sandbox.repo, &path).unwrap();

    assert_eq!(git.current_branch(&sandbox.repo).as_deref(), Some("master"));
    assert_eq!(git_output(&sandbox.repo, &["rev-parse", "HEAD"]), head_before);
    assert_eq!(git_output(&sandbox.repo, &["status", "--porcelain"]), status_before);
    assert_eq!(git_output(&sandbox.repo, &["stash", "list"]), "");
}

fn git_output(directory: &Path, args: &[&str]) -> String {
    let output = Command::new("git").arg("-C").arg(directory).args(args).output().unwrap();
    String::from_utf8_lossy(&output.stdout).into_owned()
}
