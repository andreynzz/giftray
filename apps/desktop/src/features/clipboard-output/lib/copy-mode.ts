// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import type { MediaPreviewState } from "../../media-import/hooks/useMediaImport";

export type CopyMode = "link" | "file";

export function canCopyLink(preview: MediaPreviewState): boolean {
  return (
    (preview.status === "loading" || preview.status === "ready") &&
    preview.media.candidate.kind === "url" &&
    new URL(preview.media.candidate.url).protocol === "https:"
  );
}

export function canCopyFile(preview: MediaPreviewState): boolean {
  return (
    (preview.status === "loading" || preview.status === "ready") &&
    (preview.media.candidate.kind === "file" || preview.media.candidate.kind === "url")
  );
}

export function selectedCopyMode(preview: MediaPreviewState): CopyMode | undefined {
  if (canCopyLink(preview)) return "link";
  if (canCopyFile(preview)) return "file";
  return undefined;
}
