// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { Badge, Text, makeStyles } from "@fluentui/react-components";
import type { InputDiagnostic } from "../diagnostics";

const useStyles = makeStyles({
  badges: { display: "flex", gap: "6px", flexWrap: "wrap", marginTop: "8px" },
  helper: { display: "block", marginTop: "6px" },
});

interface DiagnosticResultProps {
  diagnostic: InputDiagnostic | null;
}

export function DiagnosticResult({ diagnostic }: DiagnosticResultProps) {
  const styles = useStyles();

  return (
    <>
      <Text className={styles.helper} size={200}>
        Tipos detectados:
      </Text>
      <div className={styles.badges}>
        {diagnostic ? (
          <>
            <Badge
              appearance={
                diagnostic.status === "detected" ? "filled" : "outline"
              }
            >
              {diagnostic.mechanism}
            </Badge>
            <Badge appearance="outline">
              {diagnostic.status}: {diagnostic.eventName}
            </Badge>
            <Badge appearance="outline">
              {diagnostic.fileCount} arquivo(s)
            </Badge>
            {diagnostic.mimeTypes.map((type) => (
              <Badge key={type} appearance="outline">
                {type}
              </Badge>
            ))}
            {diagnostic.kinds.map((kind) => (
              <Badge key={kind} appearance="filled">
                {kind}
              </Badge>
            ))}
            {!diagnostic.mimeTypes.length &&
              diagnostic.mechanism === "Tauri nativo" && (
                <Text size={200}>A API nativa fornece arquivos, não MIME.</Text>
              )}
            {diagnostic.status === "failed" && (
              <Text size={200}>
                A operação falhou; tente novamente com a estratégia indicada.
              </Text>
            )}
          </>
        ) : (
          <Text size={200}>aguardando uma ação explícita</Text>
        )}
      </div>
    </>
  );
}
