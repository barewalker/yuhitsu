<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import {
    ask,
    open as openDialog,
    save as saveDialog,
  } from "@tauri-apps/plugin-dialog";
  import type { Component } from "svelte";
  import {
    type EditorMode,
    loadSettings,
    saveEditorMode,
  } from "$lib/settings";

  // Editor.svelte は codemirror-lang-typst → typst-syntax の WASM を読み込む。
  // SvelteKit の hydration 中に top-level await の解決順序が噛み合わず TDZ を踏むため、
  // onMount で動的 import して hydration 完了後にロードする。
  let Editor = $state<Component<{
    value: string;
    mode?: EditorMode;
    onChange?: (next: string) => void;
  }> | null>(null);

  let editorMode = $state<EditorMode>("default");

  const EDITOR_MODE_LABELS: Record<EditorMode, string> = {
    default: "標準",
    vim: "vim",
    emacs: "emacs",
  };

  async function changeEditorMode(next: EditorMode) {
    editorMode = next;
    try {
      await saveEditorMode(next);
    } catch (e) {
      // 永続化失敗は致命的でないので info で控えめに通知
      setStatus(`設定保存に失敗: ${String(e)}`, "error");
    }
  }

  type FileDoc = { path: string; content: string };

  const FILTERS = [{ name: "Typst", extensions: ["typ"] }];
  const PDF_FILTERS = [{ name: "PDF", extensions: ["pdf"] }];
  const PREVIEW_URL = "http://127.0.0.1:23625/";
  // tinymist preview の HTTP サーバが立ち上がるまでの待機時間(暫定)。
  // 後で HTTP プローブに置き換える前提。
  const PREVIEW_BOOT_DELAY_MS = 1500;

  let path = $state<string | null>(null);
  let content = $state("");
  let dirty = $state(false);
  let status = $state("");
  let statusKind = $state<"info" | "error">("error");

  function setStatus(message: string, kind: "info" | "error" = "error") {
    status = message;
    statusKind = kind;
  }

  function clearStatus() {
    status = "";
    statusKind = "error";
  }

  type PreviewStatus = "idle" | "starting" | "ready" | "error";
  let previewStatus = $state<PreviewStatus>("idle");
  let previewError = $state("");
  // iframe の src を都度切り替えるとキャッシュ衝突の懸念があるため、
  // 起動完了タイミングで cache-buster を載せて確実に再ロードさせる
  let previewSrc = $state<string | null>(null);

  async function startPreview(forPath: string) {
    previewStatus = "starting";
    previewError = "";
    previewSrc = null;
    try {
      await invoke("start_preview", { path: forPath });
    } catch (e) {
      previewStatus = "error";
      previewError = String(e);
      return;
    }
    // 起動完了を待つ。本来は HTTP プローブで判定するべきだが、暫定で固定 wait。
    await new Promise((resolve) => setTimeout(resolve, PREVIEW_BOOT_DELAY_MS));
    previewSrc = `${PREVIEW_URL}?t=${Date.now()}`;
    previewStatus = "ready";
  }

  async function stopPreview() {
    try {
      await invoke("stop_preview");
    } catch (e) {
      // 終了系は失敗しても致命的でないので status だけ更新
      previewError = String(e);
    }
    previewStatus = "idle";
    previewSrc = null;
  }

  async function openFile() {
    try {
      const selected = await openDialog({ multiple: false, filters: FILTERS });
      if (typeof selected !== "string") return;
      const doc = await invoke<FileDoc>("open_file", { path: selected });
      path = doc.path;
      content = doc.content;
      dirty = false;
      clearStatus();
      await startPreview(doc.path);
    } catch (e) {
      setStatus(String(e));
    }
  }

  async function saveAs() {
    try {
      const selected = await saveDialog({
        filters: FILTERS,
        defaultPath: path ?? "untitled.typ",
      });
      if (!selected) return;
      await invoke("save_file", { path: selected, content });
      const isNewPath = path !== selected;
      path = selected;
      dirty = false;
      clearStatus();
      if (isNewPath) {
        await startPreview(selected);
      }
    } catch (e) {
      setStatus(String(e));
    }
  }

  async function save() {
    if (!path) {
      await saveAs();
      return;
    }
    try {
      await invoke("save_file", { path, content });
      dirty = false;
      clearStatus();
      // tinymist がファイル変更を watch しているので start_preview 再起動は不要
    } catch (e) {
      setStatus(String(e));
    }
  }

  // 入力 .typ ファイルパスから既定の出力 .pdf パスを組み立てる
  function defaultPdfPath(typPath: string): string {
    return typPath.replace(/\.typ$/i, ".pdf");
  }

  async function exportPdf() {
    if (!path) {
      setStatus("PDF を書き出す前に、ファイルを保存してください。");
      return;
    }
    // 編集中の内容を反映させるため、未保存があれば自動で保存する。
    // ユーザにそのことを伝えたいので、自動保存の有無を覚えておく。
    let savedAutomatically = false;
    if (dirty) {
      try {
        await invoke("save_file", { path, content });
        dirty = false;
        savedAutomatically = true;
      } catch (e) {
        setStatus(String(e));
        return;
      }
    }
    let outputPath: string;
    try {
      const selected = await saveDialog({
        filters: PDF_FILTERS,
        defaultPath: defaultPdfPath(path),
      });
      if (!selected) return;
      outputPath = selected;
    } catch (e) {
      setStatus(String(e));
      return;
    }
    setStatus("PDF を書き出し中…", "info");
    try {
      await invoke("export_pdf", { input: path, output: outputPath });
      const prefix = savedAutomatically ? "保存してから " : "";
      setStatus(`${prefix}PDF を書き出しました: ${outputPath}`, "info");
    } catch (e) {
      setStatus(String(e));
    }
  }

  function onEditorChange(next: string) {
    content = next;
    dirty = true;
  }

  function basename(p: string | null): string {
    if (!p) return "(無題)";
    const i = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
    return i >= 0 ? p.slice(i + 1) : p;
  }

  function onKeydown(e: KeyboardEvent) {
    const meta = e.ctrlKey || e.metaKey;
    if (!meta) return;
    const key = e.key.toLowerCase();
    if (key === "o" && !e.shiftKey) {
      e.preventDefault();
      openFile();
    } else if (key === "s" && !e.shiftKey) {
      e.preventDefault();
      save();
    } else if (key === "s" && e.shiftKey) {
      e.preventDefault();
      saveAs();
    } else if (key === "e" && !e.shiftKey) {
      e.preventDefault();
      exportPdf();
    }
  }

  onMount(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      const win = getCurrentWindow();
      unlisten = await win.onCloseRequested(async (event) => {
        if (!dirty) {
          // dirty でなくても preview の subprocess は止めたい
          await stopPreview();
          return;
        }
        // preventDefault は同期で呼ぶ必要があるため、await より前に必ず呼ぶ
        event.preventDefault();
        const ok = await ask("未保存の変更があります。終了してよろしいですか?", {
          title: "右筆",
          kind: "warning",
        });
        if (ok) {
          await stopPreview();
          await win.destroy();
        }
      });
      // 設定の読み込みは hydration 後に。Tauri Store は async API なので
      // onMount で起動時に1回読む。失敗時はデフォルトを使う。
      try {
        const settings = await loadSettings();
        editorMode = settings.editor.mode;
      } catch (e) {
        console.warn("settings load failed, using defaults:", e);
      }
      const mod = await import("$lib/Editor.svelte");
      Editor = mod.default;
    })();
    return () => unlisten?.();
  });
