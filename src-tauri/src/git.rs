//! Git, read-mostly. Reads (branch, diff stats, worktrees) run freely; the only writes are creating
//! and removing worktrees, each triggered by an explicit, confirmed user action. Never `checkout`,
//! `reset` or `stash`, and never `--force`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStat {
    pub path: String,
    pub added: u32,
    pub removed: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffStat {
    pub added: u32,
    pub removed: u32,
    pub files: Vec<FileStat>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Worktree {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_main: bool,
    pub detached: bool,
}

pub struct Git {
    binary: PathBuf,
}

impl Git {
    pub fn new(binary: PathBuf) -> Self {
        Self { binary }
    }

    /// `GIT_OPTIONAL_LOCKS=0` keeps read commands from refreshing (rewriting) the repository's index,
    /// which could otherwise make the user's own `git commit` fail on `index.lock`.
    fn run(&self, cwd: &Path, args: &[&str]) -> Result<String, AppError> {
        let output = Command::new(&self.binary)
            .arg("-C")
            .arg(cwd)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim_end().to_owned())
        } else {
            Err(AppError::Git(String::from_utf8_lossy(&output.stderr).trim().to_owned()))
        }
    }

    pub fn current_branch(&self, cwd: &Path) -> Option<String> {
        self.run(cwd, &["branch", "--show-current"]).ok().filter(|branch| !branch.is_empty())
    }

    /// Lines added/removed against HEAD, staged and unstaged together. Uses the `diff-index`
    /// plumbing: unlike `git diff`, it never refreshes the index file.
    pub fn diff_stat(&self, cwd: &Path) -> Result<DiffStat, AppError> {
        Ok(parse_numstat(&self.run(cwd, &["diff-index", "--numstat", "HEAD"])?))
    }

    pub fn main_repository(&self, cwd: &Path) -> Option<PathBuf> {
        let common = PathBuf::from(self.run(cwd, &["rev-parse", "--path-format=absolute", "--git-common-dir"]).ok()?);
        if common.file_name().is_some_and(|name| name == ".git") { common.parent().map(Path::to_path_buf) } else { Some(common) }
    }

    pub fn worktrees(&self, repo: &Path) -> Result<Vec<Worktree>, AppError> {
        Ok(parse_worktrees(&self.run(repo, &["worktree", "list", "--porcelain"])?))
    }

    pub fn is_clean(&self, cwd: &Path) -> Result<bool, AppError> {
        Ok(self.run(cwd, &["status", "--porcelain"])?.trim().is_empty())
    }

    /// `origin`'s default branch when known locally, else `main`/`master`, else the current branch.
    pub fn default_branch(&self, repo: &Path) -> Option<String> {
        let from_origin = self.run(repo, &["symbolic-ref", "--quiet", "--short", "refs/remotes/origin/HEAD"]).ok().and_then(|reference| reference.strip_prefix("origin/").map(str::to_owned));
        let candidates = from_origin.into_iter().chain(["main".to_owned(), "master".to_owned()]);
        for candidate in candidates {
            if self.run(repo, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{candidate}")]).is_ok() {
                return Some(candidate);
            }
        }
        self.current_branch(repo)
    }

    pub fn is_valid_branch_name(&self, repo: &Path, branch: &str) -> bool {
        self.run(repo, &["check-ref-format", "--branch", branch]).is_ok()
    }

    pub fn branch_exists(&self, repo: &Path, branch: &str) -> bool {
        self.run(repo, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")]).is_ok()
    }

    pub fn add_worktree(&self, repo: &Path, path: &Path, branch: &str, base: &str) -> Result<(), AppError> {
        let path = path.to_string_lossy();
        self.run(repo, &["worktree", "add", "-b", branch, &path, base]).map(|_| ())
    }

    /// Removes a linked worktree only when it has no pending changes; never the main working tree.
    pub fn remove_worktree(&self, repo: &Path, path: &Path) -> Result<(), AppError> {
        let worktree = self.worktrees(repo)?.into_iter().find(|worktree| same_path(&worktree.path, path)).ok_or_else(|| AppError::Git("esta pasta não é um worktree do repositório".to_owned()))?;
        if worktree.is_main {
            return Err(AppError::Git("o worktree principal não pode ser removido".to_owned()));
        }
        if !self.is_clean(path)? {
            return Err(AppError::Git("o worktree tem alterações pendentes; nada foi removido".to_owned()));
        }
        self.run(repo, &["worktree", "remove", &path.to_string_lossy()]).map(|_| ())
    }

    pub fn remote_url(&self, repo: &Path) -> Option<String> {
        self.run(repo, &["remote", "get-url", "origin"]).ok().filter(|url| !url.is_empty())
    }
}

fn same_path(left: &Path, right: &Path) -> bool {
    let canonical = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    canonical(left) == canonical(right)
}

pub fn parse_numstat(text: &str) -> DiffStat {
    let mut stat = DiffStat::default();
    for line in text.lines() {
        let mut fields = line.splitn(3, '\t');
        let (Some(added), Some(removed), Some(path)) = (fields.next(), fields.next(), fields.next()) else { continue };
        // Without an index refresh, files only touched (same content) show up as 0/0: not a change.
        if added == "0" && removed == "0" {
            continue;
        }
        // Binary files report "-" for both counts.
        let added = added.parse().unwrap_or(0);
        let removed = removed.parse().unwrap_or(0);
        stat.added += added;
        stat.removed += removed;
        stat.files.push(FileStat { path: path.to_owned(), added, removed });
    }
    stat
}

