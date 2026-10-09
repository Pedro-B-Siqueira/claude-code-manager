//! Images pasted or dropped into a session's terminal, kept in the app folder until Claude Code reads them.

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime};

use serde::Serialize;

use crate::error::AppError;
use crate::paths::AppPaths;

pub const MAX_ATTACHMENT_BYTES: u64 = 20 * 1024 * 1024;
/// Claude Code reads an image when it is pasted, so the file is only needed for a short while.
pub const RETENTION: Duration = Duration::from_secs(3 * 24 * 60 * 60);
const PRUNE_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const FOLDER: &str = "attachments";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageKind {
    Png,
    Jpeg,
    Gif,
    Webp,
}

impl ImageKind {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Gif => "gif",
            Self::Webp => "webp",
        }
    }

    fn from_extension(extension: &str) -> Option<Self> {
        [Self::Png, Self::Jpeg, Self::Gif, Self::Webp].into_iter().find(|kind| kind.extension() == extension)
    }
}

/// The formats Claude reads, recognized by their signature bytes.
pub fn sniff_image(bytes: &[u8]) -> Option<ImageKind> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(ImageKind::Png);
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(ImageKind::Jpeg);
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some(ImageKind::Gif);
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some(ImageKind::Webp);
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    pub path: PathBuf,
    pub kind: ImageKind,
    pub bytes: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentUsage {
    pub count: u32,
    pub bytes: u64,
}

#[derive(Debug, Clone)]
pub struct AttachmentStore {
    dir: PathBuf,
    paths: AppPaths,
}

impl AttachmentStore {
    pub fn open(paths: &AppPaths) -> Result<Self, AppError> {
        let dir = paths.app_support().join(FOLDER);
        paths.ensure_writable(&dir)?;
        fs::create_dir_all(&dir)?;
        Ok(Self { dir, paths: paths.clone() })
    }

