// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

export type ReceivedInputOrigin = "drop" | "paste";

export interface ReceivedInput {
  files: File[];
  html?: string;
  origin: ReceivedInputOrigin;
  plainText?: string;
  uriList?: string;
}

function readText(transfer: DataTransfer, type: string): string | undefined {
  const value = transfer.getData(type);
  return value || undefined;
}

export function readReceivedInput(
  origin: ReceivedInputOrigin,
  transfer: DataTransfer,
): ReceivedInput {
  return {
    origin,
    files: Array.from(transfer.files),
    uriList: readText(transfer, "text/uri-list"),
    plainText: readText(transfer, "text/plain"),
    html: readText(transfer, "text/html"),
  };
}
