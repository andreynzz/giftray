// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use tauri::{ AppHandle, Emitter, Manager };

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToggleAction {
    Hide,
    Show,
}

fn toggle_action(is_visible: bool, is_minimized: bool) -> ToggleAction {
    if is_visible && !is_minimized { ToggleAction::Hide } else { ToggleAction::Show }
}

pub fn toggle_search_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    match
        toggle_action(window.is_visible().unwrap_or(false), window.is_minimized().unwrap_or(false))
    {
        ToggleAction::Hide => {
            let _ = window.hide();
        }
        ToggleAction::Show => show_main_window(app),
    }
}

pub fn show_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if window.show().is_ok() && window.unminimize().is_ok() && window.set_focus().is_ok() {
        let _ = app.emit("giftray://opened", ());
    }
}

#[tauri::command]
pub fn hide_main_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[cfg(test)]
mod tests {
    use super::{ toggle_action, ToggleAction };

    #[test]
    fn toggle_hides_a_visible_normal_window() {
        assert_eq!(toggle_action(true, false), ToggleAction::Hide);
    }

    #[test]
    fn toggle_shows_a_hidden_window() {
        assert_eq!(toggle_action(false, false), ToggleAction::Show);
    }

    #[test]
    fn toggle_restores_a_visible_minimized_window() {
        assert_eq!(toggle_action(true, true), ToggleAction::Show);
    }
}
