// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use super::model::LibraryError;

pub(super) fn safe_storage_key(id: &str, key: &str) -> bool {
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
pub(super) fn controlled_media_path(
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
pub(super) fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    metadata.file_attributes() & 0x0400 != 0
}

#[cfg(not(windows))]
pub(super) fn is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}
