// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { useCallback, useState } from "react";
import type { MediaPreviewState } from "../../media-import/hooks/useMediaImport";
import { copyFileToWindowsClipboard, copyText } from "../../../platform/tauri/clipboard";
import { canCopyFile, canCopyLink } from "../lib/copy-mode";

export type ClipboardOutputStatus = "idle" | "copying-link" | "copying-file" | "success" | "error";

export function useClipboardOutput() {
  const [status, setStatus] = useState<ClipboardOutputStatus>("idle");
  const [message, setMessage] = useState<string | undefined>();

  const copyLink = useCallback(async (preview: MediaPreviewState) => {
    if (
      !canCopyLink(preview) ||
      (preview.status !== "loading" && preview.status !== "ready") ||
      preview.media.candidate.kind !== "url"
    ) return;
    setStatus("copying-link");
    setMessage(undefined);
    try {
      await copyText(preview.media.candidate.url);
      setStatus("success");
      setMessage("Link copiado. Cole-o manualmente no aplicativo de destino.");
    } catch {
      setStatus("error");
      setMessage("Não foi possível copiar o link para a área de transferência.");
    }
  }, []);

  const copyFile = useCallback(async (preview: MediaPreviewState) => {
    if (
      !canCopyFile(preview) ||
      (preview.status !== "loading" && preview.status !== "ready") ||
      preview.media.candidate.kind !== "file"
    ) return;
    setStatus("copying-file");
    setMessage(undefined);
    try {
      const { file } = preview.media.candidate;
      await copyFileToWindowsClipboard(file.name, new Uint8Array(await file.arrayBuffer()));
      setStatus("success");
      setMessage("Arquivo preparado e copiado. Cole-o manualmente no aplicativo de destino.");
    } catch {
      setStatus("error");
      setMessage("Não foi possível preparar o arquivo para a área de transferência.");
    }
  }, []);

  return { copyFile, copyLink, message, status };
}
