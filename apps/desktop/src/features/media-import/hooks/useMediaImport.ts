// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { useCallback, useEffect, useRef, useState } from "react";
import type { ReceivedInput } from "../../../shared/received-input";
import type { InspectedMedia } from "../lib/media";
import { prepareMediaInput } from "../lib/media";
import { createTemporaryUrl, releaseTemporaryUrl } from "../lib/temporary-url";

export type MediaPreviewState =
  | { status: "idle" }
  | { status: "error"; message: string }
  | { status: "processing" }
  | { status: "loading"; media: InspectedMedia; src: string }
  | { status: "ready"; media: InspectedMedia; src: string };

function previewSource(media: InspectedMedia): string {
  return media.candidate.kind === "file"
    ? createTemporaryUrl(media.candidate.file)
    : media.candidate.url;
}

export function useMediaImport() {
  const [preview, setPreview] = useState<MediaPreviewState>({ status: "idle" });
  const temporaryUrlRef = useRef<string | undefined>(undefined);
  const requestIdRef = useRef(0);

  const releaseCurrentUrl = useCallback(() => {
    releaseTemporaryUrl(temporaryUrlRef.current);
    temporaryUrlRef.current = undefined;
  }, []);

  const clear = useCallback(() => {
    requestIdRef.current += 1;
    releaseCurrentUrl();
    setPreview({ status: "idle" });
  }, [releaseCurrentUrl]);

  const receiveInput = useCallback(
    (input: ReceivedInput) => {
      const requestId = requestIdRef.current + 1;
      requestIdRef.current = requestId;
      releaseCurrentUrl();
      setPreview({ status: "processing" });

      void prepareMediaInput(input).then((result) => {
        if (requestId !== requestIdRef.current) return;
        if (!result.selected) {
          setPreview({
            status: "error",
            message: result.error ?? "A importação falhou.",
          });
          return;
        }

        const src = previewSource(result.selected);
        if (result.selected.candidate.kind === "file")
          temporaryUrlRef.current = src;
        setPreview({ status: "loading", media: result.selected, src });
      });
    },
    [releaseCurrentUrl],
  );

  const markLoaded = useCallback(() => {
    setPreview((current) =>
      current.status === "loading" ? { ...current, status: "ready" } : current,
    );
  }, []);

  const markFailed = useCallback(() => {
    releaseCurrentUrl();
    setPreview({
      status: "error",
      message:
        "Não foi possível carregar a mídia selecionada. A origem pode ter expirado ou bloqueado a visualização.",
    });
  }, [releaseCurrentUrl]);

  useEffect(() => releaseCurrentUrl, [releaseCurrentUrl]);

  return { clear, markFailed, markLoaded, preview, receiveInput };
}
