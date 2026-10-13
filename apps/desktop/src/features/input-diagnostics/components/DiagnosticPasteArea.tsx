// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { Button, Text, Textarea, makeStyles } from "@fluentui/react-components";
import type { RefObject } from "react";

const useStyles = makeStyles({
  pasteControl: { marginTop: "8px" },
  pasteTarget: { marginTop: "8px", width: "100%" },
  helper: { display: "block", marginTop: "6px" },
});

interface DiagnosticPasteAreaProps {
  armed: boolean;
  pasteTargetRef: RefObject<HTMLTextAreaElement | null>;
  onArm: () => void;
}

export function DiagnosticPasteArea({
  armed,
  pasteTargetRef,
  onArm,
}: DiagnosticPasteAreaProps) {
  const styles = useStyles();

  return (
    <>
      <Button
        className={styles.pasteControl}
        appearance="secondary"
        size="small"
        onClick={onArm}
      >
        Ativar colagem de diagnóstico
      </Button>
      <Textarea
        ref={pasteTargetRef}
        className={styles.pasteTarget}
        aria-label="Destino explícito para colagem de diagnóstico"
        placeholder="Após ativar, pressione Ctrl+V aqui. O conteúdo não será exibido."
        resize="vertical"
        rows={2}
        onInput={(event) => {
          event.currentTarget.value = "";
        }}
        onKeyDown={(event) => {
          if (!(event.ctrlKey && event.key.toLowerCase() === "v")) {
            event.preventDefault();
          }
        }}
      />
      {armed && (
        <Text className={styles.helper} size={200}>
          Captura armada: cole na área dedicada.
        </Text>
      )}
    </>
  );
}
