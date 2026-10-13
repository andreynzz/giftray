// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { getCurrentWebview } from "@tauri-apps/api/webview";

export type NativeDragDropPayload =
  | { type: "enter" | "drop"; paths: string[] }
  | { type: "over" | "leave" };

export async function listenForNativeDragDrop(
  handler: (payload: NativeDragDropPayload) => void,
): Promise<() => void> {
  return getCurrentWebview().onDragDropEvent(({ payload }) => handler(payload));
}
