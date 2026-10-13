// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { Text, makeStyles, shorthands } from "@fluentui/react-components";
import type { DragEventHandler } from "react";

const useStyles = makeStyles({
  dropZone: {
    marginTop: "8px",
    minHeight: "92px",
    display: "grid",
    placeItems: "center",
    textAlign: "center",
    ...shorthands.border("1px", "dashed", "var(--colorNeutralStroke1)"),
    ...shorthands.borderRadius("8px"),
  },
});

interface DiagnosticDropZoneProps {
  onDragEnter: DragEventHandler<HTMLDivElement>;
  onDragOver: DragEventHandler<HTMLDivElement>;
  onDragLeave: DragEventHandler<HTMLDivElement>;
  onDrop: DragEventHandler<HTMLDivElement>;
}

export function DiagnosticDropZone({
  onDragEnter,
  onDragOver,
  onDragLeave,
  onDrop,
}: DiagnosticDropZoneProps) {
  const styles = useStyles();

  return (
    <div
      className={styles.dropZone}
      onDragEnter={onDragEnter}
      onDragOver={onDragOver}
      onDragLeave={onDragLeave}
      onDrop={onDrop}
      tabIndex={0}
      aria-label="Arraste conteúdo aqui ou cole com Control V"
    >
      <Text>
        Arraste conteúdo aqui ou pressione Ctrl+V.
        <br />O conteúdo não é salvo nem exibido neste experimento.
      </Text>
    </div>
  );
}
