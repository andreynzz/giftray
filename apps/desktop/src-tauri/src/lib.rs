// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod clipboard;
mod library;
mod remote_media;
mod shortcuts;
mod window;

use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub fn run() {
    let initial_shortcut = shortcuts::initial_shortcut();
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        window::toggle_search_window(app);
                    }
                })
                .build(),
        )
        .manage(shortcuts::RegisteredShortcut(std::sync::Mutex::new(
            initial_shortcut,
        )))
        .setup(move |app| {
            clipboard::cleanup_clipboard_cache(&app.handle());
            // Persistent-library recovery is native-only and separate from the
            // disposable clipboard cache. A second process simply skips it.
            match app.handle().path().app_local_data_dir() {
                Ok(directory) => match library::Library::initialize(&directory) {
                    Ok(_) | Err(library::LibraryError::InUse) => {}
                    Err(_) => {
                        return Err(Box::<dyn std::error::Error>::from(
                            "library initialization failed",
                        ))
                    }
                },
                Err(_) => {
                    return Err(Box::<dyn std::error::Error>::from(
                        "library data directory unavailable",
                    ))
                }
            }
            app.global_shortcut().register(initial_shortcut)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            window::hide_main_window,
            shortcuts::update_global_shortcut,
            clipboard::copy_media_file,
            clipboard::copy_remote_media_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running GIFTray");
}
