use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;
use tauri::{AppHandle, State};

use super::sessions::launch;
use crate::error::AppError;
use crate::git::{self, DiffStat, Git};
use crate::library::queries;
use crate::platform::{self, OpenTarget};
use crate::pty::{LaunchMode, LaunchSpec};
use crate::sessions::launch::expand_home;
use crate::sessions::view::LiveSessionView;
use crate::settings;
use crate::state::AppState;

const FALLBACK_WORKTREE_ROOT: &str = "worktrees";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub branch: Option<String>,
    pub diff: DiffStat,
}

#[tauri::command]
pub async fn git_status(state: State<'_, AppState>, cwd: String) -> Result<GitStatus, AppError> {
    let git = state.git();
    let directory = existing_directory(&cwd)?;
    Ok(GitStatus { branch: git.current_branch(&directory), diff: git.diff_stat(&directory).unwrap_or_default() })
}

fn existing_directory(path: &str) -> Result<PathBuf, AppError> {
    let directory = PathBuf::from(path);
    if directory.is_dir() { Ok(directory) } else { Err(AppError::MissingDirectory(directory)) }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreePlan {
    pub repo_root: PathBuf,
    pub repo_name: String,
    pub root: PathBuf,
    pub root_inferred: bool,
    pub path: PathBuf,
    pub branch: String,
    pub base: String,
}

/// Where new worktrees go: the configured folder, else the one existing worktrees already use.
fn worktree_root(state: &AppState, git: &Git, repo: &Path) -> Result<(PathBuf, bool), AppError> {
    if let Some(configured) = settings::load(&state.database)?.worktree_root {
        return Ok((expand_home(&configured, state.paths.home()), false));
    }
    let mut repositories = vec![repo.to_path_buf()];
    let known: Vec<String> = state
        .database
        .connection()
        .prepare("SELECT DISTINCT repo_root FROM projects WHERE repo_root IS NOT NULL")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    repositories.extend(known.into_iter().map(PathBuf::from).filter(|path| path.as_path() != repo));
    let lists: Vec<_> = repositories.iter().filter_map(|repository| git.worktrees(repository).ok()).collect();
    let local_first = git::infer_worktree_root(&lists[..1]).or_else(|| git::infer_worktree_root(&lists));
    Ok(match local_first {
        Some(root) => (root, true),
        None => (state.paths.home().join(FALLBACK_WORKTREE_ROOT), false),
    })
}

fn plan(state: &AppState, cwd: &str, prefix: &str, name: &str) -> Result<WorktreePlan, AppError> {
    let git = state.git();
    let directory = existing_directory(cwd)?;
    let repo_root = git.main_repository(&directory).ok_or_else(|| AppError::Invalid("a pasta escolhida não é um repositório git".to_owned()))?;
    let prefixes = settings::load(&state.database)?.branch_prefixes;
    if !prefixes.iter().any(|known| known == prefix) {
        return Err(AppError::Invalid(format!("prefixo de branch desconhecido: {prefix}")));
    }
    let slug = git::slugify(name);
    if slug.is_empty() {
        return Err(AppError::Invalid("dê um nome com letras ou números para a branch".to_owned()));
    }
    let repo_name = repo_root.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_else(|| "repo".to_owned());
    let (root, root_inferred) = worktree_root(state, &git, &repo_root)?;
    let branch = format!("{prefix}{slug}");
    let path = root.join(format!("{repo_name}-{slug}"));
    let base = git.default_branch(&repo_root).ok_or_else(|| AppError::Invalid("não encontrei a branch principal do repositório".to_owned()))?;
    if path.exists() {
        return Err(AppError::Invalid(format!("já existe uma pasta em {}", path.display())));
    }
    if !git.is_valid_branch_name(&repo_root, &branch) || git.branch_exists(&repo_root, &branch) {
        return Err(AppError::Invalid(format!("a branch {branch} já existe ou tem nome inválido")));
    }
    Ok(WorktreePlan { repo_root, repo_name, root, root_inferred, path, branch, base })
}

#[tauri::command]
pub async fn worktree_plan(state: State<'_, AppState>, cwd: String, prefix: String, name: String) -> Result<WorktreePlan, AppError> {
    plan(&state, &cwd, &prefix, &name)
}

/// Creates the worktree (confirmed in the UI) and opens a new session inside it. The plan is
/// recomputed here so the paths never come from the frontend.
#[tauri::command]
pub async fn worktree_create(state: State<'_, AppState>, app: AppHandle, cwd: String, prefix: String, name: String) -> Result<LiveSessionView, AppError> {
    let plan = plan(&state, &cwd, &prefix, &name)?;
    state.paths.ensure_writable(&plan.root)?;
    state.paths.ensure_writable(&plan.path)?;
    std::fs::create_dir_all(&plan.root)?;
    state.git().add_worktree(&plan.repo_root, &plan.path, &plan.branch, &plan.base)?;
    log::info!("worktree criado em {} (branch {})", plan.path.display(), plan.branch);
    let spec = LaunchSpec { session_id: uuid::Uuid::new_v4().to_string(), cwd: plan.path.clone(), mode: LaunchMode::New, worktree: Some(plan.path) };
    launch(&state, &app, spec)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_main: bool,
    pub detached: bool,
    pub clean: Option<bool>,
    pub in_use: bool,
}

fn running_session_dirs(state: &AppState) -> Vec<PathBuf> {
    state.pty.snapshots().into_iter().filter(|snapshot| !snapshot.exited).map(|snapshot| snapshot.launch.cwd).collect()
}

#[tauri::command]
pub async fn worktree_list(state: State<'_, AppState>, cwd: String) -> Result<Vec<WorktreeInfo>, AppError> {
    let git = state.git();
    let directory = existing_directory(&cwd)?;
    let Some(repo) = git.main_repository(&directory) else { return Ok(Vec::new()) };
    let busy = running_session_dirs(&state);
    Ok(git
        .worktrees(&repo)?
        .into_iter()
        .map(|worktree| WorktreeInfo {
            clean: worktree.path.is_dir().then(|| git.is_clean(&worktree.path).ok()).flatten(),
            in_use: busy.iter().any(|dir| dir.starts_with(&worktree.path)),
            path: worktree.path,
            branch: worktree.branch,
            is_main: worktree.is_main,
            detached: worktree.detached,
        })
        .collect())
}

#[tauri::command]
pub async fn worktree_remove(state: State<'_, AppState>, cwd: String, path: String) -> Result<(), AppError> {
    let git = state.git();
    let repo = git.main_repository(&existing_directory(&cwd)?).ok_or_else(|| AppError::Invalid("a pasta não é um repositório git".to_owned()))?;
    let target = PathBuf::from(&path);
    if running_session_dirs(&state).iter().any(|dir| dir.starts_with(&target)) {
        return Err(AppError::Invalid("há uma sessão aberta neste worktree; encerre-a antes".to_owned()));
    }
    git.remove_worktree(&repo, &target)?;
    log::info!("worktree removido: {}", target.display());
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenOutcome {
    pub warning: Option<String>,
}

#[tauri::command]
pub async fn open_vscode(state: State<'_, AppState>, path: String, expected_branch: Option<String>) -> Result<OpenOutcome, AppError> {
    let directory = existing_directory(&path)?;
    let current = state.git().current_branch(&directory);
    let warning = match (expected_branch.as_deref(), current.as_deref()) {
        (Some(expected), Some(current)) if expected != current => Some(format!(
            "A pasta está na branch {current}, mas a sessão trabalhou em {expected}. Nada foi alterado: troque de branch se precisar."
        )),
        _ => None,
    };
    let opened = match state.shell().find_binary("code") {
        Some(code) => Command::new(code).arg(&directory).spawn().map(drop).map_err(AppError::from),
        None => match platform::vscode_fallback(&directory, platform::CURRENT) {
            Some(fallback) => platform::spawn(&fallback),
            None => Err(AppError::Invalid("o comando `code` não foi encontrado; no VS Code, rode \"Shell Command: Install 'code' command in PATH\"".to_owned())),
        },
    };
    opened?;
    Ok(OpenOutcome { warning })
}

#[tauri::command]
pub async fn open_finder(path: String) -> Result<(), AppError> {
    let directory = existing_directory(&path)?;
    platform::open(OpenTarget::Folder(&directory))
}

fn open_url(url: &str) -> Result<(), AppError> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(AppError::Invalid("endereço de PR inválido".to_owned()));
    }
    platform::open(OpenTarget::Url(url))
}

