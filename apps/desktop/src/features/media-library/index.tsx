// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import { Badge, Button, MessageBar, MessageBarBody, Spinner, Text, makeStyles, shorthands, tokens } from "@fluentui/react-components";
import { Add20Regular, ArrowClockwise20Regular, Copy20Regular, Dismiss20Regular, Document20Regular } from "@fluentui/react-icons";
import { copyLibraryItem, listLibraryItems, selectAndImportMedia, type LibraryItem } from "../../platform/tauri/library";
import { copyFailureMessage, importFailureMessage } from "./lib/library-feedback";
import { canCopyLibraryItem, formatByteSize, libraryItemLabel, sortLibraryItems } from "./lib/library-presentation";

const useStyles = makeStyles({
  section: { display: "grid", gap: "12px" },
  toolbar: { display: "flex", alignItems: "center", justifyContent: "space-between", gap: "12px", flexWrap: "wrap" },
  title: { display: "grid", gap: "2px" },
  list: { display: "grid", gap: "4px", margin: 0, padding: 0, listStyleType: "none" },
  item: { display: "flex", alignItems: "center", gap: "10px", minHeight: "48px", padding: "8px 10px", backgroundColor: tokens.colorNeutralBackground1, ...shorthands.border("1px", "solid", tokens.colorNeutralStroke2), ...shorthands.borderRadius(tokens.borderRadiusMedium) },
  itemIcon: { color: tokens.colorBrandForeground1, flexShrink: 0 },
  itemContent: { minWidth: 0, display: "grid", gap: "3px", flexGrow: 1 },
  itemName: { overflowX: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" },
  metadata: { display: "flex", gap: "6px", flexWrap: "wrap" },
  empty: { display: "grid", justifyItems: "start", gap: "8px", padding: "20px 16px", backgroundColor: tokens.colorNeutralBackground2, ...shorthands.borderRadius(tokens.borderRadiusMedium) },
  loading: { display: "flex", alignItems: "center", minHeight: "80px" },
  feedback: { display: "block" },
});

type Feedback = { intent: "success" | "info" | "error"; message: string } | null;

interface MediaLibraryProps { importButtonRef?: RefObject<HTMLButtonElement | null>; }

export function MediaLibrary({ importButtonRef }: MediaLibraryProps) {
  const styles = useStyles();
  const [items, setItems] = useState<LibraryItem[]>([]);
  const [state, setState] = useState<"loading" | "ready" | "error">("loading");
  const [importing, setImporting] = useState(false);
  const [copyingIds, setCopyingIds] = useState<Set<string>>(() => new Set());
  const [feedback, setFeedback] = useState<Feedback>(null);
  const localImportButtonRef = useRef<HTMLButtonElement>(null);
  const isImportingRef = useRef(false);
  const copyingIdsRef = useRef(new Set<string>());
  const buttonRef = importButtonRef ?? localImportButtonRef;

  const loadItems = useCallback(async () => {
    setState("loading");
    try {
      setItems(sortLibraryItems(await listLibraryItems()));
      setState("ready");
    } catch {
      setState("error");
    }
  }, []);

  useEffect(() => {
    let active = true;
    listLibraryItems()
      .then((loadedItems) => {
        if (!active) return;
        setItems(sortLibraryItems(loadedItems));
        setState("ready");
      })
      .catch(() => {
        if (active) setState("error");
      });
    return () => { active = false; };
  }, []);

  useEffect(() => {
    if (!feedback || feedback.intent === "error") return;
    const timeout = window.setTimeout(() => setFeedback(null), 5000);
    return () => window.clearTimeout(timeout);
  }, [feedback]);

  const importMedia = async () => {
    if (isImportingRef.current) return;
    isImportingRef.current = true;
    setImporting(true);
    setFeedback(null);
    try {
      const result = await selectAndImportMedia();
      if (result.status === "cancelled") {
        buttonRef.current?.focus();
        return;
      }
      setItems((current) => sortLibraryItems([result.item, ...current.filter((item) => item.id !== result.item.id)]));
      setState("ready");
      setFeedback({
        intent: result.status === "imported" ? "success" : "info",
        message: result.status === "imported" ? "Arquivo salvo na biblioteca." : "Esse arquivo já está na biblioteca.",
      });
    } catch (error: unknown) {
      setFeedback({ intent: "error", message: importFailureMessage(error) });
    } finally {
      isImportingRef.current = false;
      setImporting(false);
    }
  };

  const copyItem = async (item: LibraryItem) => {
    if (!canCopyLibraryItem(item) || copyingIdsRef.current.has(item.id)) return;
    copyingIdsRef.current.add(item.id);
    setCopyingIds(new Set(copyingIdsRef.current));
    setFeedback(null);
    try {
      await copyLibraryItem(item.id);
      setFeedback({ intent: "success", message: "Arquivo copiado. Cole com Ctrl+V." });
    } catch (error: unknown) {
      setFeedback({ intent: "error", message: copyFailureMessage(error) });
    } finally {
      copyingIdsRef.current.delete(item.id);
      setCopyingIds(new Set(copyingIdsRef.current));
    }
  };

  return <section className={styles.section} aria-labelledby="library-title">
    <div className={styles.toolbar}>
      <div className={styles.title}>
        <Text id="library-title" as="h1" size={500} weight="semibold">Biblioteca</Text>
        {state === "ready" && <Text size={200}>{items.length} {items.length === 1 ? "arquivo salvo" : "arquivos salvos"}</Text>}
      </div>
      <Button ref={buttonRef} appearance="primary" icon={<Add20Regular />} disabled={importing} onClick={() => void importMedia()}>
        Importar arquivo
      </Button>
    </div>
    {importing && <Text className={styles.feedback} role="status">Importando arquivo…</Text>}
    {feedback && feedback.intent !== "error" && <Text className={styles.feedback} role="status">{feedback.message}</Text>}
    {feedback?.intent === "error" && <MessageBar intent="error"><MessageBarBody>
      {feedback.message} <Button appearance="transparent" size="small" icon={<Dismiss20Regular />} aria-label="Dispensar erro" onClick={() => setFeedback(null)} />
    </MessageBarBody></MessageBar>}
    {state === "loading" && <div className={styles.loading}><Spinner label="Carregando biblioteca…" /></div>}
    {state === "error" && <MessageBar intent="error"><MessageBarBody>
      Não foi possível abrir sua biblioteca. <Button appearance="transparent" size="small" icon={<ArrowClockwise20Regular />} onClick={() => void loadItems()}>Tentar novamente</Button>
    </MessageBarBody></MessageBar>}
    {state === "ready" && items.length === 0 && <div className={styles.empty}>
      <Text weight="semibold">Sua biblioteca está vazia</Text><Text size={200}>Importe seu primeiro GIF, WebP ou PNG.</Text>
    </div>}
    {state === "ready" && items.length > 0 && <ul className={styles.list} aria-label="Arquivos salvos">
      {items.map((item) => <li key={item.id} className={styles.item}>
        <Document20Regular className={styles.itemIcon} aria-hidden="true" />
        <div className={styles.itemContent}>
          <Text className={styles.itemName} weight="semibold" title={libraryItemLabel(item)}>{libraryItemLabel(item)}</Text>
          <div className={styles.metadata}>
            <Badge appearance="outline">{item.format.toUpperCase()}</Badge><Text size={200}>{formatByteSize(item.byteSize)}</Text>
            {item.availability === "missing" && <Badge color="warning">Arquivo indisponível</Badge>}
          </div>
        </div>
        <Button
          appearance="secondary"
          icon={<Copy20Regular />}
          disabled={!canCopyLibraryItem(item) || copyingIds.has(item.id)}
          onClick={() => void copyItem(item)}
        >
          {copyingIds.has(item.id) ? "Copiando…" : "Copiar arquivo"}
        </Button>
      </li>)}
    </ul>}
  </section>;
}
