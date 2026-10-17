// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { describe, expect, it } from "vitest";
import type { LibraryItem } from "../../../platform/tauri/library";
import { formatByteSize, libraryItemLabel, sortLibraryItems } from "./library-presentation";

const item = (importedAt: number, originalName: string | null): LibraryItem => ({
  id: `${importedAt}`, format: "gif", byteSize: 1024, sourceKind: "local_file",
  originalName, importedAt, availability: "available",
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
});
