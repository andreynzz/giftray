// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Windows file clipboard support. CF_HDROP owns only the path list: retained
//! cache files must outlive the clipboard operation and a receiving process.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use tauri::{AppHandle, Manager};

const MAX_MEDIA_BYTES: usize = 25 * 1024 * 1024;
const CACHE_MAX_BYTES: u64 = 100 * 1024 * 1024;
const CACHE_RETENTION: Duration = Duration::from_secs(24 * 60 * 60);
const CACHE_DIRECTORY: &str = "clipboard-output";

fn valid_media(file_name: &str, bytes: &[u8]) -> bool {
    let extension = Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some("gif") => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        Some("png") => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        Some("webp") => bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP",
        _ => false,
    }
}

fn cache_directory(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_cache_dir()
        .map_err(|_| "Não foi possível localizar o cache local.")?
        .join(CACHE_DIRECTORY);
    fs::create_dir_all(&directory).map_err(|_| "Não foi possível criar o cache local.")?;
    Ok(directory)
}

fn cache_file(directory: &Path, file_name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let extension = Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .ok_or("Formato de arquivo inválido.")?;
    let unique = format!(
        "giftray-{}-{}.{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| "Relógio do sistema inválido.")?
            .as_nanos(),
        extension
    );
    let path = directory.join(unique);
    fs::write(&path, bytes).map_err(|_| "Não foi possível gravar o arquivo temporário.")?;
    Ok(path)
}

fn prune_cache(directory: &Path) {
    let now = SystemTime::now();
    let mut files: Vec<_> = fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            entry
                .metadata()
                .ok()
                .map(|metadata| (entry.path(), metadata))
        })
        .filter(|(_, metadata)| metadata.is_file())
        .collect();
    for (path, metadata) in &files {
        if metadata
            .modified()
            .ok()
            .is_some_and(|modified| is_cache_expired(modified, now))
        {
            let _ = fs::remove_file(path);
        }
    }
    files.retain(|(path, _)| path.exists());
    files.sort_by_key(|(_, metadata)| metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH));
    let mut total: u64 = files.iter().map(|(_, metadata)| metadata.len()).sum();
    for (path, metadata) in files {
        if total <= CACHE_MAX_BYTES {
            break;
        }
        if fs::remove_file(path).is_ok() {
            total = total.saturating_sub(metadata.len());
        }
    }
}

fn is_cache_expired(modified: SystemTime, now: SystemTime) -> bool {
    now.duration_since(modified)
        .ok()
        .is_some_and(|age| age > CACHE_RETENTION)
}

#[cfg(windows)]
fn set_hdrop(path: &Path) -> Result<(), String> {
    use std::{
        mem::size_of,
        os::windows::ffi::OsStrExt,
        ptr::{copy_nonoverlapping, null_mut},
    };
    use windows_sys::Win32::{
        Foundation::GlobalFree,
        System::{
            DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData},
            Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
            Ole::CF_HDROP,
        },
        UI::Shell::DROPFILES,
    };
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.extend([0, 0]); // one terminated path plus end of the DROPFILES list
    let bytes = size_of::<DROPFILES>() + wide.len() * size_of::<u16>();
    // The allocation becomes clipboard-owned only after SetClipboardData succeeds.
    let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) };
    if memory.is_null() {
        return Err("Memória insuficiente para preparar a área de transferência.".into());
    }
    let target = unsafe { GlobalLock(memory) };
    if target.is_null() {
        unsafe {
            GlobalFree(memory);
        };
        return Err("Não foi possível bloquear a memória do clipboard.".into());
    }
    unsafe {
        let drop_files = target.cast::<DROPFILES>();
        *drop_files = std::mem::zeroed();
        (*drop_files).pFiles = size_of::<DROPFILES>() as u32;
        (*drop_files).fWide = 1;
        copy_nonoverlapping(
            wide.as_ptr().cast::<u8>(),
            target.cast::<u8>().add(size_of::<DROPFILES>()),
            wide.len() * size_of::<u16>(),
        );
        GlobalUnlock(memory);
    }
    // No logging: the path may identify a private local cache location.
    if unsafe { OpenClipboard(null_mut()) } == 0 {
        unsafe {
            GlobalFree(memory);
        };
        return Err("A área de transferência está ocupada por outro aplicativo.".into());
    }
    let result = unsafe {
        if EmptyClipboard() == 0 {
            false
        } else {
            !SetClipboardData(CF_HDROP as u32, memory).is_null()
        }
    };
    unsafe {
        CloseClipboard();
    }
    if !result {
        unsafe {
            GlobalFree(memory);
        };
        return Err("Não foi possível gravar o arquivo na área de transferência.".into());
    }
    Ok(())
}

#[cfg(not(windows))]
fn set_hdrop(_: &Path) -> Result<(), String> {
    Err("Copiar arquivo requer Windows.".into())
}

#[tauri::command]
pub fn copy_media_file(app: AppHandle, file_name: String, bytes: Vec<u8>) -> Result<(), String> {
    if bytes.is_empty() || bytes.len() > MAX_MEDIA_BYTES {
        return Err("O arquivo excede o limite experimental de 25 MB ou está vazio.".into());
    }
    if !valid_media(&file_name, &bytes) {
        return Err("O arquivo não possui um formato GIF, WebP ou PNG confirmado.".into());
    }
    let directory = cache_directory(&app)?;
    prune_cache(&directory);
    let path = cache_file(&directory, &file_name, &bytes)?;
    if let Err(error) = set_hdrop(&path) {
        let _ = fs::remove_file(path);
        return Err(error);
    }
    Ok(())
}

pub fn cleanup_clipboard_cache(app: &AppHandle) {
    if let Ok(directory) = cache_directory(app) {
        prune_cache(&directory);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_supported_extensions_with_matching_signatures() {
        assert!(valid_media("reaction.gif", b"GIF89a bytes"));
        assert!(valid_media("reaction.webp", b"RIFFxxxxWEBP bytes"));
        assert!(valid_media("reaction.png", b"\x89PNG\r\n\x1a\n bytes"));
        assert!(!valid_media("reaction.gif", b"not a gif"));
        assert!(!valid_media("reaction.exe", b"GIF89a bytes"));
    }

    #[test]
    fn expires_only_files_beyond_the_retention_window() {
        let now = SystemTime::now();
        assert!(!is_cache_expired(now - CACHE_RETENTION, now));
        assert!(is_cache_expired(
            now - CACHE_RETENTION - Duration::from_secs(1),
            now
        ));
    }
}
