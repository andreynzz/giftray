// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Windows file clipboard support. CF_HDROP owns only the path list: retained
//! cache files must outlive the clipboard operation and a receiving process.

use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, SystemTime},
};
use tauri::{AppHandle, Manager};

use crate::library::LibraryCopySource;

const MAX_MEDIA_BYTES: usize = 25 * 1024 * 1024;
const CACHE_MAX_BYTES: u64 = 100 * 1024 * 1024;
const CACHE_RETENTION: Duration = Duration::from_secs(24 * 60 * 60);
const CACHE_DIRECTORY: &str = "clipboard-output";
static CACHE_LOCK: Mutex<()> = Mutex::new(());

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
    directory
        .canonicalize()
        .map_err(|_| "Não foi possível acessar o cache local.".into())
}

fn cache_file(directory: &Path, file_name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    cache_file_with(directory, file_name, bytes, |path, contents| {
        fs::write(path, contents)
    })
}

fn cache_file_with<F>(
    directory: &Path,
    file_name: &str,
    bytes: &[u8],
    write: F,
) -> Result<PathBuf, String>
where
    F: FnOnce(&Path, &[u8]) -> std::io::Result<()>,
{
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
    let temporary = directory.join(format!(
        ".{path_name}.part",
        path_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("Formato de arquivo inválido.")?
    ));
    if write(&temporary, bytes).is_err() {
        let _ = fs::remove_file(&temporary);
        return Err("Não foi possível gravar o arquivo temporário.".into());
    }
    if fs::rename(&temporary, &path).is_err() {
        let _ = fs::remove_file(&temporary);
        return Err("Não foi possível finalizar o arquivo temporário.".into());
    }
    Ok(path)
}

fn cache_file_from_reader<R: Read>(
    directory: &Path,
    file_name: &str,
    expected_size: u64,
    expected_sha256: &str,
    mut source: R,
) -> Result<PathBuf, String> {
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
    let temporary = directory.join(format!(
        ".{path_name}.part",
        path_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("Formato de arquivo inválido.")?
    ));
    let result = (|| -> Result<(), String> {
        let mut output = File::create(&temporary)
            .map_err(|_| "Não foi possível gravar o arquivo temporário.")?;
        let mut buffer = [0u8; 8192];
        let mut prefix = Vec::with_capacity(12);
        let mut total = 0usize;
        let mut digest = Sha256::new();
        loop {
            let count = source
                .read(&mut buffer)
                .map_err(|_| "Não foi possível ler o arquivo da biblioteca.")?;
            if count == 0 {
                break;
            }
            total = total
                .checked_add(count)
                .ok_or("O arquivo excede o limite de 25 MB.")?;
            if total > MAX_MEDIA_BYTES {
                return Err("O arquivo excede o limite de 25 MB.".into());
            }
            if total as u64 > expected_size {
                return Err("O arquivo da biblioteca foi alterado durante a cópia.".into());
            }
            if prefix.len() < 12 {
                prefix.extend_from_slice(&buffer[..count.min(12 - prefix.len())]);
            }
            output
                .write_all(&buffer[..count])
                .map_err(|_| "Não foi possível gravar o arquivo temporário.")?;
            digest.update(&buffer[..count]);
        }
        if total == 0
            || total as u64 != expected_size
            || !valid_media(file_name, &prefix)
            || format!("{:x}", digest.finalize()) != expected_sha256
        {
            return Err("O arquivo não possui um formato GIF, WebP ou PNG confirmado.".into());
        }
        output
            .sync_all()
            .map_err(|_| "Não foi possível gravar o arquivo temporário.")?;
        fs::rename(&temporary, &path)
            .map_err(|_| "Não foi possível finalizar o arquivo temporário.")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map(|()| path)
}

fn is_incomplete_cache_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(".giftray-") && name.ends_with(".part"))
}

fn is_protected_cache_path(path: &Path, protected: &HashSet<PathBuf>) -> bool {
    protected.contains(path)
        || path
            .canonicalize()
            .ok()
            .is_some_and(|canonical| protected.contains(&canonical))
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
        if is_incomplete_cache_file(path)
            || (!is_protected_cache_path(path, protected)
                && metadata
                    .modified()
                    .ok()
                    .is_some_and(|modified| is_cache_expired(modified, now)))
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
        if is_protected_cache_path(&path, protected) {
            continue;
        }
        if fs::remove_file(path).is_ok() {
            total = total.saturating_sub(metadata.len());
        }
    }
    total <= limit
}

