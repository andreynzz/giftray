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

/** Sends the untrusted URL only to the native P0-protected downloader. */
export function importRemoteMedia(url: string): Promise<Exclude<ImportResult, { status: "cancelled" }>> {
  return invoke("import_remote_media", { url });
}

export function listLibraryItems(): Promise<LibraryItem[]> {
  return invoke("list_library_items");
}

/** Sends only the opaque persistent-item UUID across IPC. */
export function copyLibraryItem(id: string): Promise<void> {
  return invoke("copy_library_item", { id });
}

const libraryMediaOrigin = "http://library-media.localhost";

/** Restricted Tauri protocol URLs carry only an opaque item UUID. */
export function libraryThumbnailUrl(id: string): string {
  return `${libraryMediaOrigin}/thumbnail/${encodeURIComponent(id)}`;
}

export function libraryPreviewUrl(id: string): string {
  return `${libraryMediaOrigin}/preview/${encodeURIComponent(id)}`;
}
