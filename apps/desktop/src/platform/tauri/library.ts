// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { invoke } from "@tauri-apps/api/core";

export type LibraryFormat = "gif" | "webp" | "png";
export type LibraryAvailability = "available" | "missing";
export type LibrarySourceKind = "local_file" | "remote_url" | "external_request";

/** Public projection only: no filesystem path, hash or media bytes. */
export interface LibraryItem {
  id: string;
  format: LibraryFormat;
  byteSize: number;
  sourceKind: LibrarySourceKind;
  originalName: string | null;
  importedAt: number;
  availability: LibraryAvailability;
}

export type ImportResult =
  | { status: "imported"; item: LibraryItem }
  | { status: "alreadyPresent"; item: LibraryItem }
  | { status: "cancelled" };

export function selectAndImportMedia(): Promise<ImportResult> {
  return invoke("select_and_import_media");
}

export function listLibraryItems(): Promise<LibraryItem[]> {
  return invoke("list_library_items");
}
