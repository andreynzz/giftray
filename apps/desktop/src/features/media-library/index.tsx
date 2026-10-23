// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import { Badge, Button, Input, MessageBar, MessageBarBody, Spinner, Text, makeStyles, shorthands, tokens } from "@fluentui/react-components";
import { Add20Regular, ArrowClockwise20Regular, Copy20Regular, Dismiss20Regular, Document20Regular } from "@fluentui/react-icons";
import { copyLibraryItem, importRemoteMedia, libraryPreviewUrl, libraryThumbnailUrl, listLibraryItems, renameLibraryItem, selectAndImportMedia, type ImportResult, type LibraryItem } from "../../platform/tauri/library";
import { copyFailureMessage, importFailureMessage, renameFailureMessage } from "./lib/library-feedback";
import { canCopyLibraryItem, canPreviewLibraryItem, canStartRemoteImport, filterLibraryItems, formatByteSize, libraryItemLabel, sortLibraryItems } from "./lib/library-presentation";

const useStyles = makeStyles({
  section: { display: "grid", gap: "12px" },
  toolbar: { display: "flex", alignItems: "center", justifyContent: "space-between", gap: "12px", flexWrap: "wrap" },
  title: { display: "grid", gap: "2px" },
  list: { display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(140px, 1fr))", gap: "8px", margin: 0, padding: 0, listStyleType: "none" },
  item: { display: "grid", gap: "8px", minWidth: 0, padding: "8px", backgroundColor: tokens.colorNeutralBackground1, ...shorthands.border("1px", "solid", tokens.colorNeutralStroke2), ...shorthands.borderRadius(tokens.borderRadiusMedium) },
  mediaButton: { display: "grid", gap: "6px", minWidth: 0, padding: 0, color: tokens.colorNeutralForeground1, backgroundColor: "transparent", border: 0, textAlign: "left", cursor: "pointer", ":focus-visible": { outlineStyle: "solid", outlineWidth: "2px", outlineColor: tokens.colorBrandStroke1, outlineOffset: "2px" }, ":disabled": { cursor: "default" } },
  thumbnail: { display: "grid", placeItems: "center", width: "100%", aspectRatio: "3 / 2", overflow: "hidden", backgroundColor: tokens.colorNeutralBackground3, ...shorthands.borderRadius(tokens.borderRadiusSmall) },
  thumbnailImage: { width: "100%", height: "100%", objectFit: "contain" },
  thumbnailFallback: { color: tokens.colorNeutralForeground3 },
  itemIcon: { color: tokens.colorBrandForeground1, flexShrink: 0 },
  itemContent: { minWidth: 0, display: "grid", gap: "3px", flexGrow: 1 },
  itemName: { overflowX: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" },
  metadata: { display: "flex", gap: "6px", flexWrap: "wrap" },
  empty: { display: "grid", justifyItems: "start", gap: "8px", padding: "20px 16px", backgroundColor: tokens.colorNeutralBackground2, ...shorthands.borderRadius(tokens.borderRadiusMedium) },
  loading: { display: "flex", alignItems: "center", minHeight: "80px" },
  feedback: { display: "block" },
  remoteImport: { display: "flex", gap: "8px", flexWrap: "wrap", alignItems: "end" },
  remoteInput: { flexGrow: 1, minWidth: "180px" },
  search: { width: "100%" },
  renameActions: { display: "flex", gap: "6px", flexWrap: "wrap" },
  preview: { display: "grid", gap: "8px", padding: "12px", backgroundColor: tokens.colorNeutralBackground2, ...shorthands.borderRadius(tokens.borderRadiusMedium) },
  previewMedia: { display: "grid", placeItems: "center", minHeight: "180px", maxHeight: "300px", overflow: "hidden", backgroundColor: tokens.colorNeutralBackground3, ...shorthands.borderRadius(tokens.borderRadiusSmall) },
  previewImage: { width: "100%", height: "100%", maxHeight: "300px", objectFit: "contain" },
});

type Feedback = { intent: "success" | "info" | "error"; message: string } | null;

interface MediaLibraryProps { importButtonRef?: RefObject<HTMLButtonElement | null>; }

function useReducedMotion(): boolean {
  const [reducedMotion, setReducedMotion] = useState(false);
  useEffect(() => {
    const query = window.matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => setReducedMotion(query.matches);
    update();
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, []);
  return reducedMotion;
}

export function MediaLibrary({ importButtonRef }: MediaLibraryProps) {
  const styles = useStyles();
  const [items, setItems] = useState<LibraryItem[]>([]);
  const [state, setState] = useState<"loading" | "ready" | "error">("loading");
  const [importing, setImporting] = useState(false);
  const [remoteUrl, setRemoteUrl] = useState("");
  const [query, setQuery] = useState("");
  const [copyingId, setCopyingId] = useState<string | null>(null);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [renaming, setRenaming] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [thumbnailFailures, setThumbnailFailures] = useState<Set<string>>(() => new Set());
  const [previewResult, setPreviewResult] = useState<{ id: string; state: "ready" | "error" } | null>(null);
  const [feedback, setFeedback] = useState<Feedback>(null);
  const localImportButtonRef = useRef<HTMLButtonElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const renameTriggerRef = useRef<HTMLButtonElement | null>(null);
  const isImportingRef = useRef(false);
  const copyingIdRef = useRef<string | null>(null);
  const isRenamingRef = useRef(false);
  const buttonRef = importButtonRef ?? localImportButtonRef;
  const reducedMotion = useReducedMotion();
  const selectedItem = items.find((item) => item.id === selectedId) ?? null;
  const filteredItems = filterLibraryItems(items, query);
  const previewState = !selectedItem || !canPreviewLibraryItem(selectedItem, reducedMotion)
    ? "idle"
    : previewResult?.id === selectedItem.id ? previewResult.state : "loading";

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
      showImportResult(result);
    } catch (error: unknown) {
      setFeedback({ intent: "error", message: importFailureMessage(error) });
    } finally {
      isImportingRef.current = false;
      setImporting(false);
    }
  };

  const showImportResult = (result: Exclude<ImportResult, { status: "cancelled" }>) => {
    setItems((current) => sortLibraryItems([result.item, ...current.filter((item) => item.id !== result.item.id)]));
    setState("ready");
    setFeedback({
      intent: result.status === "imported" ? "success" : "info",
      message: result.status === "imported" ? "Arquivo salvo na biblioteca." : "Esse arquivo já está na biblioteca.",
    });
  };

  const importRemote = async () => {
    if (!canStartRemoteImport(remoteUrl, isImportingRef.current)) return;
    isImportingRef.current = true;
    setImporting(true);
    setFeedback(null);
    try {
      const result = await importRemoteMedia(remoteUrl);
      showImportResult(result);
      setRemoteUrl("");
    } catch (error: unknown) {
      setFeedback({ intent: "error", message: importFailureMessage(error) });
    } finally {
      isImportingRef.current = false;
      setImporting(false);
    }
  };

  const copyItem = async (item: LibraryItem) => {
    if (!canCopyLibraryItem(item, copyingIdRef.current)) return;
    copyingIdRef.current = item.id;
    setCopyingId(item.id);
    setFeedback(null);
    try {
      await copyLibraryItem(item.id);
      setFeedback({ intent: "success", message: "Arquivo copiado. Cole com Ctrl+V." });
    } catch (error: unknown) {
      setFeedback({ intent: "error", message: copyFailureMessage(error) });
    } finally {
      copyingIdRef.current = null;
      setCopyingId(null);
    }
  };

  const restoreRenameFocus = () => {
    window.requestAnimationFrame(() => {
      const trigger = renameTriggerRef.current;
      if (trigger?.isConnected && !trigger.disabled) {
        trigger.focus();
      } else {
        searchInputRef.current?.focus();
      }
    });
  };

  const beginRename = (item: LibraryItem, trigger: HTMLButtonElement) => {
    if (renaming) return;
    renameTriggerRef.current = trigger;
    setRenamingId(item.id);
    setRenameValue(item.displayName ?? item.originalName ?? "");
  };

  const cancelRename = () => {
    if (renaming) return;
    setRenamingId(null);
    setRenameValue("");
    restoreRenameFocus();
  };

  const saveRename = async (item: LibraryItem) => {
    if (isRenamingRef.current) return;
    isRenamingRef.current = true;
    setRenaming(true);
    setFeedback(null);
    try {
      const updated = await renameLibraryItem(item.id, renameValue);
      setItems((current) => current.map((currentItem) => currentItem.id === updated.id ? updated : currentItem));
      setFeedback({ intent: "success", message: updated.displayName ? "Nome atualizado." : "Nome personalizado removido." });
      setRenamingId(null);
      setRenameValue("");
      restoreRenameFocus();
    } catch (error: unknown) {
      setFeedback({ intent: "error", message: renameFailureMessage(error) });
    } finally {
      isRenamingRef.current = false;
      setRenaming(false);
    }
  };

  const selectItem = (item: LibraryItem) => {
    setSelectedId((current) => current === item.id ? null : item.id);
    setPreviewResult(null);
  };

  const clearSelection = () => {
    setSelectedId(null);
    setPreviewResult(null);
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
    <form className={styles.remoteImport} onSubmit={(event) => { event.preventDefault(); void importRemote(); }}>
      <Input className={styles.remoteInput} type="url" value={remoteUrl} disabled={importing} onChange={(_, data) => setRemoteUrl(data.value)} placeholder="https://exemplo.com/midia.gif" aria-label="URL da mídia" />
      <Button appearance="secondary" type="submit" disabled={!canStartRemoteImport(remoteUrl, importing)}>Salvar URL</Button>
    </form>
    <Input ref={searchInputRef} className={styles.search} value={query} onChange={(_, data) => setQuery(data.value)} placeholder="Buscar por nome ou formato" aria-label="Buscar na biblioteca" />
    {importing && <Text className={styles.feedback} role="status">Importando mídia…</Text>}
    {copyingId && <Text className={styles.feedback} role="status">Copiando arquivo…</Text>}
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
    {selectedItem && <section className={styles.preview} aria-labelledby="preview-title">
      <div className={styles.toolbar}>
        <Text id="preview-title" weight="semibold">Prévia: {libraryItemLabel(selectedItem)}</Text>
        <Button appearance="secondary" size="small" onClick={clearSelection}>Fechar prévia</Button>
      </div>
      {selectedItem.availability === "missing" && <Text role="status">Arquivo indisponível.</Text>}
      {selectedItem.availability === "available" && reducedMotion && <>
        <Text role="status">Prévia animada desativada pela preferência de movimento reduzido.</Text>
        <div className={styles.previewMedia}>
          {thumbnailFailures.has(selectedItem.id)
            ? <Document20Regular className={styles.thumbnailFallback} aria-hidden="true" />
            : <img className={styles.previewImage} src={libraryThumbnailUrl(selectedItem.id)} alt={`Miniatura estática de ${libraryItemLabel(selectedItem)}`} onError={() => setThumbnailFailures((current) => new Set(current).add(selectedItem.id))} />}
        </div>
      </>}
      {selectedItem.availability === "available" && !reducedMotion && <div className={styles.previewMedia}>
        {previewState === "loading" && <Spinner label="Carregando prévia…" />}
        {previewState === "error" && <Text role="status">Não foi possível carregar a prévia.</Text>}
        <img
          key={selectedItem.id}
          className={styles.previewImage}
          src={libraryPreviewUrl(selectedItem.id)}
          alt={`Prévia de ${libraryItemLabel(selectedItem)}`}
          hidden={previewState === "error"}
          onLoad={() => setPreviewResult({ id: selectedItem.id, state: "ready" })}
          onError={() => setPreviewResult({ id: selectedItem.id, state: "error" })}
        />
      </div>}
      <Button appearance="secondary" icon={<Copy20Regular />} disabled={!canCopyLibraryItem(selectedItem, copyingId)} onClick={() => void copyItem(selectedItem)}>
        {copyingId === selectedItem.id ? "Copiando…" : "Copiar arquivo"}
      </Button>
    </section>}
    {state === "ready" && items.length > 0 && filteredItems.length === 0 && <div className={styles.empty}>
      <Text weight="semibold">Nenhum resultado</Text><Text size={200}>Tente outro nome ou formato.</Text>
    </div>}
    {state === "ready" && filteredItems.length > 0 && <ul className={styles.list} aria-label="Arquivos salvos" onKeyDown={(event) => { if (event.key === "Escape") clearSelection(); }}>
      {filteredItems.map((item) => <li key={item.id} className={styles.item}>
        <button
          type="button"
          className={styles.mediaButton}
          aria-pressed={selectedId === item.id}
          aria-label={`Selecionar ${libraryItemLabel(item)}`}
          onClick={() => selectItem(item)}
        >
          <div className={styles.thumbnail}>
            {item.availability === "available" && !thumbnailFailures.has(item.id)
              ? <img className={styles.thumbnailImage} src={libraryThumbnailUrl(item.id)} alt="" loading="lazy" onError={() => setThumbnailFailures((current) => new Set(current).add(item.id))} />
              : <Document20Regular className={styles.thumbnailFallback} aria-hidden="true" />}
          </div>
          <div className={styles.itemContent}>
          <Text className={styles.itemName} weight="semibold" title={libraryItemLabel(item)}>{libraryItemLabel(item)}</Text>
          <div className={styles.metadata}>
            <Badge appearance="outline">{item.format.toUpperCase()}</Badge><Text size={200}>{formatByteSize(item.byteSize)}</Text>
            {item.availability === "missing" && <Badge color="warning">Arquivo indisponível</Badge>}
          </div>
          </div>
        </button>
        <Button
          appearance="secondary"
          icon={<Copy20Regular />}
          disabled={!canCopyLibraryItem(item, copyingId)}
          onClick={() => void copyItem(item)}
        >
          {copyingId === item.id ? "Copiando…" : "Copiar arquivo"}
        </Button>
        {renamingId === item.id
          ? <form className={styles.renameActions} onSubmit={(event) => { event.preventDefault(); void saveRename(item); }}>
              <Input autoFocus value={renameValue} disabled={renaming} onChange={(_, data) => setRenameValue(data.value)} onKeyDown={(event) => { if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); cancelRename(); } }} aria-label={`Novo nome para ${libraryItemLabel(item)}`} />
              <Button appearance="secondary" size="small" type="submit" disabled={renaming}>{renaming ? "Salvando…" : "Salvar"}</Button>
              <Button appearance="secondary" size="small" type="button" disabled={renaming} onClick={cancelRename}>Cancelar</Button>
            </form>
          : <Button appearance="secondary" size="small" disabled={renamingId !== null} onClick={(event) => beginRename(item, event.currentTarget)}>Renomear</Button>}
      </li>)}
    </ul>}
  </section>;
}
