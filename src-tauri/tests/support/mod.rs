//! Shared helpers: a throwaway HOME whose `.claude` tree is populated and then made read-only.

#![allow(dead_code)]

pub mod fixtures;

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

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
        fs::create_dir_all(claude_home.join("projects")).expect("claude dirs");
        fs::write(
            claude_home.join("settings.json"),
            r#"{"hooks":{"Notification":[{"hooks":[{"type":"command","command":"/Applications/ClaudeGauge.app/Contents/Resources/claude-notify.sh attention"}]}]}}"#,
        )
        .expect("settings");
        fs::create_dir_all(claude_home.join("sessions")).expect("registry dir");
        let own_pid = std::process::id();
        fs::write(
            claude_home.join(format!("sessions/{own_pid}.json")),
            format!(r#"{{"pid":{own_pid},"sessionId":"{}","cwd":"/tmp/ccm-fixture/demo-app","status":"busy"}}"#, fixtures::DEMO_SESSION_ID),
        )
        .expect("registry entry");
        fixtures::install_demo_session(&claude_home.join("projects"));
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

/// Built once per test binary. Every `cargo build` re-links `target/debug/fake-claude`, even when
/// nothing changed, and a test executing it during the re-link hangs or is killed by macOS.
pub fn fake_claude_binary() -> PathBuf {
    static BINARY: OnceLock<PathBuf> = OnceLock::new();
    BINARY
        .get_or_init(|| {
            let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
            let status = Command::new(cargo)
                .args(["build", "--quiet", "-p", "fake-claude", "--manifest-path"])
                .arg(manifest.join("Cargo.toml"))
                .status()
                .expect("cargo build fake-claude");
            assert!(status.success());
            manifest.join("target/debug/fake-claude")
        })
        .clone()
}
