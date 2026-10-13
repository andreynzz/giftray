// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { Text, makeStyles } from "@fluentui/react-components";
import { DiagnosticDropZone } from "./components/DiagnosticDropZone";
import { DiagnosticPasteArea } from "./components/DiagnosticPasteArea";
import { DiagnosticResult } from "./components/DiagnosticResult";
import { useInputDiagnostics } from "./hooks/useInputDiagnostics";

const useStyles = makeStyles({
  diagnostics: {
    marginTop: "20px",
    paddingTop: "14px",
    borderTop: "1px solid var(--colorNeutralStroke2)",
  },
});

export function InputDiagnostics() {
  const styles = useStyles();
  const {
    diagnostic,
    pasteArmed,
    pasteTargetRef,
    armPaste,
    dropZoneHandlers,
  } = useInputDiagnostics();

  return (
    <section className={styles.diagnostics} aria-labelledby="diagnostics-title">
      <Text id="diagnostics-title" weight="semibold">
        Diagnóstico de importação
      </Text>
      <DiagnosticDropZone {...dropZoneHandlers} />
      <DiagnosticPasteArea
        armed={pasteArmed}
        pasteTargetRef={pasteTargetRef}
        onArm={armPaste}
      />
      <DiagnosticResult diagnostic={diagnostic} />
    </section>
  );
}