pub fn parse_worktrees(text: &str) -> Vec<Worktree> {
    let mut worktrees = Vec::new();
    for block in text.split("\n\n").filter(|block| !block.trim().is_empty()) {
        let mut path = None;
        let mut branch = None;
        let mut detached = false;
        for line in block.lines() {
            if let Some(value) = line.strip_prefix("worktree ") {
                path = Some(PathBuf::from(value));
            } else if let Some(value) = line.strip_prefix("branch ") {
                branch = Some(value.strip_prefix("refs/heads/").unwrap_or(value).to_owned());
            } else if line == "detached" {
                detached = true;
            }
        }
        if let Some(path) = path {
            worktrees.push(Worktree { path, branch, is_main: worktrees.is_empty(), detached });
        }
    }
    worktrees
}

/// Lowercase letters, digits and single hyphens — safe for both folder and branch names.
pub fn slugify(text: &str) -> String {
    let mut slug = String::new();
    for character in text.trim().to_lowercase().chars() {
        let mapped = match character {
            'a'..='z' | '0'..='9' => character,
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            _ => '-',
        };
        if mapped != '-' || !slug.ends_with('-') {
            slug.push(mapped);
        }
    }
    slug.trim_matches('-').chars().take(60).collect::<String>().trim_end_matches('-').to_owned()
}

/// The folder most existing linked worktrees live in, so new ones follow the user's layout.
pub fn infer_worktree_root(worktree_lists: &[Vec<Worktree>]) -> Option<PathBuf> {
    let mut counts: HashMap<PathBuf, usize> = HashMap::new();
    for worktree in worktree_lists.iter().flatten().filter(|worktree| !worktree.is_main) {
        if let Some(parent) = worktree.path.parent() {
            *counts.entry(parent.to_path_buf()).or_default() += 1;
        }
    }
    counts.into_iter().max_by(|first, second| first.1.cmp(&second.1).then_with(|| second.0.cmp(&first.0))).map(|(parent, _)| parent)
}

/// Web URL to compare `branch` against `base` on GitHub, GitLab or Bitbucket remotes.
pub fn compare_url(remote: &str, base: &str, branch: &str) -> Option<String> {
    let trimmed = remote.trim().trim_end_matches(".git");
    let (host, project) = if let Some(rest) = trimmed.strip_prefix("git@") {
        rest.split_once(':')?
    } else {
        let without_scheme = trimmed.split_once("://")?.1;
        let without_user = without_scheme.rsplit_once('@').map_or(without_scheme, |(_, host)| host);
        without_user.split_once('/')?
    };
    let url = if host.contains("gitlab") {
        format!("https://{host}/{project}/-/compare/{base}...{branch}")
    } else if host.contains("bitbucket") {
        format!("https://{host}/{project}/compare/{branch}..{base}")
    } else {
        format!("https://{host}/{project}/compare/{base}...{branch}?expand=1")
    };
    Some(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_numstat_including_binary_files() {
        let stat = parse_numstat("10\t2\tsrc/a.ts\n-\t-\tlogo.png\n3\t0\tdocs/b.md\n0\t0\ttouched-only.ts");
        assert_eq!((stat.added, stat.removed, stat.files.len()), (13, 2, 3));
        assert_eq!(stat.files[1], FileStat { path: "logo.png".to_owned(), added: 0, removed: 0 });
    }

    #[test]
    fn parses_worktree_porcelain() {
        let text = "worktree /code/app\nHEAD abc\nbranch refs/heads/master\n\nworktree /wt/app-2315\nHEAD def\ndetached\n\nworktree /wt/app-bulk\nHEAD 123\nbranch refs/heads/fix-bulk\n";
        let worktrees = parse_worktrees(text);
        assert_eq!(worktrees.len(), 3);
        assert!(worktrees[0].is_main && !worktrees[1].is_main);
        assert!(worktrees[1].detached);
        assert_eq!(worktrees[2].branch.as_deref(), Some("fix-bulk"));
    }

    #[test]
    fn slugs_are_safe_for_folders_and_branches() {
        assert_eq!(slugify("Paginação da listagem!"), "paginacao-da-listagem");
        assert_eq!(slugify("  --Fix   CSV export-- "), "fix-csv-export");
        assert_eq!(slugify("../../etc"), "etc");
        assert_eq!(slugify("💥"), "");
    }

    #[test]
    fn infers_the_most_common_worktree_folder() {
        let worktree = |path: &str, is_main: bool| Worktree { path: PathBuf::from(path), branch: None, is_main, detached: false };
        let lists = vec![
            vec![worktree("/code/api", true), worktree("/home/me/worktrees/api-1", false), worktree("/home/me/worktrees/api-2", false)],
            vec![worktree("/code/web", true), worktree("/tmp/web-x", false)],
        ];
        assert_eq!(infer_worktree_root(&lists), Some(PathBuf::from("/home/me/worktrees")));
        assert_eq!(infer_worktree_root(&[vec![worktree("/code/api", true)]]), None);
    }

    #[test]
    fn compare_urls_for_common_hosts() {
        assert_eq!(compare_url("git@github.com:acme/web.git", "main", "feat-x").as_deref(), Some("https://github.com/acme/web/compare/main...feat-x?expand=1"));
        assert_eq!(compare_url("https://token@github.com/acme/web", "master", "fix-y").as_deref(), Some("https://github.com/acme/web/compare/master...fix-y?expand=1"));
        assert_eq!(compare_url("git@gitlab.com:group/sub/app.git", "main", "b").as_deref(), Some("https://gitlab.com/group/sub/app/-/compare/main...b"));
        assert_eq!(compare_url("not a url", "a", "b"), None);
    }
}
