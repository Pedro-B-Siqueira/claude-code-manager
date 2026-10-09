//! Shared helpers: a throwaway HOME whose `.claude` tree is populated and then made read-only.

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use ccm_lib::paths::AppPaths;
use tempfile::TempDir;

pub struct SandboxHome {
    _root: TempDir,
    pub home: PathBuf,
    pub claude_home: PathBuf,
    pub app_support: PathBuf,
}

impl SandboxHome {
    pub fn with_read_only_claude_home() -> Self {
        let root = TempDir::new().expect("temp dir");
        let home = root.path().join("home");
        let claude_home = home.join(".claude");
        let app_support = home.join("Library/Application Support/ClaudeCodeManager");
        fs::create_dir_all(claude_home.join("projects/-tmp-sample")).expect("claude dirs");
        fs::write(claude_home.join("settings.json"), r#"{"hooks":{}}"#).expect("settings");
        fs::write(
            claude_home.join("projects/-tmp-sample/session.jsonl"),
            "{}\n",
        )
        .expect("jsonl");
        fs::write(home.join(".claude.json"), "{}").expect("user config");
        set_read_only_recursively(&claude_home);
        Self {
            _root: root,
            home,
            claude_home,
            app_support,
        }
    }

    pub fn paths(&self) -> AppPaths {
        AppPaths::new(
            self.home.clone(),
            self.claude_home.clone(),
            self.app_support.clone(),
        )
    }

    /// Path → (size, modified) for every entry under `.claude` plus `~/.claude.json`.
    pub fn claude_fingerprint(&self) -> BTreeMap<PathBuf, (u64, std::time::SystemTime)> {
        let mut fingerprint = BTreeMap::new();
        collect_fingerprint(&self.claude_home, &mut fingerprint);
        collect_fingerprint(&self.home.join(".claude.json"), &mut fingerprint);
        fingerprint
    }
}

impl Drop for SandboxHome {
    fn drop(&mut self) {
        set_writable_recursively(&self.claude_home);
    }
}

fn collect_fingerprint(path: &Path, into: &mut BTreeMap<PathBuf, (u64, std::time::SystemTime)>) {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return;
    };
    into.insert(
        path.to_path_buf(),
        (metadata.len(), metadata.modified().expect("mtime")),
    );
    if metadata.is_dir() {
        for entry in fs::read_dir(path).expect("read dir").flatten() {
            collect_fingerprint(&entry.path(), into);
        }
    }
}

fn set_read_only_recursively(path: &Path) {
    apply_mode_recursively(path, 0o555, 0o444);
}

fn set_writable_recursively(path: &Path) {
    apply_mode_recursively(path, 0o755, 0o644);
}

fn apply_mode_recursively(path: &Path, directory_mode: u32, file_mode: u32) {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return;
    };
    if metadata.is_dir() {
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o755));
        for entry in fs::read_dir(path).expect("read dir").flatten() {
            apply_mode_recursively(&entry.path(), directory_mode, file_mode);
        }
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(directory_mode));
    } else {
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(file_mode));
    }
}
