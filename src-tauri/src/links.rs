//! Where a link clicked in the terminal goes. A local file is never handed to `open` unless it is
//! an image, so a link to an app or script printed by some command cannot start it.

use std::fs;
use std::path::{Path, PathBuf};

use tauri::Url;

const IMAGE_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "gif", "webp"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathKind {
    Missing,
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkAction {
    Browser(Url),
    Image(PathBuf),
    Editor(PathBuf),
    Reveal(PathBuf),
    Unsupported,
}

pub fn path_kind(path: &Path) -> PathKind {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => PathKind::Directory,
        Ok(metadata) if metadata.is_file() => PathKind::File,
        _ => PathKind::Missing,
    }
}

pub fn classify_link(raw: &str, kind_of: impl Fn(&Path) -> PathKind) -> LinkAction {
    let Ok(url) = Url::parse(raw.trim()) else { return LinkAction::Unsupported };
    match url.scheme() {
        "http" | "https" => LinkAction::Browser(url),
        "file" => url.to_file_path().map_or(LinkAction::Unsupported, |path| classify_file(path, kind_of)),
        _ => LinkAction::Unsupported,
    }
}

fn classify_file(path: PathBuf, kind_of: impl Fn(&Path) -> PathKind) -> LinkAction {
    match kind_of(&path) {
        PathKind::Missing => LinkAction::Unsupported,
        PathKind::Directory => LinkAction::Reveal(path),
        PathKind::File if is_image(&path) => LinkAction::Image(path),
        PathKind::File => LinkAction::Editor(path),
    }
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| IMAGE_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    fn kind_of(path: &Path) -> PathKind {
        match path.to_str() {
            Some("/Users/dev/shot.png" | "/Users/dev/my notes.md" | "/Users/dev/run.command") => PathKind::File,
            Some("/Applications/Calculator.app" | "/Users/dev/project") => PathKind::Directory,
            _ => PathKind::Missing,
        }
    }

    #[test]
    fn web_links_open_in_the_browser() {
        assert!(matches!(classify_link("https://github.com/org/repo/pull/1", kind_of), LinkAction::Browser(url) if url.as_str() == "https://github.com/org/repo/pull/1"));
        assert!(matches!(classify_link("http://localhost:5173/", kind_of), LinkAction::Browser(_)));
    }

    #[test]
    fn local_files_never_run() {
        assert_eq!(classify_link("file:///Users/dev/shot.png", kind_of), LinkAction::Image(PathBuf::from("/Users/dev/shot.png")));
        assert_eq!(classify_link("file:///Users/dev/my%20notes.md", kind_of), LinkAction::Editor(PathBuf::from("/Users/dev/my notes.md")));
        assert_eq!(classify_link("file:///Users/dev/run.command", kind_of), LinkAction::Editor(PathBuf::from("/Users/dev/run.command")));
        assert_eq!(classify_link("file:///Applications/Calculator.app", kind_of), LinkAction::Reveal(PathBuf::from("/Applications/Calculator.app")));
        assert_eq!(classify_link("file:///Users/dev/project", kind_of), LinkAction::Reveal(PathBuf::from("/Users/dev/project")));
    }

    #[test]
    fn everything_else_is_refused() {
        for link in ["javascript:alert(1)", "mailto:dev@example.com", "vscode://file/x", "file:///nowhere.png", "file://other-host/etc/passwd", "not a url", ""] {
            assert_eq!(classify_link(link, kind_of), LinkAction::Unsupported, "{link}");
        }
    }
}
