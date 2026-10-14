// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Windows file clipboard support. CF_HDROP owns only the path list: retained
//! cache files must outlive the clipboard operation and a receiving process.

use std::{
    collections::HashSet,
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

pub fn media_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
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

fn prune_cache_with_limit(directory: &Path, protected: &HashSet<PathBuf>, limit: u64) -> bool {
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
        if !protected.contains(path)
            && metadata
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
        if total <= limit {
            break;
        }
        if protected.contains(&path) {
            continue;
        }
        if fs::remove_file(path).is_ok() {
            total = total.saturating_sub(metadata.len());
        }
    }
    total <= limit
}

fn prune_cache(directory: &Path, protected: &HashSet<PathBuf>) -> bool {
    prune_cache_with_limit(directory, protected, CACHE_MAX_BYTES)
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

#[cfg(windows)]
fn protected_clipboard_paths(directory: &Path) -> Result<HashSet<PathBuf>, String> {
    use windows_sys::Win32::{
        System::{
            DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard},
            Ole::CF_HDROP,
        },
        UI::Shell::DragQueryFileW,
    };
    let mut paths = HashSet::new();
    if unsafe { OpenClipboard(std::ptr::null_mut()) } == 0 {
        return Err("Não foi possível verificar os arquivos protegidos pelo clipboard.".into());
    }
    let handle = unsafe { GetClipboardData(CF_HDROP as u32) };
    if !handle.is_null() {
        let count = unsafe { DragQueryFileW(handle, u32::MAX, std::ptr::null_mut(), 0) };
        for index in 0..count {
            let length = unsafe { DragQueryFileW(handle, index, std::ptr::null_mut(), 0) };
            let mut buffer = vec![0u16; length as usize + 1];
            unsafe {
                DragQueryFileW(handle, index, buffer.as_mut_ptr(), buffer.len() as u32);
            }
            if let Ok(path) =
                PathBuf::from(String::from_utf16_lossy(&buffer[..length as usize])).canonicalize()
            {
                if path.starts_with(directory) {
                    paths.insert(path);
                }
            }
        }
    }
    unsafe {
        CloseClipboard();
    }
    Ok(paths)
}

#[cfg(not(windows))]
fn protected_clipboard_paths(_: &Path) -> Result<HashSet<PathBuf>, String> {
    Ok(HashSet::new())
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
    let protected = protected_clipboard_paths(&directory)?;
    if !prune_cache(&directory, &protected) {
        return Err("Não há espaço seguro disponível no cache temporário.".into());
    }
    let path = cache_file(&directory, &file_name, &bytes)?;
    if let Err(error) = set_hdrop(&path) {
        let _ = fs::remove_file(path);
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
pub async fn copy_remote_media_file(app: AppHandle, url: String) -> Result<(), String> {
    let bytes = crate::remote_media::download(&url).await?;
    let extension = media_extension(&bytes)
        .ok_or("A resposta remota não é uma mídia GIF, WebP ou PNG válida.")?;
    copy_media_file(app, format!("remote-media.{extension}"), bytes)
}

pub fn cleanup_clipboard_cache(app: &AppHandle) {
    if let Ok(directory) = cache_directory(app) {
        if let Ok(protected) = protected_clipboard_paths(&directory) {
            let _ = prune_cache(&directory, &protected);
        }
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

    #[test]
    fn cache_limit_preserves_protected_file_and_rejects_when_only_it_remains() {
        let directory =
            std::env::temp_dir().join(format!("giftray-cache-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let protected_path = directory.join("active.gif");
        let disposable_path = directory.join("old.gif");
        fs::write(&protected_path, b"1234").unwrap();
        fs::write(&disposable_path, b"5678").unwrap();
        let protected = HashSet::from([protected_path.clone()]);
        assert!(prune_cache_with_limit(&directory, &protected, 4));
        assert!(protected_path.exists());
        assert!(!disposable_path.exists());
        assert!(!prune_cache_with_limit(&directory, &protected, 3));
        assert!(protected_path.exists());
        let _ = fs::remove_dir_all(directory);
    }
}
