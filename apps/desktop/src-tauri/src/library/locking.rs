// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    fs::{File, OpenOptions},
    io,
    path::Path,
    sync::{Mutex, MutexGuard},
};

use super::model::LibraryError;

pub(super) struct OperationLock<'a> {
    _in_process: MutexGuard<'a, ()>,
}

impl<'a> OperationLock<'a> {
    pub(super) fn acquire(operation_mutex: &'a Mutex<()>) -> Result<Self, LibraryError> {
        let in_process = operation_mutex.lock().map_err(|_| LibraryError::InUse)?;
        Ok(Self {
            _in_process: in_process,
        })
    }
}

#[cfg(windows)]
pub(super) fn open_library_lock(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;

    // share_mode(0) maps to a CreateFile handle that no other process can
    // open. Windows closes it automatically after a crash or normal exit.
    OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .share_mode(0)
        .open(path)
}

#[cfg(not(windows))]
pub(super) fn open_library_lock(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)
}
