//! What differs between macOS and Linux: how files and links are opened, where the app keeps its
//! data and how processes are listed. Commands are built by pure functions that take the system, so
//! both are testable from either one.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::AppError;

const APP_FOLDER: &str = "ClaudeCodeManager";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    MacOs,
    Linux,
}

pub const CURRENT: Os = if cfg!(target_os = "macos") { Os::MacOs } else { Os::Linux };

#[derive(Debug, Clone, Copy)]
pub enum OpenTarget<'a> {
    Url(&'a str),
    Image(&'a Path),
    /// Show where an item is without opening it: Finder selects it, Linux opens its folder.
    Reveal(&'a Path),
    Folder(&'a Path),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: PathBuf,
    pub args: Vec<OsString>,
}

impl CommandSpec {
    fn new(program: &str, args: Vec<OsString>) -> Self {
        Self { program: PathBuf::from(program), args }
    }
}

pub fn open_command(target: OpenTarget<'_>, os: Os) -> CommandSpec {
    match (os, target) {
        (Os::MacOs, OpenTarget::Url(url)) => CommandSpec::new("/usr/bin/open", vec![url.into()]),
        (Os::MacOs, OpenTarget::Image(path)) => CommandSpec::new("/usr/bin/open", vec!["-a".into(), "Preview".into(), path.into()]),
        (Os::MacOs, OpenTarget::Reveal(path)) => CommandSpec::new("/usr/bin/open", vec!["-R".into(), path.into()]),
        (Os::MacOs, OpenTarget::Folder(path)) => CommandSpec::new("/usr/bin/open", vec![path.into()]),
        (Os::Linux, OpenTarget::Url(url)) => CommandSpec::new("xdg-open", vec![url.into()]),
        (Os::Linux, OpenTarget::Image(path) | OpenTarget::Folder(path)) => CommandSpec::new("xdg-open", vec![path.into()]),
        (Os::Linux, OpenTarget::Reveal(path)) => CommandSpec::new("xdg-open", vec![path.parent().unwrap_or(path).into()]),
    }
}

/// Opening a folder in VS Code when its `code` command is not on the PATH.
pub fn vscode_fallback(folder: &Path, os: Os) -> Option<CommandSpec> {
    (os == Os::MacOs).then(|| CommandSpec::new("/usr/bin/open", vec!["-a".into(), "Visual Studio Code".into(), folder.into()]))
}

pub fn ps_command(os: Os) -> CommandSpec {
    let (program, flag) = match os {
        Os::MacOs => ("/bin/ps", "-axo"),
        Os::Linux => ("ps", "-eo"),
    };
    CommandSpec::new(program, vec![flag.into(), "pid=,ppid=,rss=,comm=".into()])
}

/// The XDG spec says a relative `XDG_DATA_HOME` must be ignored.
pub fn default_app_support(home: &Path, os: Os, xdg_data_home: Option<&Path>) -> PathBuf {
    match os {
        Os::MacOs => home.join("Library/Application Support").join(APP_FOLDER),
        Os::Linux => xdg_data_home.filter(|path| path.is_absolute()).map_or_else(|| home.join(".local/share"), Path::to_path_buf).join(APP_FOLDER),
    }
}

pub fn spawn(spec: &CommandSpec) -> Result<(), AppError> {
    match Command::new(&spec.program).args(&spec.args).spawn() {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Err(AppError::Invalid(format!("`{}` não foi encontrado; no Linux, instale o pacote xdg-utils", spec.program.display())))
        }
        Err(error) => Err(error.into()),
    }
}

pub fn open(target: OpenTarget<'_>) -> Result<(), AppError> {
    spawn(&open_command(target, CURRENT))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    use super::*;

    fn args(spec: &CommandSpec) -> Vec<String> {
        spec.args.iter().map(|arg| arg.to_string_lossy().into_owned()).collect()
    }

    #[test]
    fn macos_uses_open_and_preview() {
        let image = Path::new("/Users/dev/Área de Trabalho/print 1.png");
        assert_eq!(open_command(OpenTarget::Url("https://example.com"), Os::MacOs), CommandSpec { program: PathBuf::from("/usr/bin/open"), args: vec![OsString::from("https://example.com")] });
        assert_eq!(args(&open_command(OpenTarget::Image(image), Os::MacOs)), vec!["-a", "Preview", "/Users/dev/Área de Trabalho/print 1.png"]);
        assert_eq!(args(&open_command(OpenTarget::Reveal(Path::new("/a/b.ts")), Os::MacOs)), vec!["-R", "/a/b.ts"]);
        assert_eq!(args(&open_command(OpenTarget::Folder(Path::new("/a")), Os::MacOs)), vec!["/a"]);
    }

    #[test]
    fn linux_uses_xdg_open_and_shows_the_containing_folder() {
        let image = Path::new("/home/dev/Área de Trabalho/print 1.png");
        let spec = open_command(OpenTarget::Image(image), Os::Linux);
        assert_eq!(spec.program, PathBuf::from("xdg-open"));
        assert_eq!(args(&spec), vec!["/home/dev/Área de Trabalho/print 1.png"], "the path is one argument, never parsed by a shell");
        assert_eq!(args(&open_command(OpenTarget::Url("https://example.com"), Os::Linux)), vec!["https://example.com"]);
        assert_eq!(args(&open_command(OpenTarget::Reveal(Path::new("/home/dev/app/src/main.rs")), Os::Linux)), vec!["/home/dev/app/src"]);
        assert_eq!(args(&open_command(OpenTarget::Reveal(Path::new("/")), Os::Linux)), vec!["/"]);
        assert_eq!(args(&open_command(OpenTarget::Folder(Path::new("/home/dev/app")), Os::Linux)), vec!["/home/dev/app"]);
    }

    #[test]
    fn vscode_fallback_exists_only_on_macos() {
        assert_eq!(args(&vscode_fallback(Path::new("/a"), Os::MacOs).unwrap()), vec!["-a", "Visual Studio Code", "/a"]);
        assert!(vscode_fallback(Path::new("/a"), Os::Linux).is_none());
    }

    #[test]
    fn process_listing_follows_each_ps_dialect() {
        assert_eq!(ps_command(Os::MacOs), CommandSpec { program: PathBuf::from("/bin/ps"), args: vec![OsString::from("-axo"), OsString::from("pid=,ppid=,rss=,comm=")] });
        assert_eq!(ps_command(Os::Linux), CommandSpec { program: PathBuf::from("ps"), args: vec![OsString::from("-eo"), OsString::from("pid=,ppid=,rss=,comm=")] });
    }

    #[test]
    fn app_data_lives_in_each_systems_user_data_folder() {
        let home = Path::new("/home/dev");
        assert_eq!(default_app_support(Path::new("/Users/dev"), Os::MacOs, None), PathBuf::from("/Users/dev/Library/Application Support/ClaudeCodeManager"));
        assert_eq!(default_app_support(home, Os::Linux, None), PathBuf::from("/home/dev/.local/share/ClaudeCodeManager"));
        assert_eq!(default_app_support(home, Os::Linux, Some(Path::new("/data/xdg"))), PathBuf::from("/data/xdg/ClaudeCodeManager"));
        assert_eq!(default_app_support(home, Os::Linux, Some(Path::new("relative/xdg"))), PathBuf::from("/home/dev/.local/share/ClaudeCodeManager"));
        assert_eq!(default_app_support(Path::new("/Users/dev"), Os::MacOs, Some(Path::new("/data/xdg"))), PathBuf::from("/Users/dev/Library/Application Support/ClaudeCodeManager"));
    }

    #[test]
    fn a_missing_opener_says_what_to_install() {
        let missing = CommandSpec { program: PathBuf::from("/nonexistent/xdg-open"), args: vec![] };
        let error = spawn(&missing).unwrap_err();
        assert!(matches!(&error, AppError::Invalid(message) if message.contains("xdg-open")), "{error}");
    }
}
