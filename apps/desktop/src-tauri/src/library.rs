// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Persistent, native-only local media library.
//!
//! This module deliberately has no Tauri command. A future native picker may
//! call `Library::import_local_file`, but paths received from the WebView must
//! never be routed here directly.

#![allow(dead_code)] // P1.1.1 establishes the internal API before the native picker exists.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Cursor, Read, Write},
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const MAX_MEDIA_BYTES: u64 = 25 * 1024 * 1024;
const LIBRARY_DIRECTORY: &str = "library";
const MEDIA_DIRECTORY: &str = "media";
const DATABASE_FILE: &str = "library.sqlite3";
const LOCK_FILE: &str = "library.lock";
const MAX_DISPLAY_NAME_CHARS: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaFormat {
    Gif,
    Webp,
    Png,
}

impl MediaFormat {
    fn as_str(self) -> &'static str {
        match self {
            Self::Gif => "gif",
            Self::Webp => "webp",
            Self::Png => "png",
        }
    }

    fn from_str(value: &str) -> Result<Self, LibraryError> {
        match value {
            "gif" => Ok(Self::Gif),
            "webp" => Ok(Self::Webp),
            "png" => Ok(Self::Png),
            _ => Err(LibraryError::Database),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryItem {
    pub id: String,
    pub format: MediaFormat,
    pub storage_key: String,
    pub byte_size: u64,
    pub content_sha256: String,
    pub source_kind: SourceKind,
    pub original_name: Option<String>,
    pub display_name: Option<String>,
    pub imported_at: i64,
    pub availability: Availability,
}

/// The IPC-safe projection of a persistent item. It intentionally excludes
/// storage keys, hashes and every filesystem path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicLibraryItem {
    pub id: String,
    pub format: String,
    pub byte_size: u64,
    pub source_kind: String,
    pub original_name: Option<String>,
    pub display_name: Option<String>,
    pub imported_at: i64,
    pub availability: String,
}

impl From<LibraryItem> for PublicLibraryItem {
    fn from(item: LibraryItem) -> Self {
        Self {
            id: item.id,
            format: item.format.as_str().into(),
            byte_size: item.byte_size,
            source_kind: item.source_kind.as_str().into(),
            original_name: item.original_name,
            display_name: item.display_name,
            imported_at: item.imported_at,
            availability: item.availability.as_str().into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    LocalFile,
    RemoteUrl,
    ExternalRequest,
}

impl SourceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::LocalFile => "local_file",
            Self::RemoteUrl => "remote_url",
            Self::ExternalRequest => "external_request",
        }
    }

    fn from_str(value: &str) -> Result<Self, LibraryError> {
        match value {
            "local_file" => Ok(Self::LocalFile),
            "remote_url" => Ok(Self::RemoteUrl),
            "external_request" => Ok(Self::ExternalRequest),
            _ => Err(LibraryError::Database),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Available,
    Missing,
}

impl Availability {
    fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Missing => "missing",
        }
    }

    fn from_str(value: &str) -> Result<Self, LibraryError> {
        match value {
            "available" => Ok(Self::Available),
            "missing" => Ok(Self::Missing),
            _ => Err(LibraryError::Database),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ImportResult {
    Imported { item: LibraryItem },
    AlreadyPresent { item: LibraryItem },
}

#[derive(Debug, PartialEq, Eq)]
pub enum LibraryError {
    InUse,
    InvalidId,
    NotFound,
    Missing,
    EmptyFile,
    TooLarge,
    InvalidMedia,
    InvalidDisplayName,
    SourceUnavailable,
    Storage,
    Database,
}

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
        let mut statement = connection
            .prepare("SELECT id, storage_key, format, byte_size, content_sha256, source_kind, original_name, display_name, imported_at, availability FROM library_items ORDER BY imported_at DESC, id ASC")
            .map_err(|_| LibraryError::Database)?;
        let rows = statement
            .query_map([], row_to_item)
            .map_err(|_| LibraryError::Database)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|_| LibraryError::Database)
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
        let changed = connection
            .execute(
                "UPDATE library_items SET display_name = ?1 WHERE id = ?2",
                params![display_name, id],
            )
            .map_err(|_| LibraryError::Database)?;
        if changed == 0 {
            return Err(LibraryError::NotFound);
        }
        query_item(&connection, "WHERE id = ?1", params![id])?.ok_or(LibraryError::NotFound)
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
            imported_at: unix_timestamp()?,
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
        let connection =
            Connection::open(self.root.join(DATABASE_FILE)).map_err(|_| LibraryError::Database)?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| LibraryError::Database)?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(|_| LibraryError::Database)?;
        Ok(connection)
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
        let mut statement = connection
            .prepare("SELECT id, storage_key FROM library_items")
            .map_err(|_| LibraryError::Database)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|_| LibraryError::Database)?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|_| LibraryError::Database)?;
        for row in rows {
            let (id, storage_key) = row.map_err(|_| LibraryError::Database)?;
            if !safe_storage_key(&id, &storage_key)
                || controlled_media_path(&self.media_directory(), &storage_key).is_err()
            {
                transaction
                    .execute(
                        "UPDATE library_items SET availability = 'missing' WHERE id = ?1",
                        params![id],
                    )
                    .map_err(|_| LibraryError::Database)?;
            }
        }
        transaction.commit().map_err(|_| LibraryError::Database)
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

