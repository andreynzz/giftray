// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaFormat {
    Gif,
    Webp,
    Png,
}

impl MediaFormat {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Gif => "gif",
            Self::Webp => "webp",
            Self::Png => "png",
        }
    }

    pub(super) fn from_str(value: &str) -> Result<Self, LibraryError> {
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
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::LocalFile => "local_file",
            Self::RemoteUrl => "remote_url",
            Self::ExternalRequest => "external_request",
        }
    }

    pub(super) fn from_str(value: &str) -> Result<Self, LibraryError> {
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
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Missing => "missing",
        }
    }

    pub(super) fn from_str(value: &str) -> Result<Self, LibraryError> {
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
