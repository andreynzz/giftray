// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { useEffect, useRef, useState } from "react";
import {
  Accordion,
  AccordionHeader,
  AccordionItem,
  AccordionPanel,
  Button,
  FluentProvider,
  Text,
  makeStyles,
} from "@fluentui/react-components";
import { Dismiss20Regular } from "@fluentui/react-icons";
import { MediaImport, useMediaImport } from "../features/media-import";
import { InputDiagnostics } from "../features/input-diagnostics/InputDiagnostics";
import { ClipboardOutput } from "../features/clipboard-output";
import { MediaLibrary } from "../features/media-library";
import {
  readPreferences,
  writePreferences,
  type Preferences,
} from "../features/settings/preferences";
import { SettingsDialog } from "../features/settings/SettingsDialog";
import { hideMainWindow, updateGlobalShortcut } from "../platform/tauri/window";
import { resolveTheme, useSystemDarkMode } from "./theme";
import { useSearchWindowFocus } from "./use-search-window-focus";

const useStyles = makeStyles({
  shell: {
    width: "min(100%, 620px)",
    margin: "0 auto",
    padding: "20px",
    boxSizing: "border-box",
  },
  header: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    marginBottom: "12px",
  },
  tools: { marginTop: "20px" },
});

export function App() {
  const styles = useStyles();
  const [preferences, setPreferences] = useState<Preferences>(() =>
    readPreferences(),
  );
  const importButtonRef = useRef<HTMLButtonElement>(null);
  const initialShortcutRef = useRef(preferences.shortcut);
  const systemDark = useSystemDarkMode();
  const mediaImport = useMediaImport();
  useSearchWindowFocus(importButtonRef);

  useEffect(() => {
    updateGlobalShortcut(initialShortcutRef.current).catch(() => undefined);
  }, []);

  const updatePreferences = (next: Preferences) => {
    setPreferences(next);
    writePreferences(next);
  };
  const close = () => hideMainWindow().catch(() => window.close());

  return (
    <FluentProvider theme={resolveTheme(preferences.theme, systemDark)}>
      <main
        className={styles.shell}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            close();
          }
        }}
      >
        <header className={styles.header}>
          <Text weight="semibold" size={500}>
            GIFTray
          </Text>
          <div>
            <SettingsDialog
              preferences={preferences}
              onPreferencesChange={updatePreferences}
            />
            <Button
              appearance="subtle"
              icon={<Dismiss20Regular />}
              aria-label="Fechar"
              onClick={close}
            />
          </div>
        </header>
        <MediaLibrary importButtonRef={importButtonRef} />
        <Accordion collapsible className={styles.tools}>
          <AccordionItem value="p0-tools">
            <AccordionHeader>Ferramentas de teste da P0</AccordionHeader>
            <AccordionPanel>
              <InputDiagnostics onInput={mediaImport.receiveInput} />
              <MediaImport
                preview={mediaImport.preview}
                onClear={mediaImport.clear}
                onLoad={mediaImport.markLoaded}
                onLoadError={mediaImport.markFailed}
              />
              <ClipboardOutput preview={mediaImport.preview} />
            </AccordionPanel>
          </AccordionItem>
        </Accordion>
      </main>
    </FluentProvider>
  );
}
