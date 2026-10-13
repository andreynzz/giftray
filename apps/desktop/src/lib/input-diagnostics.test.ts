// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { describe, expect, it } from "vitest";
import { detectInputTypes } from "./input-diagnostics";

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
});