/// Opens the session's pull request without writing anything: the PR recorded in the transcript,
/// else `gh pr view` (read-only), else the repository's compare page.
#[tauri::command]
pub async fn open_pr(state: State<'_, AppState>, session_id: String) -> Result<String, AppError> {
    let summary = queries::session_summary(&state.database.connection(), &session_id)?.ok_or_else(|| AppError::UnknownSession(session_id.clone()))?;
    if let Some(url) = summary.pr_url {
        open_url(&url)?;
        return Ok(url);
    }
    let directory = existing_directory(summary.item.cwd.as_deref().unwrap_or_default())?;
    let git = state.git();
    let branch = summary.item.branch.or_else(|| git.current_branch(&directory)).ok_or_else(|| AppError::Invalid("a sessão não tem branch".to_owned()))?;
    if let Some(url) = existing_pr_url(&state, &directory, &branch) {
        open_url(&url)?;
        return Ok(url);
    }
    let repo = git.main_repository(&directory).unwrap_or(directory);
    let base = git.default_branch(&repo).unwrap_or_else(|| "main".to_owned());
    let url = git.remote_url(&repo).and_then(|remote| git::compare_url(&remote, &base, &branch)).ok_or_else(|| AppError::Invalid("o repositório não tem um remoto conhecido".to_owned()))?;
    open_url(&url)?;
    Ok(url)
}

fn existing_pr_url(state: &AppState, directory: &Path, branch: &str) -> Option<String> {
    let gh = state.shell().find_binary("gh")?;
    let output = Command::new(gh).current_dir(directory).args(["pr", "view", branch, "--json", "url", "--jq", ".url"]).env("GH_PROMPT_DISABLED", "1").output().ok()?;
    let url = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (output.status.success() && url.starts_with("https://")).then_some(url)
}
