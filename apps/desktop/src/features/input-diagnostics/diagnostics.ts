// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

export type DetectedInputKind = "URL" | "HTML" | "arquivo" | "imagem";
export type InputMechanism = "HTML5" | "Tauri nativo" | "clipboard";

export interface InputDiagnostic {
  eventName: string;
  fileCount: number;
  kinds: DetectedInputKind[];
  mechanism: InputMechanism;
  mimeTypes: string[];
  status: "detected" | "failed";
}

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

export function listMimeTypes(types: Iterable<string>): string[] {
  return [...new Set(Array.from(types, (type) => type.toLowerCase()))].sort();
}

export function describeHtmlInput(
  mechanism: Extract<InputMechanism, "HTML5" | "clipboard">,
  eventName: string,
  types: Iterable<string>,
  files: Iterable<File> = [],
): InputDiagnostic {
  const fileList = [...files];
  return {
    mechanism,
    eventName,
    mimeTypes: listMimeTypes(types),
    fileCount: fileList.length,
    kinds: detectInputTypes(types, fileList),
    status: "detected",
  };
}

export function describePasteInput(
  types: Iterable<string>,
  files: Iterable<File> = [],
): InputDiagnostic {
  const diagnostic = describeHtmlInput(
    "clipboard",
    "paste (document → textarea)",
    types,
    files,
  );
  return {
    ...diagnostic,
    status:
      diagnostic.mimeTypes.length || diagnostic.fileCount
        ? "detected"
        : "failed",
  };
}

export function describeNativeFileDrop(
  eventName: string,
  paths: string[],
): InputDiagnostic {
  const isImage = paths.some((path) =>
    /\.(avif|bmp|gif|jpe?g|png|svg|webp)$/i.test(path),
  );
  return {
    mechanism: "Tauri nativo",
    eventName,
    mimeTypes: [],
    fileCount: paths.length,
    kinds: paths.length ? (isImage ? ["arquivo", "imagem"] : ["arquivo"]) : [],
    status: "detected",
  };
}

export function failedInputDiagnostic(
  mechanism: InputMechanism,
  eventName: string,
): InputDiagnostic {
  return {
    mechanism,
    eventName,
    mimeTypes: [],
    fileCount: 0,
    kinds: [],
    status: "failed",
  };
}
