// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod shortcuts;
mod window;

use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub fn run() {
    let initial_shortcut = shortcuts::initial_shortcut();
    tauri::Builder::default()
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
            app.global_shortcut().register(initial_shortcut)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            window::hide_main_window,
            shortcuts::update_global_shortcut
        ])
        .run(tauri::generate_context!())
        .expect("error while running GIFTray");
}