fn reserve_cache_space(directory: &Path, protected: &HashSet<PathBuf>, incoming: usize) -> bool {
    reserve_cache_space_with_limit(directory, protected, incoming as u64, CACHE_MAX_BYTES)
}

fn reserve_cache_space_with_limit(
    directory: &Path,
    protected: &HashSet<PathBuf>,
    incoming: u64,
    limit: u64,
) -> bool {
    limit
        .checked_sub(incoming)
        .is_some_and(|available| prune_cache_with_limit(directory, protected, available))
}

fn publish_cached_file<F>(path: &Path, publish: F) -> Result<(), String>
where
    F: FnOnce(&Path) -> Result<(), String>,
{
    if let Err(error) = publish(path) {
        let _ = fs::remove_file(path);
        return Err(error);
    }
    Ok(())
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
    let _lock = CACHE_LOCK
        .lock()
        .map_err(|_| "Não foi possível acessar o cache temporário com segurança.")?;
    let protected = protected_clipboard_paths(&directory)?;
    if !reserve_cache_space(&directory, &protected, bytes.len()) {
        return Err("Não há espaço seguro disponível no cache temporário.".into());
    }
    let path = cache_file(&directory, &file_name, &bytes)?;
    publish_cached_file(&path, set_hdrop)
}

pub(crate) fn copy_library_file(app: AppHandle, source: LibraryCopySource) -> Result<(), String> {
    let metadata =
        fs::metadata(&source.path).map_err(|_| "O arquivo da biblioteca não está disponível.")?;
    if !metadata.is_file() || metadata.len() != source.byte_size || metadata.len() == 0 {
        return Err("O arquivo da biblioteca não está disponível.".into());
    }
    let directory = cache_directory(&app)?;
    let _lock = CACHE_LOCK
        .lock()
        .map_err(|_| "Não foi possível acessar o cache temporário com segurança.")?;
    let protected = protected_clipboard_paths(&directory)?;
    if !reserve_cache_space(&directory, &protected, source.byte_size as usize) {
        return Err("Não há espaço seguro disponível no cache temporário.".into());
    }
    let file =
        File::open(&source.path).map_err(|_| "O arquivo da biblioteca não está disponível.")?;
    let path = cache_file_from_reader(
        &directory,
        &source.file_name,
        source.byte_size,
        &source.content_sha256,
        file,
    )?;
    publish_cached_file(&path, set_hdrop)
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
        let Ok(_lock) = CACHE_LOCK.lock() else {
            return;
        };
        cleanup_cache_directory(&directory, protected_clipboard_paths(&directory));
    }
}

fn cleanup_cache_directory(directory: &Path, protected: Result<HashSet<PathBuf>, String>) {
    cleanup_cache_directory_with_limit(directory, protected, CACHE_MAX_BYTES);
}

