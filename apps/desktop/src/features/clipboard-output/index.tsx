// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { Button, Spinner, Text, makeStyles } from "@fluentui/react-components";
import type { MediaPreviewState } from "../media-import/hooks/useMediaImport";
import { useClipboardOutput } from "./hooks/useClipboardOutput";
import { canCopyFile, canCopyLink } from "./lib/copy-mode";

const useStyles = makeStyles({ section: { marginTop: "10px" }, actions: { display: "flex", gap: "8px", flexWrap: "wrap" }, feedback: { display: "block", marginTop: "6px" } });

export function ClipboardOutput({ preview }: { preview: MediaPreviewState }) {
  const styles = useStyles();
  const { copyFile, copyLink, message, status } = useClipboardOutput();
  const busy = status === "copying-link" || status === "copying-file";
  const linkAvailable = canCopyLink(preview);
  const fileAvailable = canCopyFile(preview);

  return <section className={styles.section} aria-label="Compartilhar mídia">
    <div className={styles.actions}>
      <Button size="small" disabled={!linkAvailable || busy} onClick={() => void copyLink(preview)}>Copiar link</Button>
      <Button size="small" disabled={!fileAvailable || busy} onClick={() => void copyFile(preview)}>Copiar arquivo</Button>
      {busy && <Spinner size="tiny" label="Preparando área de transferência" />}
    </div>
    {message && <Text className={styles.feedback} size={200} role="status">{message}</Text>}
    {!linkAvailable && !fileAvailable && <Text className={styles.feedback} size={200}>Importe um arquivo local para copiar o arquivo, ou uma URL HTTPS para copiar o link.</Text>}
  </section>;
}
