// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { describe, expect, it } from "vitest";
import type { MediaPreviewState } from "../../media-import/hooks/useMediaImport";
import { canCopyFile, canCopyLink, selectedCopyMode } from "./copy-mode";

const remotePreview: MediaPreviewState = {
  status: "ready",
  src: "https://example.test/reaction.gif?signature=unchanged",
  media: { animation: "inferred", format: "gif", formatConfidence: "inferred", candidate: { kind: "url", origin: "text/plain", url: "https://example.test/reaction.gif?signature=unchanged" } },
};

const localPreview: MediaPreviewState = {
  status: "ready",
  src: "blob:test",
  media: { animation: "confirmed", format: "webp", formatConfidence: "confirmed", candidate: { kind: "file", origin: "file", file: new File(["bytes"], "reaction.webp") } },
};

describe("clipboard copy mode", () => {
  it("allows only the original HTTPS remote URL to be copied as a link", () => {
    expect(canCopyLink(remotePreview)).toBe(true);
    expect(selectedCopyMode(remotePreview)).toBe("link");
  });

  it("makes file copy available only for an imported local file", () => {
    expect(canCopyFile(localPreview)).toBe(true);
    expect(canCopyLink(localPreview)).toBe(false);
    expect(selectedCopyMode(localPreview)).toBe("file");
  });

  it("does not enable output without a selected preview", () => {
    expect(selectedCopyMode({ status: "idle" })).toBeUndefined();
  });
});
