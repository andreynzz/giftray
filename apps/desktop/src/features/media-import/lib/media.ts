// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import type { ReceivedInput } from "../../../shared/received-input";

export const MAX_MEDIA_BYTES = 25 * 1024 * 1024;

export type AnimationStatus = "confirmed" | "inferred" | "unknown";
export type MediaFormat = "gif" | "png" | "webp" | "unknown";
export type MediaOrigin = "file" | "html" | "text/plain" | "text/uri-list";

export interface FileMediaCandidate {
  file: File;
  kind: "file";
  origin: "file";
}

export interface UrlMediaCandidate {
  kind: "url";
  origin: Exclude<MediaOrigin, "file">;
  url: string;
}

export type MediaCandidate = FileMediaCandidate | UrlMediaCandidate;

export interface InspectedMedia {
  animation: AnimationStatus;
  candidate: MediaCandidate;
  format: MediaFormat;
  formatConfidence: Exclude<AnimationStatus, "unknown"> | "unknown";
  size?: number;
}

export interface NormalizedMediaInput {
  candidates: MediaCandidate[];
  rejectedCount: number;
}

const supportedExtensions: Record<string, Exclude<MediaFormat, "unknown">> = {
  gif: "gif",
  png: "png",
  webp: "webp",
};

function allowedUrl(value: string): string | undefined {
  try {
    const url = new URL(value.trim());
    return url.protocol === "https:" ? url.toString() : undefined;
  } catch {
    return undefined;
  }
}

function urlsFromUriList(value: string): string[] {
  return value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("#"));
}

export function extractHtmlUrls(html: string): string[] {
  const attributePattern = /\b(?:src|href)\s*=\s*["']([^"']+)["']/gi;
  return Array.from(html.matchAll(attributePattern), (match) => match[1]);
}

function appendUrlCandidate(
  candidates: MediaCandidate[],
  origin: UrlMediaCandidate["origin"],
  value: string,
): boolean {
  const url = allowedUrl(value);
  if (
    !url ||
    candidates.some(
      (candidate) => candidate.kind === "url" && candidate.url === url,
    )
  ) {
    return false;
  }

  candidates.push({ kind: "url", origin, url });
  return true;
}

export function normalizeReceivedInput(
  input: ReceivedInput,
): NormalizedMediaInput {
  const candidates: MediaCandidate[] = input.files.map((file) => ({
    kind: "file",
    origin: "file",
    file,
  }));
  let rejectedCount = 0;

  for (const url of urlsFromUriList(input.uriList ?? "")) {
    if (!appendUrlCandidate(candidates, "text/uri-list", url))
      rejectedCount += 1;
  }

  if (
    input.plainText?.trim() &&
    !appendUrlCandidate(candidates, "text/plain", input.plainText)
  ) {
    rejectedCount += 1;
  }

  for (const url of extractHtmlUrls(input.html ?? "")) {
    if (!appendUrlCandidate(candidates, "html", url)) rejectedCount += 1;
  }

  return { candidates, rejectedCount };
}

export function detectMediaFormat(bytes: Uint8Array): MediaFormat {
  if (
    bytes.length >= 13 &&
    String.fromCharCode(...bytes.slice(0, 6)) === "GIF87a"
  ) {
    return "gif";
  }
  if (
    bytes.length >= 13 &&
    String.fromCharCode(...bytes.slice(0, 6)) === "GIF89a"
  ) {
    return "gif";
  }
  if (
    bytes.length >= 16 &&
    String.fromCharCode(...bytes.slice(0, 4)) === "RIFF" &&
    String.fromCharCode(...bytes.slice(8, 12)) === "WEBP"
  ) {
    return "webp";
  }
  if (
    bytes.length >= 24 &&
    [137, 80, 78, 71, 13, 10, 26, 10].every(
      (byte, index) => bytes[index] === byte,
    ) &&
    String.fromCharCode(...bytes.slice(12, 16)) === "IHDR"
  ) {
    return "png";
  }
  return "unknown";
}

function hasPngAnimation(bytes: Uint8Array): boolean {
  for (let index = 8; index + 8 <= bytes.length; ) {
    const length = new DataView(
      bytes.buffer,
      bytes.byteOffset + index,
      4,
    ).getUint32(0);
    const chunk = String.fromCharCode(...bytes.slice(index + 4, index + 8));
    if (chunk === "acTL") return true;
    if (length > bytes.length - index - 12) return false;
    index += length + 12;
  }
  return false;
}