    pub fn save(&self, bytes: &[u8]) -> Result<Attachment, AppError> {
        let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if size > MAX_ATTACHMENT_BYTES {
            return Err(too_large());
        }
        let kind = sniff_image(bytes).ok_or_else(unsupported)?;
        let id = format!("{}.{}", uuid::Uuid::new_v4().hyphenated(), kind.extension());
        let target = self.dir.join(&id);
        let temporary = self.dir.join(format!(".{id}.tmp"));
        self.paths.ensure_writable(&temporary)?;
        self.paths.ensure_writable(&target)?;
        let mut file = File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, &target)?;
        Ok(Attachment { id, path: target, kind, bytes: size })
    }

    /// Copies a dropped file, so cleanup never depends on where the original lives.
    pub fn import(&self, source: &Path) -> Result<Attachment, AppError> {
        let metadata = fs::metadata(source)?;
        if !metadata.is_file() {
            return Err(unsupported());
        }
        if metadata.len() > MAX_ATTACHMENT_BYTES {
            return Err(too_large());
        }
        let mut bytes = Vec::new();
        File::open(source)?.take(MAX_ATTACHMENT_BYTES + 1).read_to_end(&mut bytes)?;
        self.save(&bytes)
    }

    /// Only ids this store hands out (`<uuid>.<ext>`), so an id can never name a file elsewhere.
    pub fn path_of(&self, id: &str) -> Result<PathBuf, AppError> {
        let (stem, extension) = id.split_once('.').ok_or_else(invalid_id)?;
        let canonical = uuid::Uuid::parse_str(stem).map_err(|_| invalid_id())?.hyphenated().to_string();
        if canonical != stem || ImageKind::from_extension(extension).is_none() {
            return Err(invalid_id());
        }
        Ok(self.dir.join(id))
    }

    pub fn existing(&self, id: &str) -> Result<PathBuf, AppError> {
        let path = self.path_of(id)?;
        if path.is_file() {
            Ok(path)
        } else {
            Err(AppError::Invalid("a imagem não existe mais; anexe de novo".to_owned()))
        }
    }

    pub fn read(&self, id: &str) -> Result<Vec<u8>, AppError> {
        Ok(fs::read(self.existing(id)?)?)
    }

    pub fn remove(&self, id: &str) -> Result<(), AppError> {
        let path = self.path_of(id)?;
        self.delete(&path)
    }

    pub fn usage(&self) -> Result<AttachmentUsage, AppError> {
        let mut usage = AttachmentUsage::default();
        for (path, metadata) in self.entries()? {
            if !is_temporary(&path) {
                usage.count += 1;
                usage.bytes += metadata.len();
            }
        }
        Ok(usage)
    }

    /// Deletes every file, including half-written ones; returns what was freed.
    pub fn clear(&self) -> Result<AttachmentUsage, AppError> {
        let freed = self.usage()?;
        for (path, _) in self.entries()? {
            self.delete(&path)?;
        }
        Ok(freed)
    }

    pub fn prune_older_than(&self, max_age: Duration, now: SystemTime) -> Result<usize, AppError> {
        let mut removed = 0;
        for (path, metadata) in self.entries()? {
            let age = metadata.modified().ok().and_then(|modified| now.duration_since(modified).ok());
            if age.is_some_and(|age| age > max_age) {
                self.delete(&path)?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    fn entries(&self) -> Result<Vec<(PathBuf, fs::Metadata)>, AppError> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_file() {
                entries.push((entry.path(), metadata));
            }
        }
        Ok(entries)
    }

    fn delete(&self, path: &Path) -> Result<(), AppError> {
        self.paths.ensure_writable(path)?;
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}

fn is_temporary(path: &Path) -> bool {
    path.file_name().and_then(OsStr::to_str).is_some_and(|name| name.starts_with('.'))
}

fn too_large() -> AppError {
    AppError::Invalid("a imagem passa de 20 MB".to_owned())
}

fn unsupported() -> AppError {
    AppError::Invalid("formato não suportado: use PNG, JPEG, GIF ou WebP".to_owned())
}

fn invalid_id() -> AppError {
    AppError::Invalid("anexo inválido".to_owned())
}

/// Deletes attachments older than [`RETENTION`] now and every few hours.
pub fn start_pruning(store: AttachmentStore) {
    let spawned = thread::Builder::new().name("attachments-prune".to_owned()).spawn(move || loop {
        match store.prune_older_than(RETENTION, SystemTime::now()) {
            Ok(0) => {}
            Ok(removed) => log::info!("{removed} anexos antigos apagados"),
            Err(error) => log::warn!("falha ao limpar anexos antigos: {error}"),
        }
        thread::sleep(PRUNE_INTERVAL);
    });
    if let Err(error) = spawned {
        log::error!("falha ao iniciar a limpeza de anexos: {error}");
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::time::{Duration, SystemTime};

    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n-png-body";

    fn store() -> (tempfile::TempDir, AttachmentStore) {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        let paths = AppPaths::new(home.clone(), home.join(".claude"), home.join("Library/Application Support/ClaudeCodeManager"));
        let store = AttachmentStore::open(&paths).unwrap();
        (root, store)
    }

    #[test]
    fn sniffing_trusts_the_bytes_not_the_name() {
        assert_eq!(sniff_image(PNG), Some(ImageKind::Png));
        assert_eq!(sniff_image(&[0xFF, 0xD8, 0xFF, 0xE0]), Some(ImageKind::Jpeg));
        assert_eq!(sniff_image(b"GIF89a...."), Some(ImageKind::Gif));
        assert_eq!(sniff_image(b"RIFF\0\0\0\0WEBPVP8 "), Some(ImageKind::Webp));
        for refused in [&b""[..], b"hello", b"<svg xmlns=", b"\0\0\0\x18ftypheic", b"RIFF\0\0\0\0WAVE"] {
            assert_eq!(sniff_image(refused), None, "{}", String::from_utf8_lossy(refused));
        }
    }

    #[test]
    fn saved_images_live_in_the_attachments_folder() {
        let (_root, store) = store();
        let saved = store.save(PNG).unwrap();
        assert!(saved.id.ends_with(".png"));
        assert!(saved.path.ends_with(format!("attachments/{}", saved.id)));
        assert_eq!(store.read(&saved.id).unwrap(), PNG);
        assert_eq!(store.usage().unwrap(), AttachmentUsage { count: 1, bytes: PNG.len() as u64 });
    }

    #[test]
    fn oversized_and_unsupported_bytes_are_refused() {
        let (_root, store) = store();
        let mut huge = PNG.to_vec();
        huge.resize(usize::try_from(MAX_ATTACHMENT_BYTES).unwrap() + 1, 0);
        assert!(matches!(store.save(&huge), Err(AppError::Invalid(_))));
        assert!(matches!(store.save(b"plain text"), Err(AppError::Invalid(_))));
        assert_eq!(store.usage().unwrap().count, 0);
    }

    #[test]
    fn import_copies_real_images_and_refuses_impostors() {
        let (root, store) = store();
        let desktop = root.path().join("Área de Trabalho");
        fs::create_dir_all(&desktop).unwrap();
        fs::write(desktop.join("print da tela.png"), PNG).unwrap();
        fs::write(desktop.join("fake.png"), b"not an image").unwrap();
        let imported = store.import(&desktop.join("print da tela.png")).unwrap();
        assert_eq!(imported.kind, ImageKind::Png);
        assert!(desktop.join("print da tela.png").exists(), "the original stays where it was");
        assert!(matches!(store.import(&desktop.join("fake.png")), Err(AppError::Invalid(_))));
        assert!(matches!(store.import(&desktop), Err(AppError::Invalid(_))));
    }

    #[test]
    fn ids_cannot_point_outside_the_folder() {
        let (_root, store) = store();
        for hostile in ["../ccm.sqlite", "abc.png", "6f1c2a5e-0000-4000-8000-000000000000.exe", "{6f1c2a5e-0000-4000-8000-000000000000}.png", "6F1C2A5E-0000-4000-8000-000000000000.png"] {
            assert!(store.path_of(hostile).is_err(), "{hostile}");
        }
        assert!(store.path_of("6f1c2a5e-0000-4000-8000-000000000000.png").is_ok());
    }

    #[test]
    fn removing_is_idempotent_and_clear_empties_everything() {
        let (_root, store) = store();
        let first = store.save(PNG).unwrap();
        store.save(PNG).unwrap();
        store.remove(&first.id).unwrap();
        store.remove(&first.id).unwrap();
        assert!(store.existing(&first.id).is_err());
        let freed = store.clear().unwrap();
        assert_eq!(freed.count, 1);
        assert_eq!(store.usage().unwrap(), AttachmentUsage::default());
    }

    #[test]
    fn pruning_removes_only_old_files() {
        let (_root, store) = store();
        let old = store.save(PNG).unwrap();
        let fresh = store.save(PNG).unwrap();
        let four_days_ago = SystemTime::now() - Duration::from_secs(4 * 24 * 60 * 60);
        File::options().write(true).open(&old.path).unwrap().set_modified(four_days_ago).unwrap();
        assert_eq!(store.prune_older_than(RETENTION, SystemTime::now()).unwrap(), 1);
        assert!(store.existing(&old.id).is_err());
        assert!(store.existing(&fresh.id).is_ok());
    }

    #[test]
    fn the_folder_can_never_be_inside_claude_home() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().to_path_buf();
        let hostile = AppPaths::new(home.clone(), home.join(".claude"), home.join(".claude/app"));
        assert!(AttachmentStore::open(&hostile).is_err());
    }
}
