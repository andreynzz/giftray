// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{
    model::{LibraryError, MediaFormat},
    policy::format_from_signature,
    MAX_MEDIA_BYTES,
};

pub(super) fn copy_to_part(
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

pub(super) fn format_from_file(path: &Path) -> Result<MediaFormat, LibraryError> {
    let mut file = File::open(path).map_err(|_| LibraryError::Missing)?;
    let mut prefix = [0u8; 12];
    let count = file.read(&mut prefix).map_err(|_| LibraryError::Missing)?;
    format_from_signature(&prefix[..count]).ok_or(LibraryError::InvalidMedia)
}

pub(super) fn publish_part(part: &Path, final_path: &Path) -> Result<(), LibraryError> {
    fs::rename(part, final_path).map_err(|_| LibraryError::Storage)
}

pub(super) fn is_internal_part(path: &Path) -> bool {
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
