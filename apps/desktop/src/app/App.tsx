// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { useEffect, useRef, useState } from "react";
import { Button, FluentProvider, makeStyles, tokens } from "@fluentui/react-components";
import { Dismiss20Regular } from "@fluentui/react-icons";
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
  provider: {
    minHeight: "100%",
    backgroundColor: tokens.colorNeutralBackground1,
  },
  shell: {
    width: "min(100%, 620px)",
    minHeight: "100%",
    margin: "0 auto",
    padding: "16px",
    boxSizing: "border-box",
  },
  header: {
    display: "flex",
    alignItems: "center",
    justifyContent: "flex-end",
    marginBottom: "16px",
  },
  headerActions: { display: "flex", gap: "4px" },
});

export function App() {
  const styles = useStyles();
  const [preferences, setPreferences] = useState<Preferences>(() =>
    readPreferences(),
  );
  const importButtonRef = useRef<HTMLButtonElement>(null);
  const initialShortcutRef = useRef(preferences.shortcut);
  const systemDark = useSystemDarkMode();
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
    <FluentProvider
      className={styles.provider}
      theme={resolveTheme(preferences.theme, systemDark)}
    >
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
          <div className={styles.headerActions}>
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
      </main>
    </FluentProvider>
  );
}
