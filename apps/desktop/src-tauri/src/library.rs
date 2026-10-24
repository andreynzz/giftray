// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Persistent, native-only local media library.
//!
//! This module deliberately has no Tauri command. A future native picker may
//! call `Library::import_local_file`, but paths received from the WebView must
//! never be routed here directly.

#![allow(dead_code)] // P1.1.1 establishes the internal API before the native picker exists.

mod external_source;
mod model;
mod paths;
mod policy;
mod repository;
mod storage;

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Cursor, Read},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use external_source::open_external_media_file;
pub use model::{
    Availability, ImportResult, LibraryError, LibraryItem, MediaFormat, PublicLibraryItem,
    SourceKind,
};
use paths::{controlled_media_path, is_reparse_point, safe_storage_key};
use policy::{normalize_display_name, sanitized_name};
use repository::{insert_item, migrate, query_item};
use storage::{copy_to_part, format_from_file, is_internal_part, publish_part};

pub const MAX_MEDIA_BYTES: u64 = 25 * 1024 * 1024;
const LIBRARY_DIRECTORY: &str = "library";
const MEDIA_DIRECTORY: &str = "media";
const DATABASE_FILE: &str = "library.sqlite3";
const LOCK_FILE: &str = "library.lock";
/// Native-only resolved source for a clipboard-cache copy.
#[derive(Debug)]
pub(crate) struct LibraryCopySource {
    pub(crate) path: PathBuf,
    pub(crate) file_name: String,
    pub(crate) byte_size: u64,
    pub(crate) content_sha256: String,
    pub(crate) format: MediaFormat,
}

pub(crate) struct LibraryMedia {
    pub(crate) bytes: Vec<u8>,
    pub(crate) format: MediaFormat,
}

/// Directory and database handle for the permanent local library.
#[derive(Debug, Clone)]
pub struct Library {
    root: PathBuf,
    // This handle is intentionally shared by clones. The operating system
    // releases it on process exit, while the last in-process clone releases it
    // during normal shutdown.
    _instance_lock: Arc<File>,
    operation_mutex: Arc<Mutex<()>>,
}

impl Library {
    /// Opens and recovers `<app_local_data_dir>/library`. This is native-only.
    pub fn initialize(app_local_data_dir: &Path) -> Result<Self, LibraryError> {
        let root = app_local_data_dir.join(LIBRARY_DIRECTORY);
        let media_directory = root.join(MEDIA_DIRECTORY);
        fs::create_dir_all(&media_directory).map_err(|_| LibraryError::Storage)?;
        // Acquire the process-wide lock before opening SQLite or running recovery.
        // A competing process must leave both the database and media tree untouched.
        let instance_lock =
            open_library_lock(&root.join(LOCK_FILE)).map_err(|_| LibraryError::InUse)?;
        let library = Self {
            root,
            _instance_lock: Arc::new(instance_lock),
            operation_mutex: Arc::new(Mutex::new(())),
        };
        {
            let _operation = library.operation_lock()?;
            let connection = library.open_connection()?;
            migrate(&connection)?;
            library.recover(&connection)?;
        }
        Ok(library)
    }

    /// Imports a local file selected by a trusted native flow.
    pub fn import_local_file(&self, source: &Path) -> Result<ImportResult, LibraryError> {
        self.import_local_file_with(
            source,
            |part, input| copy_to_part(part, input),
            publish_part,
        )
    }

    pub(crate) fn import_external_file(&self, source: &Path) -> Result<ImportResult, LibraryError> {
        let mut file = open_external_media_file(source)?;
        self.import_reader_with(
            &mut file,
            SourceKind::LocalFile,
            sanitized_name(source),
            copy_to_part,
            publish_part,
        )
    }

    /// Persists bytes already obtained by the approved remote downloader.
    /// The originating URL is deliberately not accepted or stored here.
    pub fn import_remote_bytes(&self, bytes: &[u8]) -> Result<ImportResult, LibraryError> {
        let mut source = Cursor::new(bytes);
        self.import_reader_with(
            &mut source,
            SourceKind::RemoteUrl,
            None,
            copy_to_part,
            publish_part,
        )
    }

    pub fn get_item(&self, id: &str) -> Result<Option<LibraryItem>, LibraryError> {
        let _operation = self.operation_lock()?;
        let connection = self.open_connection()?;
        migrate(&connection)?;
        query_item(&connection, "WHERE id = ?1", params![id])
    }

    pub fn list_items(&self) -> Result<Vec<LibraryItem>, LibraryError> {
        let _operation = self.operation_lock()?;
        let connection = self.open_connection()?;
        migrate(&connection)?;
        repository::list_items(&connection)
    }