fn migrate(connection: &Connection) -> Result<(), LibraryError> {
    connection.execute_batch("CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL);").map_err(|_| LibraryError::Database)?;
    let version_one: Option<i64> = connection
        .query_row(
            "SELECT version FROM schema_migrations WHERE version = 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| LibraryError::Database)?;
    if version_one.is_none() {
        let transaction = connection
            .unchecked_transaction()
            .map_err(|_| LibraryError::Database)?;
        transaction.execute_batch("CREATE TABLE library_items (id TEXT PRIMARY KEY, storage_key TEXT NOT NULL UNIQUE, format TEXT NOT NULL CHECK (format IN ('gif', 'webp', 'png')), byte_size INTEGER NOT NULL CHECK (byte_size > 0 AND byte_size <= 26214400), content_sha256 TEXT NOT NULL UNIQUE CHECK (length(content_sha256) = 64), source_kind TEXT NOT NULL CHECK (source_kind IN ('local_file', 'remote_url', 'external_request')), original_name TEXT NULL, imported_at INTEGER NOT NULL, availability TEXT NOT NULL CHECK (availability IN ('available', 'missing')));").map_err(|_| LibraryError::Database)?;
        transaction
            .execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (1, ?1)",
                params![unix_timestamp()?],
            )
            .map_err(|_| LibraryError::Database)?;
        transaction.commit().map_err(|_| LibraryError::Database)?;
    }
    let version_two: Option<i64> = connection
        .query_row(
            "SELECT version FROM schema_migrations WHERE version = 2",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| LibraryError::Database)?;
    if version_two.is_none() {
        let transaction = connection
            .unchecked_transaction()
            .map_err(|_| LibraryError::Database)?;
        transaction
            .execute_batch("ALTER TABLE library_items ADD COLUMN display_name TEXT NULL;")
            .map_err(|_| LibraryError::Database)?;
        transaction
            .execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (2, ?1)",
                params![unix_timestamp()?],
            )
            .map_err(|_| LibraryError::Database)?;
        transaction.commit().map_err(|_| LibraryError::Database)?;
    }
    Ok(())
}

fn copy_to_part(
    part: &Path,
    source: &mut dyn Read,
) -> Result<(MediaFormat, u64, String), LibraryError> {
    let mut output = File::create(part).map_err(|_| LibraryError::Storage)?;
    let mut buffer = [0u8; 8192];
    let mut prefix = Vec::with_capacity(12);
    let mut total = 0u64;
    let mut digest = Sha256::new();
    loop {
        let count = source
            .read(&mut buffer)
            .map_err(|_| LibraryError::SourceUnavailable)?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .ok_or(LibraryError::TooLarge)?;
        if total > MAX_MEDIA_BYTES {
            return Err(LibraryError::TooLarge);
        }
        if prefix.len() < 12 {
            prefix.extend_from_slice(&buffer[..count.min(12 - prefix.len())]);
        }
        output
            .write_all(&buffer[..count])
            .map_err(|_| LibraryError::Storage)?;
        digest.update(&buffer[..count]);
    }
    output.sync_all().map_err(|_| LibraryError::Storage)?;
    if total == 0 {
        return Err(LibraryError::EmptyFile);
    }
    let format = format_from_signature(&prefix).ok_or(LibraryError::InvalidMedia)?;
    Ok((format, total, format!("{:x}", digest.finalize())))
}

fn format_from_file(path: &Path) -> Result<MediaFormat, LibraryError> {
    let mut file = File::open(path).map_err(|_| LibraryError::Missing)?;
    let mut prefix = [0u8; 12];
    let count = file.read(&mut prefix).map_err(|_| LibraryError::Missing)?;
    format_from_signature(&prefix[..count]).ok_or(LibraryError::InvalidMedia)
}

