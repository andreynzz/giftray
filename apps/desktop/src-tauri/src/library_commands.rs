// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Narrow Tauri boundary for the persistent library.

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::library::{ImportResult, Library, LibraryError, PublicLibraryItem};

pub enum LibraryRuntime {
    Ready(Library),
    InUse,
}

impl LibraryRuntime {
    pub fn library(&self) -> Result<Library, String> {
        match self {
            Self::Ready(library) => Ok(library.clone()),
            Self::InUse => Err("A biblioteca está em uso por outra instância.".into()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum PublicImportResult {
    Imported { item: PublicLibraryItem },
    AlreadyPresent { item: PublicLibraryItem },
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum PublicClipboardImportResult {
    NoFiles,
    Completed {
        imported: u32,
        already_present: u32,
        failed: u32,
        items: Vec<PublicLibraryItem>,
    },
}

/// Opens the native picker. No path crosses the WebView IPC boundary.
#[tauri::command]
pub async fn select_and_import_media(
    app: AppHandle,
    library: State<'_, LibraryRuntime>,
) -> Result<PublicImportResult, String> {
    let library = library.library()?;
    let selected = app
        .dialog()
        .file()
        .add_filter("Mídia", &["gif", "webp", "png"])
        .blocking_pick_file();
    let Some(selected) = selected else {
        return Ok(PublicImportResult::Cancelled);
    };
    let path: PathBuf = selected
        .into_path()
        .map_err(|_| "Não foi possível abrir o arquivo selecionado.")?;
    tauri::async_runtime::spawn_blocking(move || library.import_local_file(&path))
        .await
        .map_err(|_| "Não foi possível concluir a importação.")?
        .map(public_import_result)
        .map_err(sanitized_error)
}

/// Downloads through the P0 policy before acquiring the library operation lock.
/// The original URL never reaches SQLite, the filesystem metadata or the WebView result.
#[tauri::command]
pub async fn import_remote_media(
    library: State<'_, LibraryRuntime>,
    url: String,
) -> Result<PublicImportResult, String> {
    let library = library.library()?;
    let bytes = crate::remote_media::download(&url)
        .await
        .map_err(|_| "Não foi possível baixar a mídia da URL informada.")?;
    tauri::async_runtime::spawn_blocking(move || library.import_remote_bytes(&bytes))
        .await
        .map_err(|_| "Não foi possível concluir a importação da URL.")?
        .map(public_import_result)
        .map_err(sanitized_error)
}

/// Reads CF_HDROP directly from the native clipboard. No path crosses IPC.
#[tauri::command]
pub async fn import_clipboard_files(
    library: State<'_, LibraryRuntime>,
) -> Result<PublicClipboardImportResult, String> {
    let library = library.library()?;
    tauri::async_runtime::spawn_blocking(move || match crate::clipboard::clipboard_file_paths()? {
        crate::clipboard::ClipboardFilePaths::NoFiles => Ok(PublicClipboardImportResult::NoFiles),
        crate::clipboard::ClipboardFilePaths::Paths(paths) => {
            Ok(import_external_paths(&library, &paths))
        }
    })
    .await
    .map_err(|_| "Não foi possível concluir a importação da área de transferência.")?
}

#[tauri::command]
pub async fn list_library_items(
    library: State<'_, LibraryRuntime>,
) -> Result<Vec<PublicLibraryItem>, String> {
    let library = library.library()?;
    tauri::async_runtime::spawn_blocking(move || library.list_items())
        .await
        .map_err(|_| "Não foi possível consultar a biblioteca.")?
        .map(|items| items.into_iter().map(PublicLibraryItem::from).collect())
        .map_err(sanitized_error)
}

/// Updates only the optional display name of one opaque library item.
#[tauri::command]
pub async fn rename_library_item(
    library: State<'_, LibraryRuntime>,
    id: String,
    display_name: String,
) -> Result<PublicLibraryItem, String> {
    let library = library.library()?;
    tauri::async_runtime::spawn_blocking(move || library.rename_item(&id, display_name))
        .await
        .map_err(|_| "Não foi possível renomear o arquivo da biblioteca.")?
        .map(PublicLibraryItem::from)
        .map_err(sanitized_rename_error)
}

/// Deletes one known persistent item by opaque UUID only.
#[tauri::command]
pub async fn delete_library_item(
    library: State<'_, LibraryRuntime>,
    id: String,
) -> Result<(), String> {
    let library = library.library()?;
    tauri::async_runtime::spawn_blocking(move || library.delete_item(&id))
        .await
        .map_err(|_| "Não foi possível excluir o arquivo da biblioteca.")?
        .map_err(sanitized_delete_error)
}

/// Copies one persistent item by opaque ID. Paths and bytes stay native-only.
#[tauri::command]
pub async fn copy_library_item(
    app: AppHandle,
    library: State<'_, LibraryRuntime>,
    id: String,
) -> Result<(), String> {
    let library = library.library()?;
    tauri::async_runtime::spawn_blocking(move || {
        let source = library.resolve_copy_source(&id)?;
        crate::clipboard::copy_library_file(app, source).map_err(|_| LibraryError::Storage)
    })
    .await
    .map_err(|_| "Não foi possível copiar o arquivo da biblioteca.")?
    .map_err(sanitized_copy_error)
}

fn public_import_result(result: ImportResult) -> PublicImportResult {
    match result {
        ImportResult::Imported { item } => PublicImportResult::Imported { item: item.into() },
        ImportResult::AlreadyPresent { item } => {
            PublicImportResult::AlreadyPresent { item: item.into() }
        }
    }
}

pub(crate) fn import_external_paths(
    library: &Library,
    paths: &[PathBuf],
) -> PublicClipboardImportResult {
    if paths.len() > 20 {
        return PublicClipboardImportResult::Completed {
            imported: 0,
            already_present: 0,
            failed: paths.len() as u32,
            items: Vec::new(),
        };
    }
    let mut imported = 0;
    let mut already_present = 0;
    let mut failed = 0;
    let mut items = Vec::new();
    for path in paths {
        match library.import_external_file(Path::new(path)) {
            Ok(ImportResult::Imported { item }) => {
                imported += 1;
                items.push(item.into());
            }
            Ok(ImportResult::AlreadyPresent { item }) => {
                already_present += 1;
                items.push(item.into());
            }
            Err(_) => failed += 1,
        }
    }
    PublicClipboardImportResult::Completed {
        imported,
        already_present,
        failed,
        items,
    }
}

fn sanitized_error(error: LibraryError) -> String {
    match error {
        LibraryError::InUse => "A biblioteca está em uso por outra instância.".into(),
        LibraryError::InvalidId | LibraryError::NotFound | LibraryError::Missing => {
            "O arquivo solicitado não está disponível na biblioteca.".into()
        }
        LibraryError::EmptyFile | LibraryError::InvalidMedia => {
            "O arquivo não possui um formato GIF, WebP ou PNG confirmado.".into()
        }
        LibraryError::InvalidDisplayName => "O nome informado não é válido.".into(),
        LibraryError::TooLarge => "O arquivo excede o limite de 25 MB.".into(),
        LibraryError::SourceUnavailable => "Não foi possível abrir o arquivo selecionado.".into(),
        LibraryError::Storage | LibraryError::Database => {
            "Não foi possível concluir a importação da biblioteca.".into()
        }
    }
}

fn sanitized_copy_error(error: LibraryError) -> String {
    match error {
        LibraryError::InUse => "A biblioteca está em uso por outra instância.".into(),
        LibraryError::InvalidId | LibraryError::NotFound => {
            "O arquivo solicitado não foi encontrado na biblioteca.".into()
        }
        LibraryError::Missing => "Esse arquivo não está mais disponível na biblioteca.".into(),
        LibraryError::TooLarge => "O arquivo excede o limite de 25 MB.".into(),
        LibraryError::InvalidMedia => {
            "O arquivo não possui um formato GIF, WebP ou PNG confirmado.".into()
        }
        LibraryError::EmptyFile
        | LibraryError::SourceUnavailable
        | LibraryError::Storage
        | LibraryError::Database
        | LibraryError::InvalidDisplayName => {
            "Não foi possível copiar o arquivo da biblioteca.".into()
        }
    }
}

fn sanitized_rename_error(error: LibraryError) -> String {
    match error {
        LibraryError::InUse => "A biblioteca está em uso por outra instância.".into(),
        LibraryError::InvalidId | LibraryError::NotFound => {
            "O arquivo solicitado não foi encontrado na biblioteca.".into()
        }
        LibraryError::InvalidDisplayName => "O nome informado não é válido.".into(),
        _ => "Não foi possível renomear o arquivo da biblioteca.".into(),
    }
}

fn sanitized_delete_error(error: LibraryError) -> String {
    match error {
        LibraryError::InUse => "A biblioteca está em uso por outra instância.".into(),
        LibraryError::InvalidId | LibraryError::NotFound => {
            "O arquivo solicitado não foi encontrado na biblioteca.".into()
        }
        _ => "Não foi possível excluir o arquivo da biblioteca. Tente novamente.".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{Availability, LibraryItem, MediaFormat, SourceKind};

    fn item() -> LibraryItem {
        LibraryItem {
            id: "id".into(),
            format: MediaFormat::Gif,
            storage_key: "private.gif".into(),
            byte_size: 7,
            content_sha256: "a".repeat(64),
            source_kind: SourceKind::LocalFile,
            original_name: Some("reaction.gif".into()),
            display_name: None,
            imported_at: 1,
            availability: Availability::Available,
        }
    }

    #[test]
    fn public_contracts_hide_private_storage_fields() {
        let value = serde_json::to_value(public_import_result(ImportResult::Imported {
            item: item(),
        }))
        .unwrap();
        assert_eq!(value["status"], "imported");
        assert!(value["item"].get("storageKey").is_none());
        assert!(value["item"].get("contentSha256").is_none());
        assert!(value["item"].get("displayName").is_some());
    }

    #[test]
    fn serializes_duplicate_and_cancelled_results() {
        let duplicate = serde_json::to_value(public_import_result(ImportResult::AlreadyPresent {
            item: item(),
        }))
        .unwrap();
        assert_eq!(duplicate["status"], "alreadyPresent");
        assert_eq!(
            serde_json::to_value(PublicImportResult::Cancelled).unwrap()["status"],
            "cancelled"
        );
    }

    #[test]
    fn remote_failures_are_sanitized() {
        let message = "Não foi possível baixar a mídia da URL informada.";
        assert!(!message.contains("https://"));
        assert!(!message.contains("token"));
    }
}
