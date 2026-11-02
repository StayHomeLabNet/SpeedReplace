import { useEffect, useMemo, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { useI18n, type TranslationKey, type TranslationParams } from "./i18n";
import {
  DEFAULT_EXCLUDES,
  type FilePreview,
  type OperationManifest,
  type ReplacementRule,
  type ScanRequest,
  type ScanResult,
  newRule,
} from "./types";
import "./App.css";

type Mode = "simple" | "advanced";
type Phase = "idle" | "scanning" | "preview" | "applying" | "complete";
type StatusMessage = {
  key: TranslationKey;
  values?: Partial<Record<"targets" | "files" | "matches" | "conflicts", number>>;
};
type UiError = { key?: TranslationKey; text?: string };

function shortPath(path: string) {
  const parts = path.split("\\").join("/").split("/");
  return parts.length > 4 ? `…/${parts.slice(-4).join("/")}` : path;
}

function App() {
  const { language, setLanguage, locale, locales, t, count } = useI18n();
  const [mode, setMode] = useState<Mode>("simple");
  const [phase, setPhase] = useState<Phase>("idle");
  const [targets, setTargets] = useState<string[]>([]);
  const [rules, setRules] = useState<ReplacementRule[]>([newRule()]);
  const [includes, setIncludes] = useState("");
  const [excludes, setExcludes] = useState(DEFAULT_EXCLUDES.join("\n"));
  const [recursive, setRecursive] = useState(true);
  const [maxMiB, setMaxMiB] = useState(100);
  const [scan, setScan] = useState<ScanResult | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [message, setMessage] = useState<StatusMessage>({ key: "message.initial" });
  const [error, setError] = useState<UiError>({});
  const [history, setHistory] = useState<OperationManifest[]>([]);
  const [showHistory, setShowHistory] = useState(false);
  const [showUnsupportedFiles, setShowUnsupportedFiles] = useState(false);

  function buildRequest(): ScanRequest {
    return {
      targets,
      filters: {
        includes: includes.split(/[,\n]/).map((value) => value.trim()).filter(Boolean),
        excludes: excludes.split(/[,\n]/).map((value) => value.trim()).filter(Boolean),
        maxFileSize: Math.max(1, maxMiB) * 1024 * 1024,
        recursive,
      },
      rules,
    };
  }

  const stale = useMemo(
    () => (scan ? JSON.stringify(scan.request) !== JSON.stringify(buildRequest()) : false),
    [scan, targets, rules, includes, excludes, recursive, maxMiB],
  );

  const unsupportedFileCount = useMemo(
    () => scan?.files.filter((file) => file.error !== null).length ?? 0,
    [scan],
  );

  const visibleFiles = useMemo(
    () => scan?.files.filter((file) => showUnsupportedFiles || file.error === null) ?? [],
    [scan, showUnsupportedFiles],
  );

  useEffect(() => {
    if (!isTauri()) return;
    const unlisten = getCurrentWindow().onDragDropEvent((event) => {
      if (event.payload.type === "drop") addTargets(event.payload.paths);
    });
    return () => { void unlisten.then((stop) => stop()); };
  }, []);

  function renderStatus(status: StatusMessage) {
    const params: TranslationParams = {};
    for (const [name, value] of Object.entries(status.values ?? {})) {
      if (value === undefined) continue;
      const unit = name === "targets" ? "target" : name === "matches" ? "match" : name === "conflicts" ? "conflict" : "file";
      params[name] = count(unit, value);
    }
    return t(status.key, params);
  }

  function addTargets(paths: string[]) {
    setTargets((current) => Array.from(new Set([...current, ...paths])));
    setMessage({ key: "message.targetsAdded", values: { targets: paths.length } });
  }

  async function chooseTargets(kind: "file" | "folder") {
    const result = await open({ directory: kind === "folder", multiple: true });
    if (result) addTargets(Array.isArray(result) ? result : [result]);
  }

  function updateRule(id: string, patch: Partial<ReplacementRule>) {
    setRules((items) => items.map((item) => (item.id === id ? { ...item, ...patch } : item)));
  }

  function moveRule(index: number, direction: -1 | 1) {
    setRules((items) => {
      const next = [...items];
      const destination = index + direction;
      if (destination < 0 || destination >= next.length) return items;
      [next[index], next[destination]] = [next[destination], next[index]];
      return next;
    });
  }

  async function runScan() {
    setError({});
    if (!targets.length) return setError({ key: "error.noTargets" });
    if (!rules.some((rule) => rule.enabled && rule.search.length > 0)) {
      return setError({ key: "error.noSearch" });
    }
    setPhase("scanning");
    setMessage({ key: "message.scanning" });
    try {
      const result = await invoke<ScanResult>("scan_files", { request: buildRequest() });
      setScan(result);
      setSelected(new Set(result.files.filter((file) => file.changed).map((file) => file.path)));
      setPhase("preview");
      setMessage({
        key: "message.previewSummary",
        values: { files: result.summary.changed, matches: result.summary.matches },
      });
    } catch (reason) {
      setError({ text: String(reason) });
      setPhase("idle");
    }
  }

  async function applyChanges() {
    if (!scan || stale || !selected.size) return;
    if (!window.confirm(t("confirm.apply", { files: count("file", selected.size) }))) return;
    setPhase("applying");
    setError({});
    setMessage({ key: "message.applying" });
    try {
      const operation = await invoke<OperationManifest>("apply_changes", {
        request: { scan, selectedPaths: Array.from(selected) },
      });
      const changed = operation.files.filter((file) => file.status === "changed").length;
      const conflicts = operation.files.filter((file) => file.status === "conflict").length;
      setMessage({
        key: conflicts ? "message.appliedWithConflicts" : "message.applied",
        values: { files: changed, conflicts },
      });
      setPhase("complete");
      setScan(null);
      setSelected(new Set());
      await loadHistory();
    } catch (reason) {
      setError({ text: String(reason) });
      setPhase("preview");
    }
  }

  async function loadHistory() {
    try {
      setHistory(await invoke<OperationManifest[]>("list_operations"));
    } catch (reason) {
      setError({ text: String(reason) });
    }
  }

  async function openHistory() {
    await loadHistory();
    setShowHistory(true);
  }

  async function undoOperation(operation: OperationManifest) {
    setError({});
    try {
      const preview = await invoke<{ safe: string[]; conflicts: string[]; missing: string[] }>("get_undo_preview", {
        operationId: operation.id,
      });
      const detail = t("confirm.undoSafe", { files: count("file", preview.safe.length) })
        + (preview.conflicts.length
          ? t("confirm.undoConflicts", { conflicts: count("conflict", preview.conflicts.length) })
          : "");
      if (!window.confirm(`${detail}\n${t("confirm.continue")}`)) return;
      await invoke("undo_operation", {
        request: { operationId: operation.id, selectedPaths: preview.safe, overwriteConflicts: false },
      });
      setMessage({ key: "message.undone", values: { files: preview.safe.length } });
      await loadHistory();
    } catch (reason) {
      setError({ text: String(reason) });
    }
  }

  function toggleSelected(path: string) {
    setSelected((current) => {
      const next = new Set(current);
      next.has(path) ? next.delete(path) : next.add(path);
      return next;
    });
  }

  function toggleExpanded(path: string) {
    setExpanded((current) => {
      const next = new Set(current);
      next.has(path) ? next.delete(path) : next.add(path);
      return next;
    });
  }

  const displayedError = error.key ? t(error.key) : error.text;

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark">SR</span>
          <div><h1>SpeedReplace</h1><p>{t("brand.tagline")}</p></div>
        </div>
        <div className="top-actions">
          <label className="language-picker">
            <span>{t("language.label")}</span>
            <select value={language} onChange={(event) => setLanguage(event.target.value)}>
              {locales.map((item) => <option key={item.code} value={item.code}>{item.name}</option>)}
            </select>
          </label>
          <div className="mode-switch" aria-label={t("mode.label")}>
            <button className={mode === "simple" ? "active" : ""} onClick={() => setMode("simple")}>{t("mode.simple")}</button>
            <button className={mode === "advanced" ? "active" : ""} onClick={() => setMode("advanced")}>{t("mode.advanced")}</button>
          </div>
          <button className="ghost" onClick={openHistory}>{t("history.open")}</button>
        </div>
      </header>
      <main>
        <section className="panel">
          <div className="section-heading">
            <div><span className="step">1</span><h2>{t("targets.title")}</h2></div>
            <span className="muted">{t("targets.hint")}</span>
          </div>
          <div className="target-actions">
            <button onClick={() => chooseTargets("file")}>{t("targets.addFile")}</button>
            <button onClick={() => chooseTargets("folder")}>{t("targets.addFolder")}</button>
          </div>
          {targets.length ? (
            <div className="chips">
              {targets.map((target) => (
                <span className="chip" key={target} title={target}>
                  {shortPath(target)}
                  <button aria-label={t("targets.remove")} onClick={() => setTargets(targets.filter((item) => item !== target))}>×</button>
                </span>
              ))}
            </div>
          ) : <div className="drop-zone">{t("targets.drop")}</div>}
          {mode === "advanced" && (
            <div className="advanced-grid">
              <label>{t("targets.includes")}<textarea value={includes} onChange={(event) => setIncludes(event.target.value)} placeholder={"**/*.txt\n**/*.md"} /></label>
              <label>{t("targets.excludes")}<textarea value={excludes} onChange={(event) => setExcludes(event.target.value)} /></label>
              <label className="inline"><input type="checkbox" checked={recursive} onChange={(event) => setRecursive(event.target.checked)} />{t("targets.recursive")}</label>
              <label className="inline">{t("targets.maxSize")} <input className="number" type="number" min={1} value={maxMiB} onChange={(event) => setMaxMiB(Number(event.target.value))} /> MiB</label>
            </div>
          )}
        </section>
        <section className="panel">
          <div className="section-heading">
            <div><span className="step">2</span><h2>{t("rules.title")}</h2></div>
            {mode === "advanced" && <button className="ghost" onClick={() => setRules([...rules, newRule()])}>{t("rules.add")}</button>}
          </div>
          <div className="rules">
            {rules.map((rule, index) => (
              <div className="rule" key={rule.id}>
                {mode === "advanced" && <input aria-label={t("rules.enable")} type="checkbox" checked={rule.enabled} onChange={(event) => updateRule(rule.id, { enabled: event.target.checked })} />}
                <div className="rule-body">
                  <div className="rule-fields">
                    <textarea aria-label={t("rules.searchLabel")} value={rule.search} onChange={(event) => updateRule(rule.id, { search: event.target.value })} placeholder={t("rules.searchPlaceholder")} />
                    <span className="arrow">→</span>
                    <textarea aria-label={t("rules.replacementLabel")} value={rule.replacement} onChange={(event) => updateRule(rule.id, { replacement: event.target.value })} placeholder={t("rules.replacementPlaceholder")} />
                  </div>
                  {mode === "advanced" && (
                    <div className="rule-options">
                      <select value={rule.mode} onChange={(event) => updateRule(rule.id, { mode: event.target.value as "literal" | "regex" })}>
                        <option value="literal">{t("rules.literal")}</option>
                        <option value="regex">{t("rules.regex")}</option>
                      </select>
                      <label><input type="checkbox" checked={rule.caseSensitive} onChange={(event) => updateRule(rule.id, { caseSensitive: event.target.checked })} />{t("rules.caseSensitive")}</label>
                      <label><input type="checkbox" checked={rule.wholeWord} disabled={rule.mode === "regex"} onChange={(event) => updateRule(rule.id, { wholeWord: event.target.checked })} />{t("rules.wholeWord")}</label>
                      <button className="icon" aria-label={t("rules.moveUp")} title={t("rules.moveUp")} disabled={index === 0} onClick={() => moveRule(index, -1)}>↑</button>
                      <button className="icon" aria-label={t("rules.moveDown")} title={t("rules.moveDown")} disabled={index === rules.length - 1} onClick={() => moveRule(index, 1)}>↓</button>
                      {rules.length > 1 && <button className="icon danger" aria-label={t("rules.remove")} title={t("rules.remove")} onClick={() => setRules(rules.filter((item) => item.id !== rule.id))}>×</button>}
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        </section>
        <section className="panel preview-panel">
          <div className="section-heading">
            <div><span className="step">3</span><h2>{t("preview.title")}</h2></div>
            <div className="preview-heading-actions">
              <label className="show-unsupported">
                <input
                  type="checkbox"
                  checked={showUnsupportedFiles}
                  onChange={(event) => setShowUnsupportedFiles(event.target.checked)}
                />
                {t("preview.showUnsupported", { count: unsupportedFileCount })}
              </label>
              {scan && <span className="summary">{count("file", scan.summary.changed)} · {count("match", scan.summary.matches)}</span>}
            </div>
          </div>
          {stale && <div className="notice warning">{t("preview.stale")}</div>}
          {displayedError && <div className="notice error" role="alert">{displayedError}</div>}
          {!scan ? (
            <div className="empty-preview"><span>{t("preview.empty")}</span></div>
          ) : !visibleFiles.length ? (
            <div className="empty-preview"><span>{t("preview.noSupportedFiles")}</span></div>
          ) : (
            <div className="file-list">
              {visibleFiles.map((file: FilePreview) => (
                <article className={`file-card ${file.error ? "has-error" : ""}`} key={file.path}>
                  <div className="file-row">
                    <input type="checkbox" aria-label={t("preview.changeTarget")} disabled={!file.changed} checked={selected.has(file.path)} onChange={() => toggleSelected(file.path)} />
                    <button className="file-name" onClick={() => toggleExpanded(file.path)} title={file.path}>{file.changed ? (expanded.has(file.path) ? "▾" : "▸") : "·"} {shortPath(file.path)}</button>
                    {file.encoding && <span className="badge">{file.encoding.name}{file.encoding.hasBom ? " BOM" : ""}</span>}
                    <span className="badge">{file.lineEndings}</span>
                    <span className="matches">{count("match", file.matchCount)}</span>
                  </div>
                  {file.error && <div className="file-error">{file.error}</div>}
                  {expanded.has(file.path) && file.diff && <pre className="diff">{file.diff}</pre>}
                </article>
              ))}
            </div>
          )}
          <div className="actionbar">
            <div><strong>{renderStatus(message)}</strong><small>{t("preview.preserve")}</small></div>
            <button className="secondary" disabled={phase === "scanning" || phase === "applying"} onClick={runScan}>
              {phase === "scanning" ? t("preview.scanning") : scan && stale ? t("preview.rescan") : t("preview.scan")}
            </button>
            <button className="primary" disabled={!scan || stale || selected.size === 0 || phase === "applying"} onClick={applyChanges}>
              {phase === "applying" ? t("preview.applying") : t("preview.apply", { files: count("file", selected.size) })}
            </button>
          </div>
        </section>
      </main>
      {showHistory && (
        <div className="modal-backdrop" onMouseDown={() => setShowHistory(false)}>
          <section className="modal" onMouseDown={(event) => event.stopPropagation()}>
            <div className="section-heading">
              <div><h2>{t("history.title")}</h2></div>
              <button className="icon" aria-label={t("history.close")} onClick={() => setShowHistory(false)}>×</button>
            </div>
            {!history.length ? <p className="muted">{t("history.empty")}</p> : history.map((operation) => (
              <div className="history-row" key={operation.id}>
                <div>
                  <strong>{new Date(operation.createdAt).toLocaleString(locale.code)}</strong>
                  <small>{count("file", operation.files.filter((file) => file.status === "changed").length)}</small>
                </div>
                <button onClick={() => undoOperation(operation)}>{t("history.undo")}</button>
              </div>
            ))}
          </section>
        </div>
      )}
    </div>
  );
}

export default App;