fn publish_part(part: &Path, final_path: &Path) -> Result<(), LibraryError> {
    fs::rename(part, final_path).map_err(|_| LibraryError::Storage)
}

fn format_from_signature(bytes: &[u8]) -> Option<MediaFormat> {
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some(MediaFormat::Gif)
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(MediaFormat::Png)
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(MediaFormat::Webp)
    } else {
        None
    }
}

fn is_internal_part(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    let Some(stem) = name
        .strip_prefix('.')
        .and_then(|value| value.strip_suffix(".pending.part"))
    else {
        return false;
    };
    Uuid::parse_str(stem).is_ok()
}

fn safe_storage_key(id: &str, key: &str) -> bool {
    Path::new(key).components().count() == 1
        && Path::new(key)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && ["gif", "webp", "png"]
            .iter()
            .any(|extension| key == format!("{id}.{extension}"))
}

/// Resolves only a normal file directly below the native-owned media directory.
/// Links and Windows reparse points are never a valid persistent media item.
fn controlled_media_path(
    media_directory: &Path,
    storage_key: &str,
) -> Result<PathBuf, LibraryError> {
    let directory_metadata =
        fs::symlink_metadata(media_directory).map_err(|_| LibraryError::Missing)?;
    if !directory_metadata.file_type().is_dir() || is_reparse_point(&directory_metadata) {
        return Err(LibraryError::Storage);
    }

    let candidate = media_directory.join(storage_key);
    let metadata = fs::symlink_metadata(&candidate).map_err(|_| LibraryError::Missing)?;
    if !metadata.file_type().is_file() || is_reparse_point(&metadata) {
        return Err(LibraryError::Missing);
    }

    let directory = fs::canonicalize(media_directory).map_err(|_| LibraryError::Missing)?;
    let path = fs::canonicalize(&candidate).map_err(|_| LibraryError::Missing)?;
    if !path.starts_with(&directory) {
        return Err(LibraryError::Missing);
    }
    Ok(path)
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    metadata.file_attributes() & 0x0400 != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

fn sanitized_name(source: &Path) -> Option<String> {
    let name = source.file_name()?.to_str()?.trim();
    if name.is_empty() || name.chars().any(char::is_control) {
        None
    } else {
        Some(name.chars().take(255).collect())
    }
}

fn normalize_display_name(value: &str) -> Result<Option<String>, LibraryError> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Ok(None);
    }
    if normalized.chars().any(char::is_control)
        || normalized.chars().count() > MAX_DISPLAY_NAME_CHARS
    {
        return Err(LibraryError::InvalidDisplayName);
    }
    Ok(Some(normalized.into()))
}

fn unix_timestamp() -> Result<i64, LibraryError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| LibraryError::Database)
        .map(|value| value.as_secs() as i64)
}

fn insert_item(connection: &Connection, item: &LibraryItem) -> Result<(), LibraryError> {
    connection.execute("INSERT INTO library_items (id, storage_key, format, byte_size, content_sha256, source_kind, original_name, display_name, imported_at, availability) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)", params![item.id, item.storage_key, item.format.as_str(), item.byte_size as i64, item.content_sha256, item.source_kind.as_str(), item.original_name, item.display_name, item.imported_at, item.availability.as_str()]).map_err(|_| LibraryError::Database)?;
    Ok(())
}

fn query_item<P: rusqlite::Params>(
    connection: &Connection,
    filter: &str,
    parameters: P,
) -> Result<Option<LibraryItem>, LibraryError> {
    let sql = format!("SELECT id, storage_key, format, byte_size, content_sha256, source_kind, original_name, display_name, imported_at, availability FROM library_items {filter}");
    connection
        .query_row(&sql, parameters, row_to_item)
        .optional()
        .map_err(|_| LibraryError::Database)
}

fn row_to_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<LibraryItem> {
    let format = row.get::<_, String>(2)?;
    let source_kind = row.get::<_, String>(5)?;
    let availability = row.get::<_, String>(9)?;
    Ok(LibraryItem {
        id: row.get(0)?,
        storage_key: row.get(1)?,
        format: MediaFormat::from_str(&format).map_err(|_| rusqlite::Error::InvalidQuery)?,
        byte_size: row.get::<_, i64>(3)? as u64,
        content_sha256: row.get(4)?,
        source_kind: SourceKind::from_str(&source_kind)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        original_name: row.get(6)?,
        display_name: row.get(7)?,
        imported_at: row.get(8)?,
        availability: Availability::from_str(&availability)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
    })
}

#[cfg(test)]
mod tests {
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
                .rename_item(&item.id, "x".repeat(MAX_DISPLAY_NAME_CHARS + 1))
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
