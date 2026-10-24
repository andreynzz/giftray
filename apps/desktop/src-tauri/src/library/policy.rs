// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::Path;

use super::model::{LibraryError, MediaFormat};

pub(super) const MAX_DISPLAY_NAME_CHARS: usize = 120;

pub(super) fn format_from_signature(bytes: &[u8]) -> Option<MediaFormat> {
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

/// Extracts `C:\\` from a final handle path such as `\\\\?\\C:\\media.gif`.
/// UNC, volume GUID and device namespaces have no approved local-drive root.
pub(super) fn extended_dos_drive_root(path: &str) -> Result<String, LibraryError> {
    let Some(rest) = path.strip_prefix(r"\\?\") else {
        return Err(LibraryError::SourceUnavailable);
    };
    let bytes = rest.as_bytes();
    if bytes.len() < 3 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' || bytes[2] != b'\\' {
        return Err(LibraryError::SourceUnavailable);
    }
    Ok(format!("{}:\\", bytes[0] as char))
}

pub(super) fn sanitized_name(source: &Path) -> Option<String> {
    let name = source.file_name()?.to_str()?.trim();
    if name.is_empty() || name.chars().any(char::is_control) {
        None
    } else {
        Some(name.chars().take(255).collect())
    }
}

pub(super) fn normalize_display_name(value: &str) -> Result<Option<String>, LibraryError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_extended_dos_drive_roots() {
        assert_eq!(
            extended_dos_drive_root(r"\\?\C:\Users\José\media.gif").unwrap(),
            "C:\\"
        );
        assert_eq!(
            extended_dos_drive_root(r"\\?\d:\media.png").unwrap(),
            "d:\\"
        );
        for invalid in [
            r"\\server\share\a.gif",
            r"\\?\Volume{guid}\a.gif",
            r"\\.\C:\a.gif",
            r"C:\a.gif",
            r"\\?\C:",
        ] {
            assert_eq!(
                extended_dos_drive_root(invalid).unwrap_err(),
                LibraryError::SourceUnavailable
            );
        }
    }
}
