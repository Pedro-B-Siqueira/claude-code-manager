//! Groups sessions by repository: a worktree resolves to the main repository it belongs to.
//! Uses only read-only git plumbing (`rev-parse`).

use std::path::{Path, PathBuf};
use std::process::Command;

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInfo {
    pub name: String,
    pub repo_root: Option<PathBuf>,
}

pub fn ensure_project(connection: &Connection, cwd: &str) -> Result<(), AppError> {
    let known: Option<i64> = connection
        .query_row("SELECT 1 FROM projects WHERE cwd = ?1", params![cwd], |row| row.get(0))
        .optional()?;
    if known.is_some() {
        return Ok(());
    }
    let project = resolve_project(Path::new(cwd));
    connection.execute(
        "INSERT INTO projects (cwd, name, repo_root) VALUES (?1, ?2, ?3)",
        params![cwd, project.name, project.repo_root.map(|root| root.to_string_lossy().into_owned())],
    )?;
    Ok(())
}

pub fn resolve_project(cwd: &Path) -> ProjectInfo {
    let repo_root = cwd.is_dir().then(|| main_repository_root(cwd)).flatten();
    let name = repo_root
        .as_deref()
        .or(Some(cwd))
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| cwd.to_string_lossy().into_owned());
    ProjectInfo { name, repo_root }
}

/// The shared `.git` directory of a worktree lives in the main repository, so its parent names the project.
fn main_repository_root(cwd: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let common_dir = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    if common_dir.file_name().is_some_and(|name| name == ".git") {
        common_dir.parent().map(Path::to_path_buf)
    } else {
        Some(common_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_directory_falls_back_to_folder_name() {
        let project = resolve_project(Path::new("/nonexistent/place/my-service"));
        assert_eq!(project, ProjectInfo { name: "my-service".to_owned(), repo_root: None });
    }

    #[test]
    fn plain_directory_without_git_uses_its_name() {
        let directory = tempfile::tempdir().unwrap();
        let folder = directory.path().join("notes");
        std::fs::create_dir(&folder).unwrap();
        assert_eq!(resolve_project(&folder).name, "notes");
    }
}
