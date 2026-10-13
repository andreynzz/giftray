// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { describe, expect, it } from "vitest";
import {
  describeHtmlInput,
  describeNativeFileDrop,
  describePasteInput,
  detectInputTypes,
} from "./diagnostics";

describe("input diagnostics", () => {
  it("identifies formats without reading their contents", () => {
    const image = new File(["gif"], "reaction.gif", { type: "image/gif" });
    expect(
      detectInputTypes(["text/uri-list", "text/html", "Files"], [image]),
    ).toEqual(["URL", "HTML", "arquivo", "imagem"]);
  });

  it("records HTML5 metadata without content", () => {
    const file = new File(["gif"], "reaction.gif", { type: "image/gif" });
    expect(
      describeHtmlInput("HTML5", "drop", ["Files", "text/uri-list"], [file]),
    ).toMatchObject({
      mechanism: "HTML5",
      fileCount: 1,
      mimeTypes: ["files", "text/uri-list"],
      kinds: ["URL", "arquivo", "imagem"],
    });
  });

  it("classifies native files without exposing paths", () => {
    expect(
      describeNativeFileDrop("drop", ["C:\\private\\reaction.gif"]),
    ).toMatchObject({
      mechanism: "Tauri nativo",
      fileCount: 1,
      mimeTypes: [],
      kinds: ["arquivo", "imagem"],
    });
  });

  it("distinguishes an empty paste event", () => {
    expect(describePasteInput([])).toMatchObject({
      status: "failed",
      mimeTypes: [],
      fileCount: 0,
    });
    expect(describePasteInput(["text/plain"])).toMatchObject({
      status: "detected",
      mimeTypes: ["text/plain"],
    });
  });
});
