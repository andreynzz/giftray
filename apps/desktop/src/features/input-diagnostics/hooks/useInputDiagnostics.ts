// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { useEffect, useRef, useState } from "react";
import type { DragEventHandler } from "react";
import {
  readReceivedInput,
  type ReceivedInput,
} from "../../../shared/received-input";
import {
  listenForNativeDragDrop,
  type NativeDragDropPayload,
} from "../../../platform/tauri/input";
import {
  describeHtmlInput,
  describeNativeFileDrop,
  describePasteInput,
  failedInputDiagnostic,
  type InputDiagnostic,
} from "../diagnostics";

type Unlisten = () => void;
type RegisterNativeListener = (
  handler: (payload: NativeDragDropPayload) => void,
) => Promise<Unlisten>;

export function registerAsyncNativeListener(
  register: RegisterNativeListener,
  onPayload: (payload: NativeDragDropPayload) => void,
  onError: () => void,
): Unlisten {
  let active = true;
  let unlisten: Unlisten | undefined;

  void register(onPayload)
    .then((dispose) => {
      if (active) {
        unlisten = dispose;
        return;
      }

      dispose();
    })
    .catch(() => {
      if (active) onError();
    });

  return () => {
    active = false;
    unlisten?.();
  };
}

export function useInputDiagnostics(onInput?: (input: ReceivedInput) => void) {
  const [diagnostic, setDiagnostic] = useState<InputDiagnostic | null>(null);
  const [pasteArmed, setPasteArmed] = useState(false);
  const pasteTargetRef = useRef<HTMLTextAreaElement>(null);
  const pasteArmedRef = useRef(false);
  const onInputRef = useRef(onInput);

  useEffect(() => {
    onInputRef.current = onInput;
  }, [onInput]);

  useEffect(() => {
    return registerAsyncNativeListener(
      listenForNativeDragDrop,
      (payload) => {
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
      },
      () =>
        setDiagnostic(
          failedInputDiagnostic("Tauri nativo", "registro do listener"),
        ),
    );
  }, []);

  useEffect(() => {
    const onPaste = (event: ClipboardEvent) => {
      if (!pasteArmedRef.current || event.target !== pasteTargetRef.current) {
        return;
      }

      pasteArmedRef.current = false;
      setPasteArmed(false);
      event.preventDefault();
      setDiagnostic(
        event.clipboardData
          ? describePasteInput(event.clipboardData.types, event.clipboardData.files)
          : failedInputDiagnostic("clipboard", "paste (document)"),
      );
      if (event.clipboardData) {
        onInputRef.current?.(readReceivedInput("paste", event.clipboardData));
      }
    };

    document.addEventListener("paste", onPaste, true);
    return () => document.removeEventListener("paste", onPaste, true);
  }, []);

  const armPaste = () => {
    pasteArmedRef.current = true;
    setPasteArmed(true);
    pasteTargetRef.current?.focus();
  };

  const handleDragEnter: DragEventHandler<HTMLDivElement> = (event) => {
    event.preventDefault();
    setDiagnostic(
      describeHtmlInput(
        "HTML5",
        "dragenter",
        event.dataTransfer.types,
        event.dataTransfer.files,
      ),
    );
  };

  const handleDragOver: DragEventHandler<HTMLDivElement> = (event) => {
    event.preventDefault();
  };

  const handleDragLeave: DragEventHandler<HTMLDivElement> = () => {
    setDiagnostic(describeHtmlInput("HTML5", "dragleave", []));
  };

  const handleDrop: DragEventHandler<HTMLDivElement> = (event) => {
    event.preventDefault();
    setDiagnostic(
      describeHtmlInput(
        "HTML5",
        "drop",
        event.dataTransfer.types,
        event.dataTransfer.files,
      ),
    );
    onInputRef.current?.(readReceivedInput("drop", event.dataTransfer));
  };

  return {
    diagnostic,
    pasteArmed,
    pasteTargetRef,
    armPaste,
    dropZoneHandlers: {
      onDragEnter: handleDragEnter,
      onDragOver: handleDragOver,
      onDragLeave: handleDragLeave,
      onDrop: handleDrop,
    },
  };
}
