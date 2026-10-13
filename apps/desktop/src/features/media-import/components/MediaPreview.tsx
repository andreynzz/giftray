// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import {
  Badge,
  Button,
  Spinner,
  Text,
  makeStyles,
  shorthands,
} from "@fluentui/react-components";
import type { MediaPreviewState } from "../hooks/useMediaImport";

const useStyles = makeStyles({
  section: {
    marginTop: "20px",
    paddingTop: "14px",
    borderTop: "1px solid var(--colorNeutralStroke2)",
  },
  content: {
    marginTop: "8px",
    minHeight: "112px",
    display: "grid",
    placeItems: "center",
    textAlign: "center",
    ...shorthands.border("1px", "solid", "var(--colorNeutralStroke2)"),
    ...shorthands.borderRadius("8px"),
    overflow: "hidden",
  },
  image: {
    display: "block",
    maxWidth: "100%",
    maxHeight: "260px",
    objectFit: "contain",
  },
  metadata: { display: "flex", gap: "6px", flexWrap: "wrap", marginTop: "8px" },
  helper: { display: "block", marginTop: "6px" },
  actions: { marginTop: "8px" },
});

interface MediaPreviewProps {
  onClear: () => void;
  onLoad: () => void;
  onLoadError: () => void;
  preview: MediaPreviewState;
}

function formatLabel(format: string) {
  return format.toUpperCase();
}

export function MediaPreview({
  onClear,
  onLoad,
  onLoadError,
  preview,
}: MediaPreviewProps) {
  const styles = useStyles();
  const loadingOrReady =
    preview.status === "loading" || preview.status === "ready";

  return (
    <section className={styles.section} aria-labelledby="media-preview-title">
      <Text id="media-preview-title" weight="semibold">
        Prévia experimental
      </Text>
      <div className={styles.content} aria-live="polite">
        {preview.status === "idle" && (
          <Text size={200}>
            Arraste ou cole uma mídia na área de diagnóstico para visualizá-la.
          </Text>
        )}
        {preview.status === "processing" && (
          <Spinner label="Processando entrada" />
        )}
        {preview.status === "error" && (
          <Text size={200}>{preview.message}</Text>
        )}
        {loadingOrReady && (
          <>
            <img
              className={styles.image}
              src={preview.src}
              alt="Prévia da mídia importada"
              onLoad={onLoad}
              onError={onLoadError}
            />
            {preview.status === "loading" && (
              <Spinner size="tiny" label="Carregando mídia" />
            )}
          </>
        )}
      </div>
      {loadingOrReady && (
        <div className={styles.metadata}>
          <Badge appearance="filled">
            {preview.media.candidate.kind === "file"
              ? "arquivo local"
              : "URL remota"}
          </Badge>
          <Badge appearance="outline">
            {formatLabel(preview.media.format)} {preview.media.formatConfidence}
          </Badge>
          <Badge appearance="outline">animação {preview.media.animation}</Badge>
          {preview.media.size !== undefined && (
            <Badge appearance="outline">
              {Math.ceil(preview.media.size / 1024)} KB
            </Badge>
          )}
        </div>
      )}
      {preview.status !== "idle" && (
        <Button
          className={styles.actions}
          appearance="secondary"
          size="small"
          onClick={onClear}
        >
          Limpar prévia
        </Button>
      )}
      <Text className={styles.helper} size={200}>
        A mídia é mantida apenas em memória nesta sessão e não é salva.
      </Text>
    </section>
  );
}
