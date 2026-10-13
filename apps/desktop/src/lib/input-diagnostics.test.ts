// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { describe, expect, it } from "vitest";
import {
  describeHtmlInput,
  describeNativeFileDrop,
  describePasteInput,
  detectInputTypes,
} from "./input-diagnostics";

describe("detectInputTypes", () => {
  it("identifica os formatos sem ler ou registrar o conteúdo", () => {
    const image = new File(["gif"], "reaction.gif", { type: "image/gif" });
    expect(
      detectInputTypes(["text/uri-list", "text/html", "Files"], [image]),
    ).toEqual(["URL", "HTML", "arquivo", "imagem"]);
  });

  it("não infere uma imagem sem arquivo de imagem", () => {
    expect(detectInputTypes(["text/plain"])).toEqual([]);
  });
  it("registra MIME e quantidade para o mecanismo HTML5 sem conteúdo", () => {
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

  it("classifica o drop nativo sem expor caminhos", () => {
    expect(
      describeNativeFileDrop("drop", ["C:\\private\\reaction.gif"]),
    ).toMatchObject({
      mechanism: "Tauri nativo",
      fileCount: 1,
      mimeTypes: [],
      kinds: ["arquivo", "imagem"],
    });
  });

  it("diferencia paste sem formatos utilizáveis de um evento com dados", () => {
    expect(describePasteInput([])).toMatchObject({ status: "failed", mimeTypes: [], fileCount: 0 });
    expect(describePasteInput(["text/plain"])).toMatchObject({ status: "detected", mimeTypes: ["text/plain"] });
  });
});
