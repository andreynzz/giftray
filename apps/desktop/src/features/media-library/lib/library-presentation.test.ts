// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { describe, expect, it } from "vitest";
import type { LibraryItem } from "../../../platform/tauri/library";
import { canCopyLibraryItem, canPreviewLibraryItem, canStartRemoteImport, filterLibraryItems, formatByteSize, libraryItemLabel, sortLibraryItems } from "./library-presentation";

const item = (importedAt: number, originalName: string | null): LibraryItem => ({
  id: `${importedAt}`, format: "gif", byteSize: 1024, sourceKind: "local_file",
  originalName, importedAt, availability: "available",
  displayName: null,
});

describe("library presentation", () => {
  it("formats file sizes for the visible metadata", () => {
    expect(formatByteSize(999)).toBe("999 B");
    expect(formatByteSize(1536)).toBe("1,5 KB");
    expect(formatByteSize(2 * 1024 * 1024)).toBe("2 MB");
  });

  it("uses a fallback label and shows newest items first", () => {
    expect(libraryItemLabel(item(1, null))).toBe("Arquivo GIF");
    expect(sortLibraryItems([item(1, "old.gif"), item(2, "new.gif")])).toEqual([
      item(2, "new.gif"), item(1, "old.gif"),
    ]);
  });

  it("only enables copying for available items", () => {
    expect(canCopyLibraryItem(item(1, "available.gif"), null)).toBe(true);
    expect(canCopyLibraryItem({ ...item(1, "missing.gif"), availability: "missing" }, null)).toBe(false);
  });

  it("disables every copy action while one item is being copied", () => {
    expect(canCopyLibraryItem(item(1, "first.gif"), "1")).toBe(false);
    expect(canCopyLibraryItem(item(2, "second.gif"), "1")).toBe(false);
  });

  it("does not preview unavailable items or animate with reduced motion", () => {
    expect(canPreviewLibraryItem(item(1, "available.gif"), false)).toBe(true);
    expect(canPreviewLibraryItem(item(1, "available.gif"), true)).toBe(false);
    expect(canPreviewLibraryItem({ ...item(1, "missing.gif"), availability: "missing" }, false)).toBe(false);
  });

  it("requires a URL and no concurrent import before enabling remote import", () => {
    expect(canStartRemoteImport("", false)).toBe(false);
    expect(canStartRemoteImport("  ", false)).toBe(false);
    expect(canStartRemoteImport("https://cdn.example.test/reaction.gif", true)).toBe(false);
    expect(canStartRemoteImport("https://cdn.example.test/reaction.gif", false)).toBe(true);
  });

  it("filters names, formats and accents without changing an empty query", () => {
    const items = [
      { ...item(1, "café.gif"), displayName: "Reação rápida" },
      item(2, "other.webp"),
    ];
    expect(filterLibraryItems(items, "")).toEqual(items);
    expect(filterLibraryItems(items, "REACAO")).toEqual([items[0]]);
    expect(filterLibraryItems(items, "CAFE")).toEqual([items[0]]);
    expect(filterLibraryItems(items, "webp")).toEqual([items[1]]);
    expect(filterLibraryItems(items, "ausente")).toEqual([]);
    expect(libraryItemLabel(items[0])).toBe("Reação rápida");
  });
});
