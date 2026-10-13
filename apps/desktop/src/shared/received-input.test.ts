// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { describe, expect, it, vi } from "vitest";
import { readReceivedInput } from "./received-input";

describe("received input", () => {
  it("extracts text/plain during an explicit paste without retaining textarea content", () => {
    const getData = vi.fn((type: string) =>
      type === "text/plain" ? "https://example.test/reaction.gif" : "",
    );
    const transfer = { files: [], getData } as unknown as DataTransfer;

    expect(readReceivedInput("paste", transfer)).toMatchObject({
      origin: "paste",
      plainText: "https://example.test/reaction.gif",
      files: [],
    });
    expect(getData).toHaveBeenCalledWith("text/plain");
  });
});
