// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

export function copyText(text: string): Promise<void> {
  return writeText(text);
}

export function copyFileToWindowsClipboard(fileName: string, bytes: Uint8Array): Promise<void> {
  return invoke("copy_media_file", { fileName, bytes: Array.from(bytes) });
}
