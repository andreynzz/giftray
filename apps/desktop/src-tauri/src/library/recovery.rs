// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{fs, path::Path};

use rusqlite::Connection;

use super::{
    model::LibraryError,
    paths::{controlled_media_path, safe_storage_key},
    repository,
    storage::is_internal_part,
};

pub(super) fn recover(media_directory: &Path, connection: &Connection) -> Result<(), LibraryError> {
    // Recovery is deliberately conservative: only our exact unpublished name is removed.
    for entry in fs::read_dir(media_directory).map_err(|_| LibraryError::Storage)? {
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
            || controlled_media_path(media_directory, &storage_key).is_err()
        {
            missing_ids.push(id);
        }
    }
    repository::mark_missing(connection, &missing_ids)
}