fn cleanup_cache_directory_with_limit(
    directory: &Path,
    protected: Result<HashSet<PathBuf>, String>,
    limit: u64,
) {
    if let Ok(protected) = protected {
        let _ = prune_cache_with_limit(directory, &protected, limit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn test_cache_directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "giftray-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn digest(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

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
        let directory = test_cache_directory("cache-limit");
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

    #[test]
    fn cache_cleanup_preserves_a_reconstructed_clipboard_path_after_restart() {
        let directory = test_cache_directory("cache-restart");
        let active = directory.join("active.gif");
        let disposable = directory.join("old.gif");
        fs::write(&active, b"1234").unwrap();
        fs::write(&disposable, b"5678").unwrap();
        let restored_protection = HashSet::from([active.canonicalize().unwrap()]);

        assert!(prune_cache_with_limit(&directory, &restored_protection, 4));
        assert!(active.exists());
        assert!(!disposable.exists());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn cache_reservation_keeps_protected_files_and_requires_room_for_the_new_file() {
        let directory = test_cache_directory("cache-reservation");
        let protected_path = directory.join("active.gif");
        let disposable_path = directory.join("old.gif");
        fs::write(&protected_path, b"1234").unwrap();
        fs::write(&disposable_path, b"5678").unwrap();
        let protected = HashSet::from([protected_path.clone()]);

        assert!(reserve_cache_space_with_limit(&directory, &protected, 4, 8));
        assert!(protected_path.exists());
        assert!(!disposable_path.exists());
        assert!(!reserve_cache_space_with_limit(
            &directory, &protected, 1, 4
        ));
        assert!(protected_path.exists());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn cache_write_failure_removes_incomplete_file_and_preserves_existing_files() {
        let directory = test_cache_directory("cache-write-failure");
        let existing = directory.join("previous.gif");
        fs::write(&existing, b"GIF89a previous").unwrap();

        let result = cache_file_with(&directory, "new.gif", b"GIF89a new", |path, _| {
            fs::write(path, b"partial")?;
            Err(std::io::Error::other("simulated write failure"))
        });

        assert!(result.is_err());
        assert!(existing.exists());
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn streams_supported_library_media_to_cache_preserving_bytes() {
        let directory = test_cache_directory("library-stream");
        for (name, bytes) in [
            ("a.gif", b"GIF89a gif".as_slice()),
            ("b.webp", b"RIFFxxxxWEBP webp".as_slice()),
            ("c.png", b"\x89PNG\r\n\x1a\n png".as_slice()),
        ] {
            let path = cache_file_from_reader(
                &directory,
                name,
                bytes.len() as u64,
                &digest(bytes),
                Cursor::new(bytes),
            )
            .unwrap();
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn rejects_changed_or_oversized_library_streams_without_publishing_cache_files() {
        let directory = test_cache_directory("library-stream-rejection");
        assert!(cache_file_from_reader(
            &directory,
            "a.gif",
            3,
            &digest(b"GIF89a data"),
            Cursor::new(b"GIF89a data"),
        )
        .is_err());
        assert!(cache_file_from_reader(
            &directory,
            "changed.gif",
            b"GIF89a same".len() as u64,
            &digest(b"GIF89a same"),
            Cursor::new(b"GIF89a swap"),
        )
        .is_err());
        assert!(cache_file_from_reader(
            &directory,
            "large.gif",
            (MAX_MEDIA_BYTES + 1) as u64,
            &digest(&vec![b'x'; MAX_MEDIA_BYTES + 1]),
            Cursor::new(vec![b'x'; MAX_MEDIA_BYTES + 1]),
        )
        .is_err());
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 0);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn library_stream_read_failure_removes_the_incomplete_cache_file() {
        struct FailingReader(bool);
        impl Read for FailingReader {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                if self.0 {
                    return Err(std::io::Error::other("simulated read failure"));
                }
                self.0 = true;
                buffer[..6].copy_from_slice(b"GIF89a");
                Ok(6)
            }
        }

        let directory = test_cache_directory("library-stream-failure");
        assert!(cache_file_from_reader(
            &directory,
            "a.gif",
            7,
            &"a".repeat(64),
            FailingReader(false),
        )
        .is_err());
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 0);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn cleanup_removes_incomplete_cache_files() {
        let directory = test_cache_directory("cache-incomplete");
        let incomplete = directory.join(".giftray-pending.gif.part");
        let complete = directory.join("giftray-complete.gif");
        fs::write(&incomplete, b"partial").unwrap();
        fs::write(&complete, b"GIF89a complete").unwrap();

        assert!(prune_cache_with_limit(&directory, &HashSet::new(), 100));
        assert!(!incomplete.exists());
        assert!(complete.exists());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn clipboard_publication_failure_removes_only_the_new_cache_file() {
        let directory = test_cache_directory("clipboard-publication-failure");
        let existing = directory.join("previous.gif");
        fs::write(&existing, b"GIF89a previous").unwrap();
        let new_file = cache_file(&directory, "new.gif", b"GIF89a new").unwrap();

        assert!(
            publish_cached_file(&new_file, |_| Err("simulated clipboard failure".into())).is_err()
        );
        assert!(!new_file.exists());
        assert!(existing.exists());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn clipboard_query_error_does_not_trigger_cache_cleanup() {
        let directory = test_cache_directory("clipboard-query-error");
        let existing = directory.join("existing.gif");
        fs::write(&existing, b"GIF89a existing").unwrap();

        cleanup_cache_directory_with_limit(
            &directory,
            Err("simulated clipboard query failure".into()),
            0,
        );

        assert!(existing.exists());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn cache_lock_excludes_concurrent_cleanup_and_publication() {
        let lock = CACHE_LOCK.lock().unwrap();
        assert!(CACHE_LOCK.try_lock().is_err());
        drop(lock);
        assert!(CACHE_LOCK.try_lock().is_ok());
    }
}
