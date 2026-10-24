// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OptionalExtension};

use super::model::{Availability, LibraryError, LibraryItem, MediaFormat, SourceKind};
use super::DATABASE_FILE;

pub(super) fn open_connection(root: &Path) -> Result<Connection, LibraryError> {
    let connection =
        Connection::open(root.join(DATABASE_FILE)).map_err(|_| LibraryError::Database)?;
    connection
        .busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| LibraryError::Database)?;
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(|_| LibraryError::Database)?;
    Ok(connection)
}

pub(super) fn migrate(connection: &Connection) -> Result<(), LibraryError> {
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

pub(super) fn insert_item(connection: &Connection, item: &LibraryItem) -> Result<(), LibraryError> {
    connection.execute("INSERT INTO library_items (id, storage_key, format, byte_size, content_sha256, source_kind, original_name, display_name, imported_at, availability) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)", params![item.id, item.storage_key, item.format.as_str(), item.byte_size as i64, item.content_sha256, item.source_kind.as_str(), item.original_name, item.display_name, item.imported_at, item.availability.as_str()]).map_err(|_| LibraryError::Database)?;
    Ok(())
}

pub(super) fn query_item<P: rusqlite::Params>(
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

pub(super) fn list_items(connection: &Connection) -> Result<Vec<LibraryItem>, LibraryError> {
    let mut statement = connection
        .prepare("SELECT id, storage_key, format, byte_size, content_sha256, source_kind, original_name, display_name, imported_at, availability FROM library_items ORDER BY imported_at DESC, id ASC")
        .map_err(|_| LibraryError::Database)?;
    let rows = statement
        .query_map([], row_to_item)
        .map_err(|_| LibraryError::Database)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|_| LibraryError::Database)
}

pub(super) fn update_display_name(
    connection: &Connection,
    id: &str,
    display_name: Option<String>,
) -> Result<usize, LibraryError> {
    connection
        .execute(
            "UPDATE library_items SET display_name = ?1 WHERE id = ?2",
            params![display_name, id],
        )
        .map_err(|_| LibraryError::Database)
}

pub(super) fn delete_item(connection: &Connection, id: &str) -> Result<usize, LibraryError> {
    connection
        .execute("DELETE FROM library_items WHERE id = ?1", params![id])
        .map_err(|_| LibraryError::Database)
}

pub(super) fn item_storage_keys(
    connection: &Connection,
) -> Result<Vec<(String, String)>, LibraryError> {
    let mut statement = connection
        .prepare("SELECT id, storage_key FROM library_items")
        .map_err(|_| LibraryError::Database)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|_| LibraryError::Database)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|_| LibraryError::Database)
}

pub(super) fn mark_missing(connection: &Connection, ids: &[String]) -> Result<(), LibraryError> {
    let transaction = connection
        .unchecked_transaction()
        .map_err(|_| LibraryError::Database)?;
    for id in ids {
        transaction
            .execute(
                "UPDATE library_items SET availability = 'missing' WHERE id = ?1",
                params![id],
            )
            .map_err(|_| LibraryError::Database)?;
    }
    transaction.commit().map_err(|_| LibraryError::Database)
}

pub(super) fn unix_timestamp() -> Result<i64, LibraryError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| LibraryError::Database)
        .map(|value| value.as_secs() as i64)
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
