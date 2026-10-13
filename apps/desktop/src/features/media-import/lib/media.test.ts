// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { describe, expect, it } from "vitest";
import {
  detectAnimation,
  detectMediaFormat,
  extractHtmlUrls,
  inspectUrlCandidate,
  normalizeReceivedInput,
  selectMediaCandidate,
  type InspectedMedia,
} from "./media";

const remoteGif: InspectedMedia = {
  candidate: {
    kind: "url",
    origin: "text/uri-list",
    url: "https://example.test/reaction.gif",
  },
  format: "gif",
  formatConfidence: "inferred",
  animation: "inferred",
};

const staticPng: InspectedMedia = {
  candidate: {
    kind: "file",
    origin: "file",
    file: new File(["png"], "thumb.png"),
  },
  format: "png",
  formatConfidence: "confirmed",
  animation: "unknown",
  size: 3,
};

describe("media input normalization", () => {
  it("accepts HTTPS URLs from URI lists, plain text and HTML without preserving content", () => {
    const normalized = normalizeReceivedInput({
      origin: "drop",
      files: [],
      uriList: "# copied link\nhttps://example.test/reaction.gif",
      plainText: "https://example.test/reaction.gif",
      html: '<a href="https://example.test/preview.webp">reaction</a>',
    });

    expect(normalized.candidates).toEqual([
      {
        kind: "url",
        origin: "text/uri-list",
        url: "https://example.test/reaction.gif",
      },
      { kind: "url", origin: "html", url: "https://example.test/preview.webp" },
    ]);
  });

  it("rejects malformed and insecure URL values", () => {
    const normalized = normalizeReceivedInput({
      origin: "paste",
      files: [],
      plainText: "http://example.test/reaction.gif",
      html: '<img src="javascript:alert(1)"><a href="not a url">x</a>',
    });

    expect(normalized.candidates).toEqual([]);
    expect(normalized.rejectedCount).toBe(3);
  });

  it("extracts only URL-like HTML attributes and never renders HTML", () => {
    expect(
      extractHtmlUrls(
        '<img src="https://example.test/a.gif"><a href="https://example.test/b.png">',
      ),
    ).toEqual(["https://example.test/a.gif", "https://example.test/b.png"]);
  });
});

describe("media format inspection", () => {
  it("recognizes binary signatures instead of file extensions", () => {
    expect(detectMediaFormat(new Uint8Array([71, 73, 70, 56, 57, 97, 0, 0, 0, 0, 0, 0, 0]))).toBe("gif");
    expect(
      detectMediaFormat(
        new Uint8Array([82, 73, 70, 70, 0, 0, 0, 0, 87, 69, 66, 80, 86, 80, 56, 32]),
      ),
    ).toBe("webp");
    expect(
      detectMediaFormat(new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1])),
    ).toBe("png");
    expect(detectMediaFormat(new Uint8Array([0, 1, 2]))).toBe("unknown");
  });

  it("confirms animation only from animation markers", () => {
    const animatedWebp = new Uint8Array([
      82, 73, 70, 70, 0, 0, 0, 0, 87, 69, 66, 80, 86, 80, 56, 88, 10, 0, 0, 0,
      2,
    ]);
    const likelyAnimatedGif = new Uint8Array([
      71, 73, 70, 56, 57, 97, 0, 0, 0, 0, 0, 0, 0, 0x2c, 0x2c,
    ]);

    expect(detectAnimation(animatedWebp, "webp")).toBe("confirmed");
    expect(detectAnimation(likelyAnimatedGif, "gif")).toBe("inferred");
    expect(
      detectAnimation(new Uint8Array([71, 73, 70, 56, 57, 97, 0, 0, 0, 0, 0, 0, 0]), "gif"),
    ).toBe("unknown");
  });

  it("treats URL extensions as an inference, not a file signature", () => {
    expect(
      inspectUrlCandidate({
        kind: "url",
        origin: "text/plain",
        url: "https://example.test/media.webp",
      }),
    ).toMatchObject({
      format: "webp",
      formatConfidence: "inferred",
      animation: "inferred",
    });
    expect(
      inspectUrlCandidate({
        kind: "url",
        origin: "text/plain",
        url: "https://example.test/download",
      }),
    ).toEqual({
      error: "A URL não indica um formato GIF, WebP ou PNG suportado.",
    });
  });
});

describe("media source selection", () => {
  it("prefers a probable animated URL over a verified static thumbnail", () => {
    expect(selectMediaCandidate([staticPng, remoteGif])).toBe(remoteGif);
  });

  it("prefers confirmed animation over inferred URL animation", () => {
    const animatedFile: InspectedMedia = {
      ...staticPng,
      animation: "confirmed",
    };
    expect(selectMediaCandidate([remoteGif, animatedFile])).toBe(animatedFile);
  });
});
