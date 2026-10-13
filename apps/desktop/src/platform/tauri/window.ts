// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export function hideMainWindow(): Promise<void> {
  return invoke("hide_main_window");
}

export function updateGlobalShortcut(shortcut: string): Promise<void> {
  return invoke("update_global_shortcut", { shortcut });
}

export function listenForSearchWindowOpened(
  handler: () => void,
): Promise<UnlistenFn> {
  return listen("giftray://opened", handler);
}
