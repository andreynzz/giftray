// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import type { LibraryItem } from "../../../platform/tauri/library";

const byteFormatter = new Intl.NumberFormat("pt-BR", { maximumFractionDigits: 1 });

export function formatByteSize(byteSize: number): string {
  if (byteSize < 1024) return `${byteSize} B`;
  if (byteSize < 1024 * 1024) return `${byteFormatter.format(byteSize / 1024)} KB`;
  return `${byteFormatter.format(byteSize / (1024 * 1024))} MB`;
}

export function libraryItemLabel(item: LibraryItem): string {
  return item.displayName?.trim() || item.originalName?.trim() || `Arquivo ${item.format.toUpperCase()}`;
}

function searchText(value: string): string {
  return value.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLocaleLowerCase("pt-BR");
}

export function filterLibraryItems(items: LibraryItem[], query: string): LibraryItem[] {
  const normalizedQuery = searchText(query.trim());
  if (!normalizedQuery) return items;
  return items.filter((item) => [libraryItemLabel(item), item.originalName ?? "", item.format]
    .some((value) => searchText(value).includes(normalizedQuery)));
}

export function canCopyLibraryItem(item: LibraryItem, copyingId: string | null): boolean {
  return item.availability === "available" && copyingId === null;
}

export function canPreviewLibraryItem(item: LibraryItem, reducedMotion: boolean): boolean {
  return item.availability === "available" && !reducedMotion;
}

export function canStartRemoteImport(url: string, importing: boolean): boolean {
  return !importing && url.trim().length > 0;
}

export function sortLibraryItems(items: LibraryItem[]): LibraryItem[] {
  return [...items].sort((left, right) => right.importedAt - left.importedAt);
}
