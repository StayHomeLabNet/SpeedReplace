import { useEffect, useMemo, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { DEFAULT_EXCLUDES, type FilePreview, type OperationManifest, type ReplacementRule, type ScanRequest, type ScanResult, newRule } from "./types";
import "./App.css";

type Mode = "simple" | "advanced";
type Phase = "idle" | "scanning" | "preview" | "applying" | "complete";

function shortPath(path: string) {
  const parts = path.split("\\").join("/").split("/");
  return parts.length > 4 ? `…/${parts.slice(-4).join("/")}` : path;
}

function App() {
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
  const [message, setMessage] = useState("ファイルまたはフォルダーを追加してください");
  const [error, setError] = useState("");
  const [history, setHistory] = useState<OperationManifest[]>([]);
  const [showHistory, setShowHistory] = useState(false);

  function buildRequest(): ScanRequest {
    return {
      targets,
      filters: {
        includes: includes.split(/[,\n]/).map((v) => v.trim()).filter(Boolean),
        excludes: excludes.split(/[,\n]/).map((v) => v.trim()).filter(Boolean),
        maxFileSize: Math.max(1, maxMiB) * 1024 * 1024,
        recursive,
      },
      rules,
    };
  }

  const stale = useMemo(() => scan ? JSON.stringify(scan.request) !== JSON.stringify(buildRequest()) : false,
    [scan, targets, rules, includes, excludes, recursive, maxMiB]);

  useEffect(() => {
    if (!isTauri()) return;
    const unlisten = getCurrentWindow().onDragDropEvent((event) => {
      if (event.payload.type === "drop") addTargets(event.payload.paths);
    });
    return () => { void unlisten.then((fn) => fn()); };
  }, []);

  function addTargets(paths: string[]) {
    setTargets((current) => Array.from(new Set([...current, ...paths])));
    setMessage(`${paths.length}件の対象を追加しました`);
  }

  async function chooseTargets(kind: "file" | "folder") {
    const result = await open({ directory: kind === "folder", multiple: true });
    if (result) addTargets(Array.isArray(result) ? result : [result]);
  }

  function updateRule(id: string, patch: Partial<ReplacementRule>) {
    setRules((items) => items.map((item) => item.id === id ? { ...item, ...patch } : item));
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
    setError("");
    if (!targets.length) return setError("対象ファイルまたはフォルダーを追加してください");
    if (!rules.some((rule) => rule.enabled && rule.search.length > 0)) return setError("有効な検索文字列を入力してください");
    setPhase("scanning");
    setMessage("安全なプレビューを作成しています…");
    try {
      const result = await invoke<ScanResult>("scan_files", { request: buildRequest() });
      setScan(result);
      setSelected(new Set(result.files.filter((file) => file.changed).map((file) => file.path)));
      setPhase("preview");
      setMessage(`${result.summary.changed}ファイル、${result.summary.matches}箇所の変更候補`);
    } catch (reason) {
      setError(String(reason)); setPhase("idle");
    }
  }

  async function applyChanges() {
    if (!scan || stale || !selected.size) return;
    if (!window.confirm(`${selected.size}ファイルを変更します。検証済みバックアップを先に作成します。`)) return;
    setPhase("applying"); setError(""); setMessage("バックアップを作成して変更を適用しています…");
    try {
      const operation = await invoke<OperationManifest>("apply_changes", { request: { scan, selectedPaths: Array.from(selected) } });
      const changed = operation.files.filter((file) => file.status === "changed").length;
      const conflicts = operation.files.filter((file) => file.status === "conflict").length;
      setMessage(`${changed}ファイルを変更しました${conflicts ? `（競合 ${conflicts}件）` : ""}`);
      setPhase("complete"); setScan(null); setSelected(new Set()); await loadHistory();
    } catch (reason) {
      setError(String(reason)); setPhase("preview");
    }
  }

  async function loadHistory() {
    try { setHistory(await invoke<OperationManifest[]>("list_operations")); }
    catch (reason) { setError(String(reason)); }
  }

  async function openHistory() { await loadHistory(); setShowHistory(true); }

  async function undoOperation(operation: OperationManifest) {
    setError("");
    try {
      const preview = await invoke<{ safe: string[]; conflicts: string[]; missing: string[] }>("get_undo_preview", { operationId: operation.id });
      const detail = `${preview.safe.length}件を安全に復元できます。` + (preview.conflicts.length ? ` 外部変更との競合 ${preview.conflicts.length}件は復元しません。` : "");
      if (!window.confirm(`${detail}\n続行しますか？`)) return;
      await invoke("undo_operation", { request: { operationId: operation.id, selectedPaths: preview.safe, overwriteConflicts: false } });
      setMessage(`${preview.safe.length}ファイルを元に戻しました`); await loadHistory();
    } catch (reason) { setError(String(reason)); }
  }

  function toggleSelected(path: string) {
    setSelected((current) => { const next = new Set(current); next.has(path) ? next.delete(path) : next.add(path); return next; });
  }

  function toggleExpanded(path: string) {
    setExpanded((current) => { const next = new Set(current); next.has(path) ? next.delete(path) : next.add(path); return next; });
  }

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand"><span className="brand-mark">SR</span><div><h1>SpeedReplace</h1><p>高速で、安全な一括置換</p></div></div>
        <div className="top-actions"><div className="mode-switch" aria-label="表示モード"><button className={mode === "simple" ? "active" : ""} onClick={() => setMode("simple")}>シンプル</button><button className={mode === "advanced" ? "active" : ""} onClick={() => setMode("advanced")}>詳細</button></div><button className="ghost" onClick={openHistory}>履歴・Undo</button></div>
      </header>
      <main>
        <section className="panel"><div className="section-heading"><div><span className="step">1</span><h2>対象を選択</h2></div><span className="muted">ドラッグ＆ドロップにも対応</span></div>
          <div className="target-actions"><button onClick={() => chooseTargets("file")}>＋ ファイルを追加</button><button onClick={() => chooseTargets("folder")}>＋ フォルダーを追加</button></div>
          {targets.length ? <div className="chips">{targets.map((target) => <span className="chip" key={target} title={target}>{shortPath(target)}<button aria-label="対象から削除" onClick={() => setTargets(targets.filter((item) => item !== target))}>×</button></span>)}</div> : <div className="drop-zone">ここにファイルまたはフォルダーをドロップ</div>}
          {mode === "advanced" && <div className="advanced-grid"><label>対象パターン<textarea value={includes} onChange={(e) => setIncludes(e.target.value)} placeholder="**/*.txt&#10;**/*.md" /></label><label>除外パターン<textarea value={excludes} onChange={(e) => setExcludes(e.target.value)} /></label><label className="inline"><input type="checkbox" checked={recursive} onChange={(e) => setRecursive(e.target.checked)} />サブフォルダーを含む</label><label className="inline">最大サイズ <input className="number" type="number" min={1} value={maxMiB} onChange={(e) => setMaxMiB(Number(e.target.value))} /> MiB</label></div>}
        </section>
        <section className="panel"><div className="section-heading"><div><span className="step">2</span><h2>置換ルール</h2></div>{mode === "advanced" && <button className="ghost" onClick={() => setRules([...rules, newRule()])}>＋ ルール追加</button>}</div>
          <div className="rules">{rules.map((rule, index) => <div className="rule" key={rule.id}>{mode === "advanced" && <input aria-label="ルールを有効化" type="checkbox" checked={rule.enabled} onChange={(e) => updateRule(rule.id, { enabled: e.target.checked })} />}<div className="rule-body"><div className="rule-fields"><textarea aria-label="検索文字列" value={rule.search} onChange={(e) => updateRule(rule.id, { search: e.target.value })} placeholder="検索する文字列" /><span className="arrow">→</span><textarea aria-label="置換文字列" value={rule.replacement} onChange={(e) => updateRule(rule.id, { replacement: e.target.value })} placeholder="置換後の文字列" /></div>{mode === "advanced" && <div className="rule-options"><select value={rule.mode} onChange={(e) => updateRule(rule.id, { mode: e.target.value as "literal" | "regex" })}><option value="literal">通常</option><option value="regex">正規表現</option></select><label><input type="checkbox" checked={rule.caseSensitive} onChange={(e) => updateRule(rule.id, { caseSensitive: e.target.checked })} />大文字小文字を区別</label><label><input type="checkbox" checked={rule.wholeWord} disabled={rule.mode === "regex"} onChange={(e) => updateRule(rule.id, { wholeWord: e.target.checked })} />単語単位</label><button className="icon" disabled={index === 0} onClick={() => moveRule(index, -1)}>↑</button><button className="icon" disabled={index === rules.length - 1} onClick={() => moveRule(index, 1)}>↓</button>{rules.length > 1 && <button className="icon danger" onClick={() => setRules(rules.filter((item) => item.id !== rule.id))}>×</button>}</div>}</div></div>)}</div>
        </section>
        <section className="panel preview-panel"><div className="section-heading"><div><span className="step">3</span><h2>プレビューと実行</h2></div>{scan && <span className="summary">{scan.summary.changed}ファイル・{scan.summary.matches}件</span>}</div>
          {stale && <div className="notice warning">条件が変更されました。安全のためプレビューを再作成してください。</div>}{error && <div className="notice error" role="alert">{error}</div>}
          {!scan ? <div className="empty-preview"><span>差分プレビューはここに表示されます</span></div> : <div className="file-list">{scan.files.map((file: FilePreview) => <article className={`file-card ${file.error ? "has-error" : ""}`} key={file.path}><div className="file-row"><input type="checkbox" aria-label="変更対象" disabled={!file.changed} checked={selected.has(file.path)} onChange={() => toggleSelected(file.path)} /><button className="file-name" onClick={() => toggleExpanded(file.path)} title={file.path}>{file.changed ? (expanded.has(file.path) ? "▾" : "▸") : "·"} {shortPath(file.path)}</button>{file.encoding && <span className="badge">{file.encoding.name}{file.encoding.hasBom ? " BOM" : ""}</span>}<span className="badge">{file.lineEndings}</span><span className="matches">{file.matchCount}件</span></div>{file.error && <div className="file-error">{file.error}</div>}{expanded.has(file.path) && file.diff && <pre className="diff">{file.diff}</pre>}</article>)}</div>}
          <div className="actionbar"><div><strong>{message}</strong><small>元の文字コード・BOM・改行コードを維持します</small></div><button className="secondary" disabled={phase === "scanning" || phase === "applying"} onClick={runScan}>{phase === "scanning" ? "スキャン中…" : scan && stale ? "再プレビュー" : "検索・プレビュー"}</button><button className="primary" disabled={!scan || stale || selected.size === 0 || phase === "applying"} onClick={applyChanges}>{phase === "applying" ? "適用中…" : `${selected.size}ファイルを置換`}</button></div>
        </section>
      </main>
      {showHistory && <div className="modal-backdrop" onMouseDown={() => setShowHistory(false)}><section className="modal" onMouseDown={(e) => e.stopPropagation()}><div className="section-heading"><div><h2>置換履歴とUndo</h2></div><button className="icon" onClick={() => setShowHistory(false)}>×</button></div>{!history.length ? <p className="muted">履歴はまだありません。</p> : history.map((operation) => <div className="history-row" key={operation.id}><div><strong>{new Date(operation.createdAt).toLocaleString()}</strong><small>{operation.files.filter((file) => file.status === "changed").length}ファイル</small></div><button onClick={() => undoOperation(operation)}>安全に元に戻す</button></div>)}</section></div>}
    </div>
  );
}

export default App;