    pub fn find_by_sha256(&self, sha256: &str) -> Result<Option<LibraryItem>, LibraryError> {
        let _operation = self.operation_lock()?;
        let connection = self.open_connection()?;
        migrate(&connection)?;
        query_item(&connection, "WHERE content_sha256 = ?1", params![sha256])
    }

    pub fn rename_item(&self, id: &str, display_name: String) -> Result<LibraryItem, LibraryError> {
        if Uuid::parse_str(id).is_err() {
            return Err(LibraryError::InvalidId);
        }
        let display_name = normalize_display_name(&display_name)?;
        let _operation = self.operation_lock()?;
        let connection = self.open_connection()?;
        migrate(&connection)?;
        let changed = repository::update_display_name(&connection, id, display_name)?;
        if changed == 0 {
            return Err(LibraryError::NotFound);
        }
        query_item(&connection, "WHERE id = ?1", params![id])?.ok_or(LibraryError::NotFound)
    }

    /// Deletes only the controlled file associated with an existing opaque ID.
    /// A missing controlled file is an intentional cleanup of its known record.
    pub fn delete_item(&self, id: &str) -> Result<(), LibraryError> {
        if Uuid::parse_str(id).is_err() {
            return Err(LibraryError::InvalidId);
        }
        let _operation = self.operation_lock()?;
        let connection = self.open_connection()?;
        migrate(&connection)?;
        let item =
            query_item(&connection, "WHERE id = ?1", params![id])?.ok_or(LibraryError::NotFound)?;
        if !safe_storage_key(&item.id, &item.storage_key) {
            return Err(LibraryError::Storage);
        }
        let media_directory = self.media_directory();
        let candidate = media_directory.join(&item.storage_key);
        match fs::symlink_metadata(&candidate) {
            Ok(metadata) => {
                if !metadata.file_type().is_file() || is_reparse_point(&metadata) {
                    return Err(LibraryError::Storage);
                }
                let path = controlled_media_path(&media_directory, &item.storage_key)?;
                fs::remove_file(path).map_err(|_| LibraryError::Storage)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(LibraryError::Storage),
        }
        let changed = repository::delete_item(&connection, id)?;
        if changed == 0 {
            return Err(LibraryError::NotFound);
        }
        Ok(())
    }

    pub(crate) fn resolve_copy_source(&self, id: &str) -> Result<LibraryCopySource, LibraryError> {
        if Uuid::parse_str(id).is_err() {
            return Err(LibraryError::InvalidId);
        }
        let _operation = self.operation_lock()?;
        let connection = self.open_connection()?;
        migrate(&connection)?;
        let item =
            query_item(&connection, "WHERE id = ?1", params![id])?.ok_or(LibraryError::NotFound)?;
        if item.availability != Availability::Available {
            return Err(LibraryError::Missing);
        }
        if !safe_storage_key(&item.id, &item.storage_key) {
            return Err(LibraryError::Storage);
        }
        let media_directory = self.media_directory();
        let path = controlled_media_path(&media_directory, &item.storage_key)?;
        let metadata = fs::metadata(&path).map_err(|_| LibraryError::Missing)?;
        if metadata.len() == 0 || metadata.len() > MAX_MEDIA_BYTES {
            return Err(LibraryError::Missing);
        }
        if metadata.len() != item.byte_size || format_from_file(&path)? != item.format {
            return Err(LibraryError::InvalidMedia);
        }
        Ok(LibraryCopySource {
            path,
            file_name: item.storage_key,
            byte_size: item.byte_size,
            content_sha256: item.content_sha256,
            format: item.format,
        })
    }

    pub(crate) fn read_media_for_preview(&self, id: &str) -> Result<LibraryMedia, LibraryError> {
        let source = self.resolve_copy_source(id)?;
        let bytes = fs::read(&source.path).map_err(|_| LibraryError::Missing)?;
        if bytes.len() as u64 != source.byte_size
            || format!("{:x}", Sha256::digest(&bytes)) != source.content_sha256
        {
            return Err(LibraryError::InvalidMedia);
        }
        Ok(LibraryMedia {
            bytes,
            format: source.format,
        })
    }

    fn import_local_file_with<Copy, Publish>(
        &self,
        source: &Path,
        copy: Copy,
        publish: Publish,
    ) -> Result<ImportResult, LibraryError>
    where
        Copy: FnOnce(&Path, &mut dyn Read) -> Result<(MediaFormat, u64, String), LibraryError>,
        Publish: FnOnce(&Path, &Path) -> Result<(), LibraryError>,
    {
        let mut source_file = File::open(source).map_err(|_| LibraryError::SourceUnavailable)?;
        self.import_reader_with(
            &mut source_file,
            SourceKind::LocalFile,
            sanitized_name(source),
            copy,
            publish,
        )
    }

    fn import_reader_with<Copy, Publish>(
        &self,
        source: &mut dyn Read,
        source_kind: SourceKind,
        original_name: Option<String>,
        copy: Copy,
        publish: Publish,
    ) -> Result<ImportResult, LibraryError>
    where
        Copy: FnOnce(&Path, &mut dyn Read) -> Result<(MediaFormat, u64, String), LibraryError>,
        Publish: FnOnce(&Path, &Path) -> Result<(), LibraryError>,
    {
        let _operation = self.operation_lock()?;
        let id = Uuid::new_v4();
        let part = self.media_directory().join(format!(".{id}.pending.part"));
        let (format, byte_size, digest) = match copy(&part, source) {
            Ok(value) => value,
            Err(error) => {
                let _ = fs::remove_file(&part);
                return Err(error);
            }
        };

        let connection = match self.open_connection().and_then(|connection| {
            migrate(&connection)?;
            Ok(connection)
        }) {
            Ok(connection) => connection,
            Err(error) => {
                let _ = fs::remove_file(&part);
                return Err(error);
            }
        };
        let existing = match query_item(&connection, "WHERE content_sha256 = ?1", params![&digest])
        {
            Ok(item) => item,
            Err(error) => {
                let _ = fs::remove_file(&part);
                return Err(error);
            }
        };
        if let Some(item) = existing {
            let _ = fs::remove_file(&part);
            return Ok(ImportResult::AlreadyPresent { item });
        }

        let storage_key = format!("{id}.{}", format.as_str());
        let final_path = self.media_directory().join(&storage_key);
        if publish(&part, &final_path).is_err() {
            let _ = fs::remove_file(&part);
            return Err(LibraryError::Storage);
        }

        let item = LibraryItem {
            id: id.to_string(),
            format,
            storage_key,
            byte_size,
            content_sha256: digest,
            source_kind,
            original_name,
            display_name: None,
            imported_at: repository::unix_timestamp()?,
            availability: Availability::Available,
        };
        if insert_item(&connection, &item).is_err() {
            // Do not remove a final file when cleanup itself fails; recovery preserves it.
            let _ = fs::remove_file(&final_path);
            return Err(LibraryError::Database);
        }
        Ok(ImportResult::Imported { item })
    }

    fn media_directory(&self) -> PathBuf {
        self.root.join(MEDIA_DIRECTORY)
    }

    fn open_connection(&self) -> Result<Connection, LibraryError> {
        repository::open_connection(&self.root)
    }

    fn operation_lock(&self) -> Result<OperationLock<'_>, LibraryError> {
        let in_process = self
            .operation_mutex
            .lock()
            .map_err(|_| LibraryError::InUse)?;
        Ok(OperationLock {
            _in_process: in_process,
        })
    }