</script>

<svelte:window onkeydown={onKeydown} />

<div class="app">
  <header class="toolbar">
    <button onclick={openFile}>開く</button>
    <button onclick={save}>保存</button>
    <button onclick={saveAs}>名前を付けて保存</button>
    <button onclick={exportPdf} disabled={!path}>PDF 出力</button>
    <label class="mode-select">
      操作モード:
      <select
        value={editorMode}
        onchange={(e) =>
          changeEditorMode((e.currentTarget as HTMLSelectElement).value as EditorMode)}
      >
        {#each Object.entries(EDITOR_MODE_LABELS) as [value, label]}
          <option {value}>{label}</option>
        {/each}
      </select>
    </label>
    <span class="filename">{basename(path)}{dirty ? " *" : ""}</span>
    {#if status}
      <span class="status status-{statusKind}">{status}</span>
    {/if}
  </header>

  <div class="workspace">
    <div class="editor-pane">
      {#if Editor}
        <Editor value={content} mode={editorMode} onChange={onEditorChange} />
      {:else}
        <div class="placeholder">エディタを読み込み中…</div>
      {/if}
    </div>

    <div class="preview-pane">
      {#if previewStatus === "idle"}
        <div class="placeholder">
          ファイルを開く、または保存するとプレビューが表示されます。
        </div>
      {:else if previewStatus === "starting"}
        <div class="placeholder">プレビューを起動中…</div>
      {:else if previewStatus === "error"}
        <div class="placeholder error">
          プレビューの起動に失敗しました。
          <br />
          <small>{previewError}</small>
        </div>
      {:else if previewStatus === "ready" && previewSrc}
        <iframe class="preview-frame" title="プレビュー" src={previewSrc}
        ></iframe>
      {/if}
    </div>
  </div>
</div>

<style>
  :global(:root) {
    color-scheme: dark;
  }

  :global(html, body) {
    margin: 0;
    height: 100%;
    font-family: system-ui, sans-serif;
    background: #1e1e1e;
    color: #e6e6e6;
  }

  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: #2a2a2a;
    border-bottom: 1px solid #3a3a3a;
  }

  .toolbar button {
    padding: 4px 10px;
    background: #3a3a3a;
    color: #e6e6e6;
    border: 1px solid #4a4a4a;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
  }

  .toolbar button:hover {
    background: #4a4a4a;
  }

  .toolbar button:disabled {
    color: #6a6a6a;
    background: #2f2f2f;
    border-color: #3a3a3a;
    cursor: not-allowed;
  }

  .toolbar button:disabled:hover {
    background: #2f2f2f;
  }

  .mode-select {
    margin-left: 8px;
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: #b0b0b0;
  }

  .mode-select select {
    background: #3a3a3a;
    color: #e6e6e6;
    border: 1px solid #4a4a4a;
    border-radius: 4px;
    padding: 3px 6px;
    font-size: 12px;
    cursor: pointer;
  }

  .filename {
    margin-left: 8px;
    font-size: 13px;
    color: #b0b0b0;
  }

  .status {
    margin-left: auto;
    font-size: 12px;
    max-width: 60%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-error {
    color: #ff8080;
  }

  .status-info {
    color: #98c379;
  }

  .workspace {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: row;
  }

  .editor-pane,
  .preview-pane {
    flex: 1 1 50%;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .editor-pane {
    border-right: 1px solid #3a3a3a;
  }

  .preview-pane {
    background: #2a2a2a;
  }

  .preview-frame {
    flex: 1;
    width: 100%;
    border: none;
    background: white;
  }

  .editor-pane :global(.cm-host) {
    flex: 1;
    min-height: 0;
  }

  .placeholder {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #6a6a6a;
    font-size: 13px;
    padding: 16px;
    text-align: center;
  }

  .placeholder.error {
    color: #ff8080;
  }

  .placeholder small {
    display: block;
    margin-top: 8px;
    color: #888;
    font-size: 11px;
    word-break: break-all;
  }
</style>
