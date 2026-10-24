// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    fs::{self, File},
    path::{Component, Path},
};

use super::{
    model::LibraryError, paths::is_reparse_point, policy::extended_dos_drive_root, MAX_MEDIA_BYTES,
};

#[cfg(windows)]
pub(super) fn open_external_media_file(source: &Path) -> Result<File, LibraryError> {
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
    use windows_sys::Win32::{
        Storage::FileSystem::{
            GetDriveTypeW, GetFinalPathNameByHandleW, FILE_FLAG_OPEN_REPARSE_POINT,
        },
        System::WindowsProgramming::DRIVE_FIXED,
    };

    if !source.is_absolute()
        || source.as_os_str().is_empty()
        || source.to_string_lossy().starts_with("\\\\")
        || source
            .components()
            .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(LibraryError::SourceUnavailable);
    }
    for component in source.ancestors().skip(1) {
        let metadata =
            fs::symlink_metadata(component).map_err(|_| LibraryError::SourceUnavailable)?;
        if is_reparse_point(&metadata) {
            return Err(LibraryError::SourceUnavailable);
        }
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(source)
        .map_err(|_| LibraryError::SourceUnavailable)?;
    let metadata = file
        .metadata()
        .map_err(|_| LibraryError::SourceUnavailable)?;
    if !metadata.file_type().is_file()
        || is_reparse_point(&metadata)
        || metadata.len() > MAX_MEDIA_BYTES
    {
        return Err(LibraryError::SourceUnavailable);
    }
    let handle = file.as_raw_handle().cast();
    let length = unsafe { GetFinalPathNameByHandleW(handle, std::ptr::null_mut(), 0, 0) };
    if length == 0 || length > 32767 {
        return Err(LibraryError::SourceUnavailable);
    }
    let mut wide = vec![0u16; length as usize + 1];
    let final_length =
        unsafe { GetFinalPathNameByHandleW(handle, wide.as_mut_ptr(), wide.len() as u32, 0) };
    if final_length == 0 || final_length as usize >= wide.len() {
        return Err(LibraryError::SourceUnavailable);
    }
    let final_path = String::from_utf16(&wide[..final_length as usize])
        .map_err(|_| LibraryError::SourceUnavailable)?;
    let root = extended_dos_drive_root(&final_path)?;
    let mut root_wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
    if unsafe { GetDriveTypeW(root_wide.as_mut_ptr()) } != DRIVE_FIXED {
        return Err(LibraryError::SourceUnavailable);
    }
    Ok(file)
}

#[cfg(not(windows))]
pub(super) fn open_external_media_file(_: &Path) -> Result<File, LibraryError> {
    Err(LibraryError::SourceUnavailable)
}
