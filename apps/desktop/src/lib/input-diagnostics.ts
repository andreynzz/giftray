// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

export type DetectedInputKind = "URL" | "HTML" | "arquivo" | "imagem";

export function detectInputTypes(
  types: Iterable<string>,
  files: Iterable<File> = [],
): DetectedInputKind[] {
  const received = new Set(Array.from(types, (type) => type.toLowerCase()));
  const detected = new Set<DetectedInputKind>();
  if (received.has("text/uri-list")) detected.add("URL");
  if (received.has("text/html")) detected.add("HTML");
  for (const file of files) {
    detected.add("arquivo");
    if (file.type.startsWith("image/")) detected.add("imagem");
  }
  if (received.has("files")) detected.add("arquivo");
  return [...detected];
}
