// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { useEffect, useRef, useState } from "react";
import {
  Badge,
  Button,
  Text,
  Textarea,
  makeStyles,
  shorthands,
} from "@fluentui/react-components";
import { listenForNativeDragDrop } from "../../platform/tauri/input";
import {
  describeHtmlInput,
  describeNativeFileDrop,
  describePasteInput,
  failedInputDiagnostic,
  type InputDiagnostic,
} from "./diagnostics";

const useStyles = makeStyles({
  diagnostics: {
    marginTop: "20px",
    paddingTop: "14px",
    borderTop: "1px solid var(--colorNeutralStroke2)",
  },
  dropZone: {
    marginTop: "8px",
    minHeight: "92px",
    display: "grid",
    placeItems: "center",
    textAlign: "center",
    ...shorthands.border("1px", "dashed", "var(--colorNeutralStroke1)"),
    ...shorthands.borderRadius("8px"),
  },
  badges: { display: "flex", gap: "6px", flexWrap: "wrap", marginTop: "8px" },
  helper: { display: "block", marginTop: "6px" },
});

export function InputDiagnostics() {
  const styles = useStyles();
  const [diagnostic, setDiagnostic] = useState<InputDiagnostic | null>(null);
  const [pasteArmed, setPasteArmed] = useState(false);
  const pasteTargetRef = useRef<HTMLTextAreaElement>(null);
  const pasteArmedRef = useRef(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listenForNativeDragDrop((payload) => {
      if (payload.type === "enter" || payload.type === "drop") {
        setDiagnostic(describeNativeFileDrop(payload.type, payload.paths));
        return;
      }
      setDiagnostic({
        mechanism: "Tauri nativo",
        eventName: payload.type,
        mimeTypes: [],
        fileCount: 0,
        kinds: [],
        status: "detected",
      });
    })
      .then((dispose) => {
        unlisten = dispose;
      })
      .catch(() =>
        setDiagnostic(
          failedInputDiagnostic("Tauri nativo", "registro do listener"),
        ),
      );
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    const onPaste = (event: ClipboardEvent) => {
      if (!pasteArmedRef.current || event.target !== pasteTargetRef.current)
        return;
      pasteArmedRef.current = false;
      setPasteArmed(false);
      event.preventDefault();
      setDiagnostic(
        event.clipboardData
          ? describePasteInput(
              event.clipboardData.types,
              event.clipboardData.files,
            )
          : failedInputDiagnostic("clipboard", "paste (document)"),
      );
    };
    document.addEventListener("paste", onPaste, true);
    return () => document.removeEventListener("paste", onPaste, true);
  }, []);

  const armPaste = () => {
    pasteArmedRef.current = true;
    setPasteArmed(true);
    pasteTargetRef.current?.focus();
  };

  return (
    <section className={styles.diagnostics} aria-labelledby="diagnostics-title">
      <Text id="diagnostics-title" weight="semibold">
        Diagnóstico de importação
      </Text>
      <div
        className={styles.dropZone}
        onDragEnter={(event) => {
          event.preventDefault();
          setDiagnostic(
            describeHtmlInput(
              "HTML5",
              "dragenter",
              event.dataTransfer.types,
              event.dataTransfer.files,
            ),
          );
        }}
        onDragOver={(event) => {
          event.preventDefault();
          setDiagnostic(
            describeHtmlInput(
              "HTML5",
              "dragover",
              event.dataTransfer.types,
              event.dataTransfer.files,
            ),
          );
        }}
        onDragLeave={() =>
          setDiagnostic(describeHtmlInput("HTML5", "dragleave", []))
        }
        onDrop={(event) => {
          event.preventDefault();
          setDiagnostic(
            describeHtmlInput(
              "HTML5",
              "drop",
              event.dataTransfer.types,
              event.dataTransfer.files,
            ),
          );
        }}
        tabIndex={0}
        aria-label="Arraste conteúdo aqui ou cole com Control V"
      >
        <Text>
          Arraste conteúdo aqui ou pressione Ctrl+V.
          <br />O conteúdo não é salvo nem exibido neste experimento.
        </Text>
      </div>
      <Button
        appearance="secondary"
        size="small"
        style={{ marginTop: 8 }}
        onClick={armPaste}
      >
        Ativar colagem de diagnóstico
      </Button>
      <Textarea
        ref={pasteTargetRef}
        aria-label="Destino explícito para colagem de diagnóstico"
        placeholder="Após ativar, pressione Ctrl+V aqui. O conteúdo não será exibido."
        resize="vertical"
        rows={2}
        style={{ marginTop: 8, width: "100%" }}
        onInput={(event) => {
          event.currentTarget.value = "";
        }}
        onKeyDown={(event) => {
          if (!(event.ctrlKey && event.key.toLowerCase() === "v"))
            event.preventDefault();
        }}
      />
      {pasteArmed && (
        <Text className={styles.helper} size={200}>
          Captura armada: cole na área dedicada.
        </Text>
      )}
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
    </section>
  );
}
