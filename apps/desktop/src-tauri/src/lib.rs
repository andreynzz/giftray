// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

struct RegisteredShortcut(Mutex<Shortcut>);

fn toggle_search_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit("giftray://opened", ());
    }
}

#[tauri::command]
fn hide_main_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[tauri::command]
fn update_global_shortcut(
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

    // Tenta registrar antes de substituir: se o Windows rejeitar o atalho,
    // a combinação em uso permanece ativa.
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

pub fn run() {
    let initial_shortcut: Shortcut = "Ctrl+Alt+G"
        .parse()
        .expect("default shortcut must be valid");
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_search_window(app);
                    }
                })
                .build(),
        )
        .manage(RegisteredShortcut(Mutex::new(initial_shortcut)))
        .setup(move |app| {
            app.global_shortcut().register(initial_shortcut)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            hide_main_window,
            update_global_shortcut
        ])
        .run(tauri::generate_context!())
        .expect("error while running GIFTray");
}