    fn recover(&self, connection: &Connection) -> Result<(), LibraryError> {
        // Recovery is deliberately conservative: only our exact unpublished name is removed.
        for entry in fs::read_dir(self.media_directory()).map_err(|_| LibraryError::Storage)? {
            let entry = entry.map_err(|_| LibraryError::Storage)?;
            if entry
                .file_type()
                .map_err(|_| LibraryError::Storage)?
                .is_file()
                && is_internal_part(&entry.path())
            {
                fs::remove_file(entry.path()).map_err(|_| LibraryError::Storage)?;
            }
        }
        let mut missing_ids = Vec::new();
        for (id, storage_key) in repository::item_storage_keys(connection)? {
            if !safe_storage_key(&id, &storage_key)
                || controlled_media_path(&self.media_directory(), &storage_key).is_err()
            {
                missing_ids.push(id);
            }
        }
        repository::mark_missing(connection, &missing_ids)
    }
}

struct OperationLock<'a> {
    _in_process: std::sync::MutexGuard<'a, ()>,
}

#[cfg(windows)]
fn open_library_lock(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;

    // share_mode(0) maps to a CreateFile handle that no other process can
    // open. Windows closes it automatically after a crash or normal exit.
    OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .share_mode(0)
        .open(path)
}

#[cfg(not(windows))]
fn open_library_lock(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("giftray-library-{name}-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }
    fn source(root: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = root.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }
    fn imported(result: ImportResult) -> LibraryItem {
        match result {
            ImportResult::Imported { item } => item,
            _ => panic!("expected import"),
        }
    }

    #[test]
    fn initializes_empty_database_and_reapplies_migration() {
        let root = temp_root("migration");
        let library = Library::initialize(&root).unwrap();
        assert!(library.list_items().unwrap().is_empty());
        drop(library);
        let reopened = Library::initialize(&root).unwrap();
        drop(reopened);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn migrates_version_one_to_display_names_without_losing_items() {
        let root = temp_root("migration-v2");
        let library_root = root.join(LIBRARY_DIRECTORY);
        fs::create_dir_all(library_root.join(MEDIA_DIRECTORY)).unwrap();
        let connection = Connection::open(library_root.join(DATABASE_FILE)).unwrap();
        connection.execute_batch("CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL); INSERT INTO schema_migrations(version, applied_at) VALUES (1, 1); CREATE TABLE library_items (id TEXT PRIMARY KEY, storage_key TEXT NOT NULL UNIQUE, format TEXT NOT NULL, byte_size INTEGER NOT NULL, content_sha256 TEXT NOT NULL UNIQUE, source_kind TEXT NOT NULL, original_name TEXT NULL, imported_at INTEGER NOT NULL, availability TEXT NOT NULL);").unwrap();
        let id = Uuid::new_v4().to_string();
        connection.execute("INSERT INTO library_items (id, storage_key, format, byte_size, content_sha256, source_kind, original_name, imported_at, availability) VALUES (?1, ?2, 'gif', 7, ?3, 'local_file', 'legacy.gif', 1, 'available')", params![id, format!("{id}.gif"), "a".repeat(64)]).unwrap();
        drop(connection);

        let library = Library::initialize(&root).unwrap();
        let item = library.get_item(&id).unwrap().unwrap();
        assert_eq!(item.original_name, Some("legacy.gif".into()));
        assert_eq!(item.display_name, None);
        drop(library);
        let reopened = Library::initialize(&root).unwrap();
        assert_eq!(reopened.get_item(&id).unwrap().unwrap().display_name, None);
        drop(reopened);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn renames_and_removes_display_names_without_changing_media_identity() {
        let root = temp_root("rename");
        let library = Library::initialize(&root).unwrap();
        let source = source(&root, "original.gif", b"GIF89a rename");
        let item = imported(library.import_local_file(&source).unwrap());
        let renamed = library
            .rename_item(&item.id, "  Café reação  ".into())
            .unwrap();
        assert_eq!(renamed.display_name, Some("Café reação".into()));
        assert_eq!(renamed.original_name, item.original_name);
        assert_eq!(renamed.storage_key, item.storage_key);
        assert_eq!(renamed.content_sha256, item.content_sha256);
        assert_eq!(renamed.source_kind, item.source_kind);
        assert_eq!(
            library
                .rename_item(&item.id, "   ".into())
                .unwrap()
                .display_name,
            None
        );
        drop(library);
        let reopened = Library::initialize(&root).unwrap();
        assert_eq!(
            reopened.get_item(&item.id).unwrap().unwrap().display_name,
            None
        );
        drop(reopened);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn deletes_known_available_and_missing_items_without_touching_other_media() {
        let root = temp_root("delete");
        let library = Library::initialize(&root).unwrap();
        let first = imported(
            library
                .import_local_file(&source(&root, "first.gif", b"GIF89a first"))
                .unwrap(),
        );
        let second = imported(
            library
                .import_local_file(&source(&root, "second.png", b"\x89PNG\r\n\x1a\n second"))
                .unwrap(),
        );
        let first_path = library.media_directory().join(&first.storage_key);
        library.delete_item(&first.id).unwrap();
        assert!(!first_path.exists());
        assert!(library.get_item(&first.id).unwrap().is_none());
        fs::remove_file(library.media_directory().join(&second.storage_key)).unwrap();
        library.delete_item(&second.id).unwrap();
        assert!(library.get_item(&second.id).unwrap().is_none());
        assert_eq!(
            library.delete_item("not-a-uuid").unwrap_err(),
            LibraryError::InvalidId
        );
        drop(library);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn delete_reports_sqlite_failure_after_file_removal_and_recovers_as_missing() {
        let root = temp_root("delete-sqlite-failure");
        let library = Library::initialize(&root).unwrap();
        let item = imported(
            library
                .import_local_file(&source(&root, "item.gif", b"GIF89a delete"))
                .unwrap(),
        );
        let path = library.media_directory().join(&item.storage_key);
        let connection = Connection::open(library.root.join(DATABASE_FILE)).unwrap();
        connection.execute_batch("CREATE TRIGGER reject_library_delete BEFORE DELETE ON library_items BEGIN SELECT RAISE(ABORT, 'test'); END;").unwrap();
        drop(connection);
        assert_eq!(
            library.delete_item(&item.id).unwrap_err(),
            LibraryError::Database
        );
        assert!(!path.exists());
        assert!(library.get_item(&item.id).unwrap().is_some());
        drop(library);
        let reopened = Library::initialize(&root).unwrap();
        assert_eq!(
            reopened.get_item(&item.id).unwrap().unwrap().availability,
            Availability::Missing
        );
        let connection = Connection::open(reopened.root.join(DATABASE_FILE)).unwrap();
        connection
            .execute_batch("DROP TRIGGER reject_library_delete;")
            .unwrap();
        drop(connection);
        reopened.delete_item(&item.id).unwrap();
        assert!(reopened.get_item(&item.id).unwrap().is_none());
        drop(reopened);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_invalid_names_and_keeps_the_previous_name_on_sqlite_failure() {
        let root = temp_root("rename-errors");
        let library = Library::initialize(&root).unwrap();
        let source = source(&root, "original.gif", b"GIF89a errors");
        let item = imported(library.import_local_file(&source).unwrap());
        assert_eq!(
            library
                .rename_item("not-a-uuid", "name".into())
                .unwrap_err(),
            LibraryError::InvalidId
        );
        assert_eq!(
            library
                .rename_item(&Uuid::new_v4().to_string(), "name".into())
                .unwrap_err(),
            LibraryError::NotFound
        );
        assert_eq!(
            library
                .rename_item(&item.id, "bad\nname".into())
                .unwrap_err(),
            LibraryError::InvalidDisplayName
        );
        assert_eq!(
            library
                .rename_item(&item.id, "x".repeat(policy::MAX_DISPLAY_NAME_CHARS + 1),)
                .unwrap_err(),
            LibraryError::InvalidDisplayName
        );
        library.rename_item(&item.id, "Preservado".into()).unwrap();
        let connection = library.open_connection().unwrap();
        connection.execute_batch("CREATE TRIGGER reject_rename BEFORE UPDATE OF display_name ON library_items BEGIN SELECT RAISE(ABORT, 'test'); END;").unwrap();
        drop(connection);
        assert_eq!(
            library.rename_item(&item.id, "Falha".into()).unwrap_err(),
            LibraryError::Database
        );
        assert_eq!(
            library.get_item(&item.id).unwrap().unwrap().display_name,
            Some("Preservado".into())
        );
        drop(library);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn imports_supported_formats_preserving_bytes_hash_and_uuid() {
        let root = temp_root("formats");
        let library = Library::initialize(&root).unwrap();
        for (name, bytes, format) in [
            ("a.gif", b"GIF89a gif".as_slice(), MediaFormat::Gif),
            ("b.webp", b"RIFFxxxxWEBP webp".as_slice(), MediaFormat::Webp),
            (
                "c.png",
                b"\x89PNG\r\n\x1a\n png".as_slice(),
                MediaFormat::Png,
            ),
        ] {
            let original = source(&root, name, bytes);
            let item = imported(library.import_local_file(&original).unwrap());
            assert_eq!(item.format, format);
            assert!(Uuid::parse_str(&item.id).is_ok());
            assert_eq!(item.content_sha256.len(), 64);
            assert_eq!(
                fs::read(library.media_directory().join(&item.storage_key)).unwrap(),
                bytes
            );
        }
        drop(library);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn imports_validated_external_files_preserving_metadata_and_reopening() {
        let root = temp_root("external-formats");
        let library = Library::initialize(&root).unwrap();
        let mut ids = Vec::new();
        for (name, bytes, format) in [
            ("a.gif", b"GIF89a external gif".as_slice(), MediaFormat::Gif),
            (
                "b.webp",
                b"RIFFxxxxWEBP external webp".as_slice(),
                MediaFormat::Webp,
            ),
            (
                "c.png",
                b"\x89PNG\r\n\x1a\n external png".as_slice(),
                MediaFormat::Png,
            ),
        ] {
            let external = source(&root, name, bytes);
            let item = imported(library.import_external_file(&external).unwrap());
            assert_eq!(item.format, format);
            assert_eq!(item.source_kind, SourceKind::LocalFile);
            assert_eq!(item.original_name, Some(name.into()));
            assert_eq!(
                fs::read(library.media_directory().join(&item.storage_key)).unwrap(),
                bytes
            );
            ids.push(item.id);
        }
        let duplicate = root.join("a.gif");
        match library.import_external_file(&duplicate).unwrap() {
            ImportResult::AlreadyPresent { item } => assert_eq!(item.id, ids[0]),
            _ => panic!("expected duplicate"),
        }
        drop(library);
        let reopened = Library::initialize(&root).unwrap();
        assert_eq!(reopened.list_items().unwrap().len(), 3);
        drop(reopened);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn rejects_invalid_external_sources_without_persisting_an_item() {
        let root = temp_root("external-invalid");
        let library = Library::initialize(&root).unwrap();
        assert_eq!(
            library
                .import_external_file(Path::new("relative.gif"))
                .unwrap_err(),
            LibraryError::SourceUnavailable
        );
        assert_eq!(
            library
                .import_external_file(&root.join("missing.gif"))
                .unwrap_err(),
            LibraryError::SourceUnavailable
        );
        assert_eq!(
            library.import_external_file(&root).unwrap_err(),
            LibraryError::SourceUnavailable
        );
        let oversized = root.join("oversized.gif");
        let file = File::create(&oversized).unwrap();
        file.set_len(MAX_MEDIA_BYTES + 1).unwrap();
        assert_eq!(
            library.import_external_file(&oversized).unwrap_err(),
            LibraryError::SourceUnavailable
        );
        let invalid = source(&root, "invalid.gif", b"not media");
        assert_eq!(
            library.import_external_file(&invalid).unwrap_err(),
            LibraryError::InvalidMedia
        );
        assert!(library.list_items().unwrap().is_empty());
        assert_eq!(fs::read_dir(library.media_directory()).unwrap().count(), 0);
        drop(library);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn rejects_external_file_symlinks_without_reading_their_target() {
        use std::os::windows::fs::symlink_file;

        let root = temp_root("external-symlink");
        let library = Library::initialize(&root).unwrap();
        let target = source(&root, "target.gif", b"GIF89a external target");
        let link = root.join("link.gif");
        match symlink_file(&target, &link) {
            Ok(()) => {
                assert_eq!(
                    library.import_external_file(&link).unwrap_err(),
                    LibraryError::SourceUnavailable
                );
                assert!(library.list_items().unwrap().is_empty());
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::PermissionDenied
                    || error.raw_os_error() == Some(1314) =>
            {
                // Symlink creation needs Developer Mode or a privileged token on some Windows hosts.
            }
            Err(error) => panic!("failed to create test symlink: {error}"),
        }
        drop(library);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_keeps_existing_metadata_and_reopens_database() {
        let root = temp_root("duplicate");
        let source = source(&root, "reaction.gif", b"GIF89a same");
        let item = imported(
            Library::initialize(&root)
                .unwrap()
                .import_local_file(&source)
                .unwrap(),
        );
        let library = Library::initialize(&root).unwrap();
        match library.import_local_file(&source).unwrap() {
            ImportResult::AlreadyPresent { item: existing } => assert_eq!(existing.id, item.id),
            _ => panic!("expected duplicate"),
        }
        assert_eq!(library.get_item(&item.id).unwrap(), Some(item));
        drop(library);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn imports_remote_bytes_without_storing_the_url_and_reopens() {
        let root = temp_root("remote");
        let library = Library::initialize(&root).unwrap();
        for (bytes, format) in [
            (b"GIF89a remote".as_slice(), MediaFormat::Gif),
            (b"RIFFxxxxWEBP remote".as_slice(), MediaFormat::Webp),
            (b"\x89PNG\r\n\x1a\n remote".as_slice(), MediaFormat::Png),
        ] {
            let item = imported(library.import_remote_bytes(bytes).unwrap());
            assert_eq!(item.format, format);
            assert_eq!(item.source_kind, SourceKind::RemoteUrl);
            assert_eq!(item.original_name, None);
            assert_eq!(
                fs::read(library.media_directory().join(&item.storage_key)).unwrap(),
                bytes
            );
        }
        drop(library);
        let reopened = Library::initialize(&root).unwrap();
        assert_eq!(reopened.list_items().unwrap().len(), 3);
        drop(reopened);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn remote_duplicates_preserve_existing_local_metadata() {
        let root = temp_root("remote-duplicate");
        let library = Library::initialize(&root).unwrap();
        let local = source(&root, "local-name.gif", b"GIF89a same bytes");
        let item = imported(library.import_local_file(&local).unwrap());

        match library.import_remote_bytes(b"GIF89a same bytes").unwrap() {
            ImportResult::AlreadyPresent { item: existing } => {
                assert_eq!(existing.id, item.id);
                assert_eq!(existing.source_kind, SourceKind::LocalFile);
                assert_eq!(existing.original_name, Some("local-name.gif".into()));
            }
            _ => panic!("expected remote duplicate"),
        }
        match library.import_remote_bytes(b"GIF89a same bytes").unwrap() {
            ImportResult::AlreadyPresent { item: existing } => assert_eq!(existing.id, item.id),
            _ => panic!("expected repeated remote duplicate"),
        }
        drop(library);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_invalid_or_oversized_remote_bytes_without_persisting_files() {
        let root = temp_root("remote-invalid");
        let library = Library::initialize(&root).unwrap();
        assert_eq!(
            library
                .import_remote_bytes(b"<html>blocked</html>")
                .unwrap_err(),
            LibraryError::InvalidMedia
        );
        assert_eq!(
            library.import_remote_bytes(b"").unwrap_err(),
            LibraryError::EmptyFile
        );
        let mut oversized = b"GIF89a".to_vec();
        oversized.resize(MAX_MEDIA_BYTES as usize + 1, 0);
        assert_eq!(
            library.import_remote_bytes(&oversized).unwrap_err(),
            LibraryError::TooLarge
        );
        assert!(library.list_items().unwrap().is_empty());
        assert_eq!(fs::read_dir(library.media_directory()).unwrap().count(), 0);
        drop(library);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn remote_sqlite_failure_removes_the_published_file_and_record() {
        let root = temp_root("remote-database-failure");
        let library = Library::initialize(&root).unwrap();
        let connection = library.open_connection().unwrap();
        connection
            .execute_batch(
                "CREATE TRIGGER reject_remote_insert BEFORE INSERT ON library_items BEGIN SELECT RAISE(ABORT, 'test'); END;",
            )
            .unwrap();
        drop(connection);

        assert_eq!(
            library
                .import_remote_bytes(b"GIF89a remote database")
                .unwrap_err(),
            LibraryError::Database
        );
        assert!(library.list_items().unwrap().is_empty());
        assert_eq!(fs::read_dir(library.media_directory()).unwrap().count(), 0);
        drop(library);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_invalid_empty_and_oversized_files() {
        let root = temp_root("invalid");
        let library = Library::initialize(&root).unwrap();
        assert_eq!(
            library
                .import_local_file(&source(&root, "bad.bin", b"not media"))
                .unwrap_err(),
            LibraryError::InvalidMedia
        );
        assert_eq!(
            library
                .import_local_file(&source(&root, "empty.gif", b""))
                .unwrap_err(),
            LibraryError::EmptyFile
        );
        let large = root.join("large.gif");
        let mut file = File::create(&large).unwrap();
        file.write_all(b"GIF89a").unwrap();
        file.set_len(MAX_MEDIA_BYTES + 1).unwrap();
        assert_eq!(
            library.import_local_file(&large).unwrap_err(),
            LibraryError::TooLarge
        );
        drop(library);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resolves_copy_source_only_for_a_valid_available_controlled_item() {
        let root = temp_root("copy-source");
        let library = Library::initialize(&root).unwrap();
        let input = source(&root, "reaction.gif", b"GIF89a bytes");
        let item = imported(library.import_local_file(&input).unwrap());

        let resolved = library.resolve_copy_source(&item.id).unwrap();
        assert_eq!(resolved.file_name, item.storage_key);
        assert_eq!(fs::read(resolved.path).unwrap(), b"GIF89a bytes");
        assert_eq!(
            library.resolve_copy_source("not-a-uuid").unwrap_err(),
            LibraryError::InvalidId
        );
        assert_eq!(
            library
                .resolve_copy_source(&Uuid::new_v4().to_string())
                .unwrap_err(),
            LibraryError::NotFound
        );

        let connection = library.open_connection().unwrap();
        connection
            .execute(
                "UPDATE library_items SET availability = 'missing' WHERE id = ?1",
                params![item.id],
            )
            .unwrap();
        drop(connection);
        assert_eq!(
            library.resolve_copy_source(&item.id).unwrap_err(),
            LibraryError::Missing
        );

        fs::remove_file(library.media_directory().join(&item.storage_key)).unwrap();
        assert_eq!(
            library.resolve_copy_source(&item.id).unwrap_err(),
            LibraryError::Missing
        );
        drop(library);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn rejects_media_symlinks_that_target_outside_the_library() {
        use std::os::windows::fs::symlink_file;

        let root = temp_root("linked-media");
        let library = Library::initialize(&root).unwrap();
        let input = source(&root, "reaction.gif", b"GIF89a bytes");
        let item = imported(library.import_local_file(&input).unwrap());
        let outside = source(&root, "outside.gif", b"GIF89a other");
        let media_path = library.media_directory().join(&item.storage_key);
        fs::remove_file(&media_path).unwrap();
        if let Err(error) = symlink_file(&outside, &media_path) {
            if error.kind() == io::ErrorKind::PermissionDenied || error.raw_os_error() == Some(1314)
            {
                drop(library);
                fs::remove_dir_all(root).unwrap();
                return;
            }
            panic!("failed to create symlink: {error}");
        }

        assert_eq!(
            library.resolve_copy_source(&item.id).unwrap_err(),
            LibraryError::Missing
        );
        drop(library);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_copy_source_with_changed_metadata_or_signature() {
        let root = temp_root("copy-source-integrity");
        let library = Library::initialize(&root).unwrap();
        let input = source(&root, "reaction.gif", b"GIF89a bytes");
        let item = imported(library.import_local_file(&input).unwrap());
        let path = library.media_directory().join(&item.storage_key);
        fs::write(&path, b"not media!!!").unwrap();
        assert_eq!(
            library.resolve_copy_source(&item.id).unwrap_err(),
            LibraryError::InvalidMedia
        );

        let connection = library.open_connection().unwrap();
        connection
            .execute(
                "UPDATE library_items SET storage_key = 'outside.gif' WHERE id = ?1",
                params![item.id],
            )
            .unwrap();
        assert_eq!(
            library.resolve_copy_source(&item.id).unwrap_err(),
            LibraryError::Storage
        );
        drop(connection);
        drop(library);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cleans_part_after_write_or_publish_failure() {
        let root = temp_root("failures");
        let library = Library::initialize(&root).unwrap();
        let mut first_source = Cursor::new(b"GIF89a ok".as_slice());
        assert_eq!(
            library
                .import_reader_with(
                    &mut first_source,
                    SourceKind::RemoteUrl,
                    None,
                    |part, _| {
                        fs::write(part, b"partial").unwrap();
                        Err(LibraryError::Storage)
                    },
                    publish_part
                )
                .unwrap_err(),
            LibraryError::Storage
        );
        let mut second_source = Cursor::new(b"GIF89a ok".as_slice());
        assert_eq!(
            library
                .import_reader_with(
                    &mut second_source,
                    SourceKind::RemoteUrl,
                    None,
                    copy_to_part,
                    |_, _| Err(LibraryError::Storage),
                )
                .unwrap_err(),
            LibraryError::Storage
        );
        assert_eq!(fs::read_dir(library.media_directory()).unwrap().count(), 0);
        drop(library);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recovery_removes_our_parts_marks_missing_and_preserves_orphans() {
        let root = temp_root("recovery");
        let library = Library::initialize(&root).unwrap();
        let input = source(&root, "ok.gif", b"GIF89a ok");
        let item = imported(library.import_local_file(&input).unwrap());
        fs::remove_file(library.media_directory().join(&item.storage_key)).unwrap();
        let part = library
            .media_directory()
            .join(format!(".{}.pending.part", Uuid::new_v4()));
        fs::write(&part, b"partial").unwrap();
        let orphan = library
            .media_directory()
            .join(format!("{}.gif", Uuid::new_v4()));
        fs::write(&orphan, b"GIF89a orphan").unwrap();
        let unknown = library.media_directory().join("foreign.part");
        fs::write(&unknown, b"keep").unwrap();
        drop(library);
        let recovered = Library::initialize(&root).unwrap();
        assert!(!part.exists());
        assert!(orphan.exists());
        assert!(unknown.exists());
        assert_eq!(
            recovered.get_item(&item.id).unwrap().unwrap().availability,
            Availability::Missing
        );
        drop(recovered);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn list_is_ordered_and_lock_is_exclusive() {
        let root = temp_root("locking");
        let library = Library::initialize(&root).unwrap();
        imported(
            library
                .import_local_file(&source(&root, "a.gif", b"GIF89a a"))
                .unwrap(),
        );
        imported(
            library
                .import_local_file(&source(&root, "b.gif", b"GIF89a b"))
                .unwrap(),
        );
        let items = library.list_items().unwrap();
        assert!(items.windows(2).all(|pair| {
            pair[0].imported_at > pair[1].imported_at
                || (pair[0].imported_at == pair[1].imported_at && pair[0].id <= pair[1].id)
        }));
        drop(library);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lock_is_exclusive_between_processes() {
        if let Some(root) = std::env::var_os("GIFTRAY_LIBRARY_LOCK_HELPER") {
            let root = PathBuf::from(root);
            let part = PathBuf::from(std::env::var_os("GIFTRAY_LIBRARY_LOCK_PART").unwrap());
            let database = root.join(LIBRARY_DIRECTORY).join(DATABASE_FILE);
            let database_before = fs::read(&database).unwrap();
            assert_eq!(
                Library::initialize(Path::new(&root)).unwrap_err(),
                LibraryError::InUse
            );
            assert_eq!(fs::read(part).unwrap(), b"must-survive");
            assert_eq!(fs::read(database).unwrap(), database_before);
            return;
        }

        let root = temp_root("process-lock");
        let library = Library::initialize(&root).unwrap();
        let part = library
            .media_directory()
            .join(format!(".{}.pending.part", Uuid::new_v4()));
        fs::write(&part, b"must-survive").unwrap();
        let retained_clone = library.clone();
        drop(library);
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "library::tests::lock_is_exclusive_between_processes",
            ])
            .env("GIFTRAY_LIBRARY_LOCK_HELPER", &root)
            .env("GIFTRAY_LIBRARY_LOCK_PART", &part)
            .status()
            .unwrap();
        assert!(status.success());
        assert!(part.exists());
        drop(retained_clone);
        let reopened = Library::initialize(&root).unwrap();
        assert!(!part.exists());
        drop(reopened);
        fs::remove_dir_all(root).unwrap();
    }
}
