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

  // Editor.svelte は codemirror-lang-typst → typst-syntax の WASM を読み込む。
  // SvelteKit の hydration 中に top-level await の解決順序が噛み合わず TDZ を踏むため、
  // onMount で動的 import して hydration 完了後にロードする。
  let Editor = $state<Component<{
    value: string;
    onChange?: (next: string) => void;
  }> | null>(null);

  type FileDoc = { path: string; content: string };

  const FILTERS = [{ name: "Typst", extensions: ["typ"] }];

  let path = $state<string | null>(null);
  let content = $state("");
  let dirty = $state(false);
  let status = $state("");

  async function openFile() {
    try {
      const selected = await openDialog({ multiple: false, filters: FILTERS });
      if (typeof selected !== "string") return;
      const doc = await invoke<FileDoc>("open_file", { path: selected });
      path = doc.path;
      content = doc.content;
      dirty = false;
      status = "";
    } catch (e) {
      status = String(e);
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
      path = selected;
      dirty = false;
      status = "";
    } catch (e) {
      status = String(e);
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
      status = "";
    } catch (e) {
      status = String(e);
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
    }
  }

  onMount(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      const win = getCurrentWindow();
      unlisten = await win.onCloseRequested(async (event) => {
        if (!dirty) return;
        // preventDefault は同期で呼ぶ必要があるため、await より前に必ず呼ぶ
        event.preventDefault();
        const ok = await ask("未保存の変更があります。終了してよろしいですか?", {
          title: "右筆",
          kind: "warning",
        });
        if (ok) await win.destroy();
      });
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
    <span class="filename">{basename(path)}{dirty ? " *" : ""}</span>
    {#if status}<span class="status">{status}</span>{/if}
  </header>
  {#if Editor}
    <Editor value={content} onChange={onEditorChange} />
  {:else}
    <div class="loading">エディタを読み込み中…</div>
  {/if}
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

  .filename {
    margin-left: 8px;
    font-size: 13px;
    color: #b0b0b0;
  }

  .status {
    margin-left: auto;
    font-size: 12px;
    color: #ff8080;
  }

  .app :global(.cm-host) {
    flex: 1;
    min-height: 0;
  }

  .loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #6a6a6a;
    font-size: 13px;
  }
</style>
