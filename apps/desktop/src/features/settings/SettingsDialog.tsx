// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { useState } from "react";
import {
  Button,
  Dialog,
  DialogActions,
  DialogBody,
  DialogContent,
  DialogSurface,
  DialogTitle,
  Field,
  Input,
  Label,
  Radio,
  RadioGroup,
} from "@fluentui/react-components";
import { Settings20Regular } from "@fluentui/react-icons";
import { updateGlobalShortcut } from "../../platform/tauri/window";
import type { Preferences, ThemePreference } from "./preferences";

interface SettingsDialogProps {
  preferences: Preferences;
  onPreferencesChange: (preferences: Preferences) => void;
}

export function SettingsDialog({
  preferences,
  onPreferencesChange,
}: SettingsDialogProps) {
  const [open, setOpen] = useState(false);
  const [shortcutDraft, setShortcutDraft] = useState(preferences.shortcut);
  const [shortcutError, setShortcutError] = useState("");

  const saveShortcut = async () => {
    setShortcutError("");
    try {
      await updateGlobalShortcut(shortcutDraft);
      onPreferencesChange({ ...preferences, shortcut: shortcutDraft });
      setOpen(false);
    } catch {
      setShortcutError(
        "Esse atalho não está disponível. O atalho anterior foi mantido.",
      );
    }
  };

  return (
    <Dialog open={open} onOpenChange={(_, data) => setOpen(data.open)}>
      <Button
        appearance="subtle"
        icon={<Settings20Regular />}
        aria-label="Configurações"
        onClick={() => {
          setShortcutDraft(preferences.shortcut);
          setOpen(true);
        }}
      />
      <DialogSurface>
        <DialogBody>
          <DialogTitle>Configurações</DialogTitle>
          <DialogContent>
            <Field
              label="Atalho global"
              validationMessage={shortcutError}
              validationState={shortcutError ? "error" : "none"}
            >
              <Input
                value={shortcutDraft}
                onChange={(_, data) => setShortcutDraft(data.value)}
                placeholder="Ctrl+Alt+G"
              />
            </Field>
            <Field label="Tema" style={{ marginTop: 16 }}>
              <RadioGroup
                value={preferences.theme}
                onChange={(_, data) =>
                  onPreferencesChange({
                    ...preferences,
                    theme: data.value as ThemePreference,
                  })
                }
              >
                <Radio value="system" label="Usar configuração do Windows" />
                <Radio value="light" label="Claro" />
                <Radio value="dark" label="Escuro" />
              </RadioGroup>
            </Field>
            <Label style={{ marginTop: 12, display: "block" }}>
              O visual usa cores sólidas Fluent; não depende de Mica ou Acrylic.
            </Label>
          </DialogContent>
          <DialogActions>
            <Button appearance="secondary" onClick={() => setOpen(false)}>
              Cancelar
            </Button>
            <Button appearance="primary" onClick={saveShortcut}>
              Salvar
            </Button>
          </DialogActions>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
}
