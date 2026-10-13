// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import {
  Badge,
  Button,
  Dialog,
  DialogActions,
  DialogBody,
  DialogContent,
  DialogSurface,
  DialogTitle,
  Field,
  FluentProvider,
  Input,
  Label,
  Radio,
  RadioGroup,
  Text,
  makeStyles,
  shorthands,
  teamsDarkTheme,
  teamsLightTheme,
} from "@fluentui/react-components";
import { Dismiss20Regular, Settings20Regular } from "@fluentui/react-icons";
import {
  detectInputTypes,
  type DetectedInputKind,
} from "./lib/input-diagnostics";
import {
  readPreferences,
  writePreferences,
  type Preferences,
  type ThemePreference,
} from "./lib/preferences";

const samples = [
  { title: "Aprovação entusiasmada", tags: "aprovação, festa" },
  { title: "Pensando no assunto", tags: "reação, dúvida" },
  { title: "Vitória discreta", tags: "comemoração, sucesso" },
];

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
  search: { width: "100%" },
  list: { display: "grid", gap: "6px", marginTop: "12px" },
  result: {
    textAlign: "left",
    justifyContent: "flex-start",
    minHeight: "52px",
  },
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

function resolveTheme(preference: ThemePreference, systemDark: boolean) {
  return preference === "dark" || (preference === "system" && systemDark)
    ? teamsDarkTheme
    : teamsLightTheme;
}

export function App() {
  const styles = useStyles();
  const [preferences, setPreferences] = useState<Preferences>(() =>
    readPreferences(),
  );
  const [systemDark, setSystemDark] = useState(
    () => matchMedia("(prefers-color-scheme: dark)").matches,
  );
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const [diagnostics, setDiagnostics] = useState<DetectedInputKind[]>([]);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [shortcutDraft, setShortcutDraft] = useState(preferences.shortcut);
  const [shortcutError, setShortcutError] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
  const initialShortcutRef = useRef(preferences.shortcut);
  const filtered = useMemo(
    () =>
      samples.filter((item) =>
        `${item.title} ${item.tags}`
          .toLowerCase()
          .includes(query.toLowerCase()),
      ),
    [query],
  );

  useEffect(() => {
    const media = matchMedia("(prefers-color-scheme: dark)");
    const update = () => setSystemDark(media.matches);
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);

  useEffect(() => {
    const focus = () => inputRef.current?.focus();
    focus();
    let unlisten: (() => void) | undefined;
    listen("giftray://opened", focus)
      .then((dispose) => {
        unlisten = dispose;
      })
      .catch(() => undefined);
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    // A preferência só é aplicada ao iniciar o app; a falha não altera o
    // valor salvo e o atalho padrão nativo continua disponível.
    invoke("update_global_shortcut", { shortcut: initialShortcutRef.current }).catch(() => undefined);
  }, []);

  const updatePreferences = (next: Preferences) => {
    setPreferences(next);
    writePreferences(next);
  };

  const close = () => invoke("hide_main_window").catch(() => window.close());
  const inspect = (types: Iterable<string>, files?: Iterable<File>) =>
    setDiagnostics(detectInputTypes(types, files));

  const saveShortcut = async () => {
    setShortcutError("");
    try {
      await invoke("update_global_shortcut", { shortcut: shortcutDraft });
      updatePreferences({ ...preferences, shortcut: shortcutDraft });
      setSettingsOpen(false);
    } catch {
      setShortcutError(
        "Esse atalho não está disponível. O atalho anterior foi mantido.",
      );
    }
  };

  return (
    <FluentProvider theme={resolveTheme(preferences.theme, systemDark)}>
      <main
        className={styles.shell}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            close();
          }
          if (event.key === "ArrowDown") {
            event.preventDefault();
            setSelected((value) =>
              Math.min(value + 1, Math.max(filtered.length - 1, 0)),
            );
          }
          if (event.key === "ArrowUp") {
            event.preventDefault();
            setSelected((value) => Math.max(value - 1, 0));
          }
        }}
      >
        <div className={styles.header}>
          <Text weight="semibold" size={500}>
            GIFTray
          </Text>
          <div>
            <Button
              appearance="subtle"
              icon={<Settings20Regular />}
              aria-label="Configurações"
              onClick={() => {
                setShortcutDraft(preferences.shortcut);
                setSettingsOpen(true);
              }}
            />
            <Button
              appearance="subtle"
              icon={<Dismiss20Regular />}
              aria-label="Fechar"
              onClick={close}
            />
          </div>
        </div>
        <Input
          ref={inputRef}
          className={styles.search}
          size="large"
          placeholder="Pesquisar GIFs"
          value={query}
          onChange={(_, data) => {
            setQuery(data.value);
            setSelected(0);
          }}
          aria-label="Pesquisar GIFs"
        />
        <div
          className={styles.list}
          role="listbox"
          aria-label="Resultados da pesquisa"
        >
          {filtered.map((item, index) => (
            <Button
              key={item.title}
              className={styles.result}
              appearance={index === selected ? "secondary" : "subtle"}
              onClick={() => setSelected(index)}
            >
              <span>
                <strong>{item.title}</strong>
                <br />
                <Text size={200}>{item.tags}</Text>
              </span>
            </Button>
          ))}
          {filtered.length === 0 && (
            <Text>Nenhum GIF de exemplo encontrado.</Text>
          )}
        </div>
        <section
          className={styles.diagnostics}
          aria-labelledby="diagnostics-title"
        >
          <Text id="diagnostics-title" weight="semibold">
            Diagnóstico de importação
          </Text>
          <div
            className={styles.dropZone}
            onDragOver={(event) => event.preventDefault()}
            onDrop={(event) => {
              event.preventDefault();
              inspect(event.dataTransfer.types, event.dataTransfer.files);
            }}
            onPaste={(event) =>
              inspect(event.clipboardData.types, event.clipboardData.files)
            }
            tabIndex={0}
            aria-label="Arraste conteúdo aqui ou cole com Control V"
          >
            <Text>
              Arraste conteúdo aqui ou pressione Ctrl+V.
              <br />O conteúdo não é salvo nem exibido neste experimento.
            </Text>
          </div>
          <Text className={styles.helper} size={200}>
            Tipos detectados:
          </Text>
          <div className={styles.badges}>
            {diagnostics.length ? (
              diagnostics.map((kind) => (
                <Badge key={kind} appearance="filled">
                  {kind}
                </Badge>
              ))
            ) : (
              <Text size={200}>aguardando uma ação explícita</Text>
            )}
          </div>
        </section>
      </main>
      <Dialog
        open={settingsOpen}
        onOpenChange={(_, data) => setSettingsOpen(data.open)}
      >
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
                    updatePreferences({
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
                O visual usa cores sólidas Fluent; não depende de Mica ou
                Acrylic.
              </Label>
            </DialogContent>
            <DialogActions>
              <Button
                appearance="secondary"
                onClick={() => setSettingsOpen(false)}
              >
                Cancelar
              </Button>
              <Button appearance="primary" onClick={saveShortcut}>
                Salvar
              </Button>
            </DialogActions>
          </DialogBody>
        </DialogSurface>
      </Dialog>
    </FluentProvider>
  );
}
