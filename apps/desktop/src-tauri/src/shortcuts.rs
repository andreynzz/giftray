// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::sync::Mutex;
use tauri::{AppHandle, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub struct RegisteredShortcut(pub Mutex<Shortcut>);

pub fn initial_shortcut() -> Shortcut {
    "Ctrl+Alt+G"
        .parse()
        .expect("default shortcut must be valid")
}

#[tauri::command]
pub fn update_global_shortcut(
    app: AppHandle,
    state: State<'_, RegisteredShortcut>,
    shortcut: String,
) -> Result<(), String> {
    let requested: Shortcut = shortcut
        .parse()
        .map_err(|_| "Formato de atalho inválido".to_owned())?;
    let mut current = state
        .0
        .lock()
        .map_err(|_| "Estado do atalho indisponível".to_owned())?;
    if *current == requested {
        return Ok(());
    }

    app.global_shortcut()
        .register(requested)
        .map_err(|_| "Atalho indisponível".to_owned())?;
    app.global_shortcut()
        .unregister(*current)
        .map_err(|error| {
            let _ = app.global_shortcut().unregister(requested);
            error.to_string()
        })?;
    *current = requested;
    Ok(())
}