function hasWebpAnimation(bytes: Uint8Array): boolean {
  for (let index = 12; index + 8 <= bytes.length; ) {
    const chunk = String.fromCharCode(...bytes.slice(index, index + 4));
    const length = new DataView(
      bytes.buffer,
      bytes.byteOffset + index + 4,
      4,
    ).getUint32(0, true);
    if (chunk === "VP8X" && index + 8 < bytes.length)
      return (bytes[index + 8] & 0x02) !== 0;
    if (chunk === "ANIM") return true;
    index += 8 + length + (length % 2);
  }
  return false;
}

function hasMultipleGifFrames(bytes: Uint8Array): boolean {
  let imageDescriptors = 0;
  for (const byte of bytes.slice(6)) {
    if (byte === 0x2c) imageDescriptors += 1;
    if (imageDescriptors > 1) return true;
  }
  return false;
}

export function detectAnimation(
  bytes: Uint8Array,
  format: MediaFormat,
): AnimationStatus {
  if (format === "png" && hasPngAnimation(bytes)) return "confirmed";
  if (format === "webp" && hasWebpAnimation(bytes)) return "confirmed";
  if (format === "gif" && hasMultipleGifFrames(bytes)) return "inferred";
  return "unknown";
}

export async function inspectFileCandidate(
  candidate: FileMediaCandidate,
): Promise<InspectedMedia | { error: string }> {
  if (!candidate.file.size) return { error: "O arquivo está vazio." };
  if (candidate.file.size > MAX_MEDIA_BYTES) {
    return { error: "O arquivo excede o limite experimental de 25 MB." };
  }

  try {
    const bytes = new Uint8Array(
      await candidate.file.slice(0, 256 * 1024).arrayBuffer(),
    );
    const format = detectMediaFormat(bytes);
    if (format === "unknown")
      return {
        error: "O arquivo não possui uma assinatura GIF, WebP ou PNG válida.",
      };
    return {
      candidate,
      format,
      formatConfidence: "confirmed",
      animation: detectAnimation(bytes, format),
      size: candidate.file.size,
    };
  } catch {
    return { error: "Não foi possível ler o arquivo recebido." };
  }
}

export function inspectUrlCandidate(
  candidate: UrlMediaCandidate,
): InspectedMedia | { error: string } {
  const extension = new URL(candidate.url).pathname
    .split(".")
    .pop()
    ?.toLowerCase();
  const format = extension
    ? (supportedExtensions[extension] ?? "unknown")
    : "unknown";
  if (format === "unknown") {
    return { error: "A URL não indica um formato GIF, WebP ou PNG suportado." };
  }
  return {
    candidate,
    format,
    formatConfidence: "inferred",
    animation: format === "png" ? "unknown" : "inferred",
  };
}

export function selectMediaCandidate(
  candidates: InspectedMedia[],
): InspectedMedia | undefined {
  const score = (media: InspectedMedia) => {
    if (media.candidate.kind === "file" && media.animation === "confirmed")
      return 500;
    if (media.candidate.kind === "url" && media.animation === "inferred")
      return 400;
    if (media.candidate.kind === "file" && media.animation === "inferred")
      return 300;
    if (media.candidate.kind === "file") return 200;
    return 100;
  };

  return candidates.reduce<InspectedMedia | undefined>(
    (selected, candidate) =>
      !selected || score(candidate) > score(selected) ? candidate : selected,
    undefined,
  );
}

export async function prepareMediaInput(input: ReceivedInput): Promise<{
  error?: string;
  selected?: InspectedMedia;
}> {
  const normalized = normalizeReceivedInput(input);
  return prepareNormalizedMediaInput(normalized);
}

export async function prepareNormalizedMediaInput(
  normalized: NormalizedMediaInput,
): Promise<{
  error?: string;
  selected?: InspectedMedia;
}> {
  const results = await Promise.all(
    normalized.candidates.map((candidate) =>
      candidate.kind === "file"
        ? inspectFileCandidate(candidate)
        : inspectUrlCandidate(candidate),
    ),
  );
  const inspected = results.filter(
    (result): result is InspectedMedia => "candidate" in result,
  );
  const selected = selectMediaCandidate(inspected);

  if (selected) return { selected };
  return {
    error:
      normalized.candidates.length || normalized.rejectedCount
        ? "Nenhuma mídia GIF, WebP ou PNG válida foi encontrada na entrada."
        : "Nenhuma mídia utilizável foi recebida.",
  };
}
