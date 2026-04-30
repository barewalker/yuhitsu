<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import {
    ask,
    open as openDialog,
    save as saveDialog,
  } from "@tauri-apps/plugin-dialog";
  import { dirname } from "@tauri-apps/api/path";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { dlog } from "$lib/dev-log";
  import type { Component } from "svelte";
  import {
    type EditorMode,
    type KeybindingsSettings,
    loadSettings,
    saveEditorMode,
    saveWorkspace,
  } from "$lib/settings";
  import type { LSPClient } from "@codemirror/lsp-client";
  import { pathToFileUri, startLspSession, type LspSession } from "$lib/lsp";
  import type { EditorView } from "@codemirror/view";
  import { insertImage } from "$lib/editor-commands";
  import { listDirectory, type DirEntry } from "$lib/project";
  import ProjectTree from "$lib/ProjectTree.svelte";
  import {
    COMMANDS,
    COMMAND_IDS,
    getDefaultToolbarItems,
    type CommandContext,
    type CommandId,
    type ToolbarItem,
  } from "$lib/commands";

  // Editor.svelte は codemirror-lang-typst → typst-syntax の WASM を読み込む。
  // SvelteKit の hydration 中に top-level await の解決順序が噛み合わず TDZ を踏むため、
  // onMount で動的 import して hydration 完了後にロードする。
  let Editor = $state<Component<{
    value: string;
    mode?: EditorMode;
    languageMode?: "typst" | "plain";
    lspClient?: LSPClient | null;
    filePath?: string | null;
    onChange?: (next: string) => void;
    onReady?: (view: EditorView) => void;
    onTeardown?: () => void;
    onValueApplied?: (view: EditorView) => void;
  }> | null>(null);

  // Editor.svelte が onReady で渡してくる EditorView。GUI 挿入ボタンや
  // ショートカットからのコマンド呼び出しに使う。
  let editorView = $state<EditorView | null>(null);

  let lspSession = $state<LspSession | null>(null);
  let lspClient = $derived(lspSession?.client ?? null);

  let editorMode = $state<EditorMode>("default");

  // ツールバーの並び・キーバインドは設定で書き換え可能。
  // 設定読み込み完了までは getDefaultToolbarItems() を表示する。
  let toolbarItems = $state<ToolbarItem[]>(getDefaultToolbarItems());
  let keybindings = $state<KeybindingsSettings>({});

  // ワークスペース表示。preview / プロジェクトビューの有無 + 境界比率はユーザが操作する。
  let previewVisible = $state(true);
  let editorPaneRatio = $state(0.5);
  let projectViewVisible = $state(false);
  let projectPaneRatio = $state(0.18);
  let workspaceEl = $state<HTMLDivElement | null>(null);
  let editPreviewEl = $state<HTMLDivElement | null>(null);
  let splitterTarget = $state<"project" | "editor" | null>(null);

  // プロジェクト状態。currentFolder は永続化、projectTree は起動時に
  // 読み込む。expanded はセッション内のみ(リロードで初期化)。
  let currentFolder = $state<string | null>(null);
  let projectTree = $state<DirEntry | null>(null);
  let projectExpanded = $state<Record<string, boolean>>({});

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

  // 1 タブ = 1 ファイルの編集セッション。path=null は未保存の新規タブ。
  // カーソル/スクロール位置はタブ切替で復元するため per-tab に保持する。
  type TabId = string;
  type Tab = {
    id: TabId;
    path: string | null;
    content: string;
    dirty: boolean;
    cursorAnchor: number;
    cursorHead: number;
    scrollTop: number;
  };

  let nextTabSeq = 1;
  function newTabId(): TabId {
    return `tab-${nextTabSeq++}`;
  }

  function makeEmptyTab(): Tab {
    return {
      id: newTabId(),
      path: null,
      content: "",
      dirty: false,
      cursorAnchor: 0,
      cursorHead: 0,
      scrollTop: 0,
    };
  }

  function isTypstPath(p: string | null | undefined): boolean {
    return !!p && p.toLowerCase().endsWith(".typ");
  }

  const FILTERS = [{ name: "Typst", extensions: ["typ"] }];
  const PDF_FILTERS = [{ name: "PDF", extensions: ["pdf"] }];
  const IMAGE_EXTS = [
    "png",
    "jpg",
    "jpeg",
    "gif",
    "svg",
    "webp",
    "avif",
  ];
  const IMAGE_FILTERS = [{ name: "画像", extensions: IMAGE_EXTS }];
  const PREVIEW_URL = "http://127.0.0.1:23625/";
  // tinymist preview の HTTP サーバが立ち上がるまでの待機時間(暫定)。
  // 後で HTTP プローブに置き換える前提。
  const PREVIEW_BOOT_DELAY_MS = 1500;

  // タブ配列とアクティブ ID。最初は空タブ 1 枚で起動。
  const initialTab = makeEmptyTab();
  let tabs = $state<Tab[]>([initialTab]);
  let activeTabId = $state<TabId | null>(initialTab.id);

  function getActiveTab(): Tab | null {
    return tabs.find((t) => t.id === activeTabId) ?? null;
  }
  function getTab(id: TabId): Tab | null {
    return tabs.find((t) => t.id === id) ?? null;
  }

  // タブ D&D は pointer events で自前実装。HTML5 Drag and Drop は WebKitGTK
  // 上でドラッグ ghost が他要素の hit testing を妨げ、方向次第で drop が
  // 走らない不具合があるため避ける。
  // - pointerdown で「掴む準備」(クリック判定の保留状態に)
  // - 5px 以上動いた時点でドラッグ開始
  // - pointermove で elementFromPoint からホバー中タブを判定
  // - pointerup でドロップ反映、移動が無ければ通常クリックとして switchTab
  let draggingTabId = $state<TabId | null>(null);
  let dragOverTabId = $state<TabId | null>(null);
  let pendingTabId: TabId | null = null;
  let pendingPointerId: number | null = null;
  let pendingStartX = 0;
  let pendingStartY = 0;
  const DRAG_THRESHOLD_PX = 5;

  function onTabPointerDown(e: PointerEvent, id: TabId) {
    if (e.button !== 0) return;
    // タブのテキストがドラッグで選択されるのを防ぐ。WebKitGTK 上では
    // CSS の user-select: none だけでは抑制されないため pointerdown で
    // 直接 preventDefault する。
    e.preventDefault();
    pendingTabId = id;
    pendingPointerId = e.pointerId;
    pendingStartX = e.clientX;
    pendingStartY = e.clientY;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onTabPointerMove(e: PointerEvent) {
    if (pendingPointerId === null || e.pointerId !== pendingPointerId) return;
    if (!draggingTabId) {
      const dx = e.clientX - pendingStartX;
      const dy = e.clientY - pendingStartY;
      if (Math.hypot(dx, dy) < DRAG_THRESHOLD_PX) return;
      // threshold 超え → ドラッグ開始
      draggingTabId = pendingTabId;
    }
    // ホバー中のタブを判定
    const el = document.elementFromPoint(e.clientX, e.clientY);
    const tabEl = (el as HTMLElement | null)?.closest("[data-tab-id]") as
      | HTMLElement
      | null;
    const id = tabEl?.dataset.tabId as TabId | undefined;
    if (id && id !== draggingTabId) {
      dragOverTabId = id;
    } else {
      dragOverTabId = null;
    }
  }

  function onTabPointerUp(e: PointerEvent, id: TabId) {
    if (pendingPointerId === null || e.pointerId !== pendingPointerId) return;
    const wasDragging = draggingTabId !== null;
    const src = draggingTabId;
    const target = dragOverTabId;
    draggingTabId = null;
    dragOverTabId = null;
    pendingTabId = null;
    pendingPointerId = null;
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      // capture 解除に失敗しても致命的でない
    }
    if (wasDragging) {
      if (src && target && src !== target) {
        reorderTabs(src, target);
      }
    } else {
      // ドラッグ閾値未満 = 通常クリック扱い → タブ切替
      switchTab(id);
    }
  }

  function reorderTabs(srcId: TabId, dstId: TabId) {
    const srcIdx = tabs.findIndex((t) => t.id === srcId);
    const dstIdx = tabs.findIndex((t) => t.id === dstId);
    if (srcIdx < 0 || dstIdx < 0) return;
    const next = [...tabs];
    const [moved] = next.splice(srcIdx, 1);
    next.splice(dstIdx, 0, moved);
    tabs = next;
  }

  // 既存コードに最小の変更で乗るよう、active tab 由来の値を path/content/dirty
  // として derived 公開する。書き換えは getActiveTab() を直接 mutate する。
  let path = $derived<string | null>(getActiveTab()?.path ?? null);
  let content = $derived(getActiveTab()?.content ?? "");
  let dirty = $derived(getActiveTab()?.dirty ?? false);

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

  async function ensureLspFor(forPath: string) {
    // 別ファイルへの切替も含め、既存セッションは止めて新規に張り直す。
    if (lspSession) {
      await lspSession.shutdown();
      lspSession = null;
    }
    try {
      lspSession = await startLspSession(forPath);
    } catch (e) {
      // LSP が起動しなくてもエディタ自体は使えるので、エラー表示のみ
      setStatus(`LSP 起動に失敗: ${String(e)}`);
    }
  }

  async function stopLspSession() {
    if (!lspSession) return;
    try {
      await lspSession.shutdown();
    } catch (e) {
      console.warn("[lsp] shutdown failed:", e);
    }
    lspSession = null;
  }

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
      // .typ に限らずテキスト全般を許容(タブで参照しながら編集する用途)
      const selected = await openDialog({ multiple: false });
      if (typeof selected !== "string") return;
      await openFileAtPath(selected);
    } catch (e) {
      setStatus(String(e));
    }
  }

  async function saveAs() {
    const tab = getActiveTab();
    if (!tab) return;
    try {
      const selected = await saveDialog({
        filters: FILTERS,
        defaultPath: tab.path ?? "untitled.typ",
      });
      if (!selected) return;
      await invoke("save_file", { path: selected, content: tab.content });
      const isNewPath = tab.path !== selected;
      tab.path = selected;
      tab.dirty = false;
      clearStatus();
      if (isNewPath) {
        if (isTypstPath(selected)) {
          await Promise.all([startPreview(selected), ensureLspFor(selected)]);
        } else {
          await Promise.all([stopPreview(), stopLspSession()]);
        }
      }
    } catch (e) {
      setStatus(String(e));
    }
  }

  async function save() {
    const tab = getActiveTab();
    if (!tab) return;
    if (!tab.path) {
      await saveAs();
      return;
    }
    try {
      await invoke("save_file", { path: tab.path, content: tab.content });
      tab.dirty = false;
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
    const tab = getActiveTab();
    if (!tab) return;
    if (!isTypstPath(tab.path)) {
      setStatus("PDF 出力は Typst (.typ) ファイルのみ対応しています。");
      return;
    }
    if (!tab.path) {
      setStatus("PDF を書き出す前に、ファイルを保存してください。");
      return;
    }
    // 編集中の内容を反映させるため、未保存があれば自動で保存する。
    // ユーザにそのことを伝えたいので、自動保存の有無を覚えておく。
    let savedAutomatically = false;
    if (tab.dirty) {
      try {
        await invoke("save_file", { path: tab.path, content: tab.content });
        tab.dirty = false;
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
        defaultPath: defaultPdfPath(tab.path),
      });
      if (!selected) return;
      outputPath = selected;
    } catch (e) {
      setStatus(String(e));
      return;
    }
    setStatus("PDF を書き出し中…", "info");
    try {
      await invoke("export_pdf", { input: tab.path, output: outputPath });
      const prefix = savedAutomatically ? "保存してから " : "";
      setStatus(`${prefix}PDF を書き出しました: ${outputPath}`, "info");
    } catch (e) {
      setStatus(String(e));
    }
  }

  function onEditorChange(next: string) {
    const tab = getActiveTab();
    if (!tab) return;
    // タブ切替時の dispatch でも updateListener が走るが、その時点では
    // tab.content は新値と一致するので no-op で抜ける(dirty フラグを誤らせない)
    if (tab.content === next) return;
    tab.content = next;
    tab.dirty = true;
  }

  function onEditorReady(view: EditorView) {
    editorView = view;
  }

  function onEditorTeardown() {
    editorView = null;
  }

  // 外部由来の value 変更(タブ切替や open)が doc に反映された直後に呼ばれる。
  // 親側でカーソル / スクロール位置を復元する。
  function onEditorValueApplied(view: EditorView) {
    const tab = getActiveTab();
    if (!tab) return;
    const len = view.state.doc.length;
    const anchor = Math.min(tab.cursorAnchor, len);
    const head = Math.min(tab.cursorHead, len);
    view.dispatch({ selection: { anchor, head } });
    view.scrollDOM.scrollTop = tab.scrollTop;
  }

  // タブ切替前に現在 active の状態(カーソル / スクロール)を控える。
  // tab.content は updateListener 経由で常に最新化されているので保存不要。
  function captureActiveTabState() {
    const tab = getActiveTab();
    if (!tab || !editorView) return;
    const sel = editorView.state.selection.main;
    tab.cursorAnchor = sel.anchor;
    tab.cursorHead = sel.head;
    tab.scrollTop = editorView.scrollDOM.scrollTop;
  }


  async function switchTab(targetId: TabId) {
    if (activeTabId === targetId) return;
    captureActiveTabState();
    activeTabId = targetId;
    const tab = getActiveTab();
    if (!tab) return;
    // doc / カーソル / scroll は Editor.svelte 側の $effect が反映する。
    // typ なら preview / LSP、それ以外は停止しエディタだけ使う。
    if (isTypstPath(tab.path) && tab.path) {
      await Promise.all([startPreview(tab.path), ensureLspFor(tab.path)]);
    } else {
      await Promise.all([stopPreview(), stopLspSession()]);
    }
  }

  // 既存タブで開いていればそれを active に、なければ新規タブで開く。
  // ファイル open 系(ダイアログ / ツリー / 起動時)の共通入口。
  async function openFileAtPath(filePath: string) {
    const existing = tabs.find((t) => t.path === filePath);
    if (existing) {
      await switchTab(existing.id);
      return;
    }
    try {
      const doc = await invoke<FileDoc>("open_file", { path: filePath });
      // 現在の active が空タブ(無題かつ未編集)ならそれを使い回す、
      // そうでなければ新規タブを足す。
      captureActiveTabState();
      const current = getActiveTab();
      const reuseEmpty =
        current && current.path === null && !current.dirty &&
        current.content === "";
      if (reuseEmpty && current) {
        current.path = doc.path;
        current.content = doc.content;
        current.dirty = false;
        current.cursorAnchor = 0;
        current.cursorHead = 0;
        current.scrollTop = 0;
      } else {
        const tab: Tab = {
          id: newTabId(),
          path: doc.path,
          content: doc.content,
          dirty: false,
          cursorAnchor: 0,
          cursorHead: 0,
          scrollTop: 0,
        };
        tabs = [...tabs, tab];
        activeTabId = tab.id;
      }
      clearStatus();
      const newActive = getActiveTab();
      if (newActive && isTypstPath(newActive.path) && newActive.path) {
        await Promise.all([
          startPreview(newActive.path),
          ensureLspFor(newActive.path),
        ]);
      } else {
        await Promise.all([stopPreview(), stopLspSession()]);
      }
    } catch (e) {
      setStatus(String(e));
    }
  }

  function addEmptyTab() {
    captureActiveTabState();
    const tab = makeEmptyTab();
    tabs = [...tabs, tab];
    activeTabId = tab.id;
    // doc / カーソル / scroll は Editor.svelte 側の $effect が反映する。
    // 空タブは Typst でないので preview / LSP は止める
    void Promise.all([stopPreview(), stopLspSession()]);
  }

  async function closeTab(targetId: TabId) {
    const tab = getTab(targetId);
    if (!tab) return;
    if (tab.dirty) {
      const ok = await ask("未保存の変更があります。破棄して閉じますか?", {
        title: "右筆",
        kind: "warning",
      });
      if (!ok) return;
    }
    const idx = tabs.findIndex((t) => t.id === targetId);
    if (idx < 0) return;
    const wasActive = activeTabId === targetId;
    tabs = tabs.filter((t) => t.id !== targetId);
    if (tabs.length === 0) {
      // 全部閉じたら空タブを 1 枚自動で生成(ようこそ画面相当)
      const fresh = makeEmptyTab();
      tabs = [fresh];
      activeTabId = fresh.id;
      await Promise.all([stopPreview(), stopLspSession()]);
      return;
    }
    if (wasActive) {
      // 隣のタブを active に。手前優先(右に増えた感覚に合う)。
      const next = tabs[Math.max(0, idx - 1)];
      await switchTab(next.id);
    }
  }

  // コマンドカタログ (commands.ts) から各コマンドを呼ぶための依存。
  // ファイル系・画像ピッカーはここで束ねてカタログ側に渡す。
  function commandContext(): CommandContext {
    return {
      view: editorView,
      openFile,
      openFolder,
      save,
      saveAs,
      exportPdf,
      pickAndInsertImage,
      togglePreview,
      toggleProjectView,
      newTab: addEmptyTab,
      closeActiveTab: () => {
        if (activeTabId) closeTab(activeTabId);
      },
    };
  }

  function togglePreview() {
    previewVisible = !previewVisible;
    persistWorkspace();
  }

  function toggleProjectView() {
    projectViewVisible = !projectViewVisible;
    // 開かれたタイミングでフォルダ未選択なら自動で Open Folder ダイアログ
    if (projectViewVisible && !currentFolder) {
      openFolder();
    }
    persistWorkspace();
  }

  function persistWorkspace() {
    saveWorkspace({
      previewVisible,
      editorPaneRatio,
      projectViewVisible,
      projectPaneRatio,
      currentFolder,
    }).catch((e) => {
      // 永続化失敗はログのみ(ボタン操作はそのまま受け付ける)
      console.warn("workspace save failed:", e);
    });
  }

  function clampRatio(v: number, min: number, max: number): number {
    return Math.max(min, Math.min(max, v));
  }

  // スプリッタを掴んだら pointer capture して move/up を listen する。
  // 左:プロジェクトビューと右側全体の境界。右:エディタとプレビューの境界。
  function onSplitterDown(e: PointerEvent, which: "project" | "editor") {
    if (!workspaceEl) return;
    splitterTarget = which;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    e.preventDefault();
  }

  function onSplitterMove(e: PointerEvent) {
    if (!splitterTarget || !workspaceEl) return;
    if (splitterTarget === "project") {
      const rect = workspaceEl.getBoundingClientRect();
      const ratio = (e.clientX - rect.left) / rect.width;
      projectPaneRatio = clampRatio(ratio, 0.1, 0.5);
    } else if (splitterTarget === "editor") {
      // editor splitter は「右側ペイン(エディタ + プレビュー)」内の比率
      if (!editPreviewEl) return;
      const rect = editPreviewEl.getBoundingClientRect();
      const ratio = (e.clientX - rect.left) / rect.width;
      editorPaneRatio = clampRatio(ratio, 0.1, 0.9);
    }
  }

  function onSplitterUp(e: PointerEvent) {
    if (!splitterTarget) return;
    splitterTarget = null;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    persistWorkspace();
  }

  async function openFolder() {
    try {
      const selected = await openDialog({ directory: true, multiple: false });
      if (typeof selected !== "string") return;
      await loadProjectTree(selected);
      currentFolder = selected;
      projectViewVisible = true;
      persistWorkspace();
    } catch (e) {
      setStatus(String(e));
    }
  }

  async function loadProjectTree(folder: string) {
    try {
      projectTree = await listDirectory(folder);
    } catch (e) {
      projectTree = null;
      setStatus(`フォルダの読み込みに失敗: ${String(e)}`);
    }
  }

  async function refreshProjectTree() {
    if (!currentFolder) return;
    await loadProjectTree(currentFolder);
  }

  function toggleProjectExpanded(p: string) {
    projectExpanded = { ...projectExpanded, [p]: !projectExpanded[p] };
  }

  // ツリーのファイルクリック動作:
  //   .typ            → タブで開く(既存タブがあれば active に、なければ新規)
  //   テキスト系       → 同上(エディタで素のテキストとして開く)
  //   バイナリ(PDF/画像/etc)→ OS デフォルトアプリで開く
  const TEXT_LIKE_EXT = new Set([
    "typ",
    "md",
    "markdown",
    "txt",
    "csv",
    "tsv",
    "json",
    "jsonc",
    "yaml",
    "yml",
    "toml",
    "bib",
    "ini",
    "log",
    "html",
    "htm",
    "css",
    "scss",
    "js",
    "ts",
    "tsx",
    "jsx",
    "rs",
    "py",
    "sh",
    "go",
    "java",
    "c",
    "cpp",
    "h",
    "hpp",
    "xml",
    "svg",
  ]);

  function isTextLikePath(p: string): boolean {
    const ext = p.toLowerCase().split(".").pop() ?? "";
    return TEXT_LIKE_EXT.has(ext);
  }

  async function selectFromTree(target: string) {
    if (isTextLikePath(target)) {
      await openFileAtPath(target);
      return;
    }
    // バイナリは OS デフォルトに流す
    try {
      await openUrl(pathToFileUri(target));
    } catch (e) {
      setStatus(`外部で開くのに失敗: ${String(e)}`);
    }
  }

  async function runCommand(id: CommandId) {
    const def = COMMANDS[id];
    if (def.needsEditor && !editorView) return;
    await def.run(commandContext());
  }

  function effectiveKey(id: CommandId): string | undefined {
    return keybindings[id] ?? COMMANDS[id].defaultKey;
  }

  // "Mod-Shift-b" 形式のキー指定を表示用 ("Ctrl+Shift+B") に整える。
  function displayKey(spec: string): string {
    return spec.replaceAll("Mod", "Ctrl").replaceAll("-", "+");
  }

  // KeyboardEvent が "Mod-b" 形式の指定にマッチするかを判定。
  // Mod は Ctrl/Cmd 両対応。最終キーは大文字小文字を無視。
  function matchKey(e: KeyboardEvent, spec: string): boolean {
    const parts = spec.split("-");
    const last = parts[parts.length - 1];
    const wantMod = parts.includes("Mod");
    const wantShift = parts.includes("Shift");
    const wantAlt = parts.includes("Alt");
    if (e.key.toLowerCase() !== last.toLowerCase()) return false;
    const hasMod = e.ctrlKey || e.metaKey;
    if (wantMod !== hasMod) return false;
    if (wantShift !== e.shiftKey) return false;
    if (wantAlt !== e.altKey) return false;
    return true;
  }

  function buttonTitle(id: CommandId): string {
    const def = COMMANDS[id];
    const key = effectiveKey(id);
    return key ? `${def.label} (${displayKey(key)})` : def.label;
  }

  function isImagePath(p: string): boolean {
    const ext = p.toLowerCase().split(".").pop() ?? "";
    return IMAGE_EXTS.includes(ext);
  }

  // 現在編集中ファイルのディレクトリを基準に、選んだ画像の絶対パスを
  // 相対パスへ変換する。サブ階層は `foo.png`、上階層をまたぐ場合は
  // `../../Pictures/foo.png` のように `../` を積む。ファイル未保存や
  // Windows でドライブが違うなど相対化できないときは絶対パス(forward slash)
  // をそのまま返す(Typst は OS を跨いでも `/` 区切りで読める)。
  async function toRelativePath(absolute: string): Promise<string> {
    if (!path) return absolute.replaceAll("\\", "/");
    try {
      const baseDir = await dirname(path);
      return relativeFromDir(baseDir, absolute);
    } catch {
      return absolute.replaceAll("\\", "/");
    }
  }

  function relativeFromDir(fromDir: string, toFile: string): string {
    const isWin =
      /^[A-Za-z]:[\\/]/.test(fromDir) || /^[A-Za-z]:[\\/]/.test(toFile);
    const split = (p: string) => p.replaceAll("\\", "/").split("/").filter(Boolean);
    const fromParts = split(fromDir);
    const toParts = split(toFile);
    // Windows でドライブが違うと相対化不能 → 絶対パス(/)で返す
    if (
      isWin &&
      fromParts[0]?.toLowerCase() !== toParts[0]?.toLowerCase()
    ) {
      return toFile.replaceAll("\\", "/");
    }
    let i = 0;
    while (
      i < fromParts.length &&
      i < toParts.length &&
      fromParts[i] === toParts[i]
    ) {
      i++;
    }
    const ups = "../".repeat(fromParts.length - i);
    const rest = toParts.slice(i).join("/");
    const result = ups + rest;
    return result === "" ? "./" : result;
  }

  async function pickAndInsertImage() {
    if (!editorView) return;
    try {
      const selected = await openDialog({
        multiple: false,
        filters: IMAGE_FILTERS,
      });
      if (typeof selected !== "string") return;
      const rel = await toRelativePath(selected);
      insertImage(editorView, rel);
    } catch (e) {
      setStatus(String(e));
    }
  }

  function basename(p: string | null): string {
    if (!p) return "(無題)";
    const i = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
    return i >= 0 ? p.slice(i + 1) : p;
  }

  // LSP の hover ツールチップなどに含まれる外部リンク(file:// を含む)は
  // WebView 内でナビゲーションすると "The URL can't be shown" で弾かれる。
  // capture phase で横取りし、tauri-plugin-opener 経由で OS デフォルトに流す。
  function onAnchorClickCapture(e: MouseEvent) {
    const target = e.target as HTMLElement | null;
    const anchor = target?.closest("a");
    dlog(
      "[anchor-click] target=",
      target?.tagName,
      "anchor=",
      anchor?.tagName,
      "href=",
      anchor?.getAttribute("href"),
    );
    if (!anchor) return;
    const href = anchor.getAttribute("href");
    if (!href) return;
    // ページ内アンカーや javascript: 等はそのまま流す
    if (href.startsWith("#") || href.startsWith("javascript:")) return;
    e.preventDefault();
    e.stopPropagation();
    dlog("[anchor-click] forwarding to openUrl:", href);
    openUrl(href)
      .then(() => dlog("[anchor-click] openUrl resolved"))
      .catch((err) => dlog("[anchor-click] openUrl failed:", String(err)));
  }

  function onKeydown(e: KeyboardEvent) {
    // 修飾キーを伴わないキー入力はエディタ本体に渡す(IME / vim 等)
    if (!e.ctrlKey && !e.metaKey && !e.altKey) return;
    for (const id of COMMAND_IDS) {
      const key = effectiveKey(id);
      if (!key) continue;
      if (!matchKey(e, key)) continue;
      const def = COMMANDS[id];
      // editor を必要とするコマンドは view が無いときパススルー。
      // これにより vim Normal モードのデフォルトキー(Ctrl+B 等)も
      // editor 起動前は素で動く。
      if (def.needsEditor && !editorView) return;
      e.preventDefault();
      runCommand(id);
      return;
    }
  }

  onMount(() => {
    let unlisten: (() => void) | undefined;
    let unlistenDrop: (() => void) | undefined;
    (async () => {
      const win = getCurrentWindow();
      unlisten = await win.onCloseRequested(async (event) => {
        if (!dirty) {
          // dirty でなくても preview / LSP の subprocess は止めたい
          await stopPreview();
          await stopLspSession();
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
          await stopLspSession();
          await win.destroy();
        }
      });
      // 設定の読み込みは hydration 後に。Tauri Store は async API なので
      // onMount で起動時に1回読む。失敗時はデフォルトを使う。
      try {
        const settings = await loadSettings();
        editorMode = settings.editor.mode;
        toolbarItems = settings.toolbar.items;
        keybindings = settings.keybindings;
        previewVisible = settings.workspace.previewVisible;
        editorPaneRatio = settings.workspace.editorPaneRatio;
        projectViewVisible = settings.workspace.projectViewVisible;
        projectPaneRatio = settings.workspace.projectPaneRatio;
        currentFolder = settings.workspace.currentFolder;
        // 前回開いていたフォルダを復元(失敗しても致命的でない)
        if (currentFolder) {
          await loadProjectTree(currentFolder);
        }
      } catch (e) {
        console.warn("settings load failed, using defaults:", e);
      }
      // ウィンドウへの画像ファイル D&D を受け取り、ドロップされた最初の
      // 画像を現在のカーソル位置に挿入する。Tauri 2 の onDragDropEvent は
      // OS 経由のネイティブ D&D を捕まえる。
      try {
        const webview = getCurrentWebview();
        unlistenDrop = await webview.onDragDropEvent(async (event) => {
          if (event.payload.type !== "drop") return;
          if (!editorView) return;
          const target = event.payload.paths.find(isImagePath);
          if (!target) return;
          const rel = await toRelativePath(target);
          insertImage(editorView, rel);
        });
      } catch (e) {
        console.warn("dragdrop subscribe failed:", e);
      }
      const mod = await import("$lib/Editor.svelte");
      Editor = mod.default;
    })();
    return () => {
      unlisten?.();
      unlistenDrop?.();
    };
  });
</script>

<svelte:window onkeydown={onKeydown} onclickcapture={onAnchorClickCapture} />

<div class="app">
  <header class="toolbar">
    {#each toolbarItems as item, i (i)}
      {#if item === "divider"}
        <span class="toolbar-divider" aria-hidden="true"></span>
      {:else}
        {@const def = COMMANDS[item]}
        {@const Icon = def.icon}
        <button
          class="icon-btn"
          aria-label={def.label}
          title={buttonTitle(def.id)}
          disabled={def.needsEditor && !editorView}
          onclick={() => runCommand(def.id)}
        >
          <Icon size={18} />
        </button>
      {/if}
    {/each}
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

  <div class="workspace" bind:this={workspaceEl}>
    {#if projectViewVisible}
      <aside
        class="project-pane"
        style:flex={`0 0 ${projectPaneRatio * 100}%`}
      >
        <div class="project-header">
          {#if currentFolder}
            <span class="folder-name" title={currentFolder}
              >{basename(currentFolder)}</span
            >
            <button
              class="header-action"
              title="フォルダを開く"
              onclick={openFolder}>変更</button
            >
            <button
              class="header-action"
              title="再読み込み"
              onclick={refreshProjectTree}>更新</button
            >
          {:else}
            <span class="folder-name muted">フォルダ未選択</span>
            <button
              class="header-action"
              title="フォルダを開く"
              onclick={openFolder}>開く</button
            >
          {/if}
        </div>
        <div class="project-body">
          {#if projectTree && projectTree.children}
            <ProjectTree
              entries={projectTree.children}
              activePath={path}
              onOpenFile={selectFromTree}
              expanded={projectExpanded}
              onToggleExpanded={toggleProjectExpanded}
            />
          {:else if currentFolder}
            <div class="placeholder">読み込み中…</div>
          {:else}
            <div class="placeholder">
              「フォルダを開く」を押して、文書プロジェクトのフォルダを選んでください。
            </div>
          {/if}
        </div>
      </aside>

      <div
        class="splitter"
        class:dragging={splitterTarget === "project"}
        role="separator"
        aria-orientation="vertical"
        aria-label="プロジェクトビューとエディタの境界"
        onpointerdown={(e) => onSplitterDown(e, "project")}
        onpointermove={onSplitterMove}
        onpointerup={onSplitterUp}
        onpointercancel={onSplitterUp}
      ></div>
    {/if}

    <div class="edit-preview">
      <div class="tabbar" role="tablist">
        {#each tabs as tab (tab.id)}
          <div
            class="tab"
            class:active={tab.id === activeTabId}
            class:dragging={draggingTabId === tab.id}
            class:drag-over={dragOverTabId === tab.id}
            role="tab"
            aria-selected={tab.id === activeTabId}
            tabindex={tab.id === activeTabId ? 0 : -1}
            title={tab.path ?? "(無題)"}
            data-tab-id={tab.id}
            onpointerdown={(e) => onTabPointerDown(e, tab.id)}
            onpointermove={onTabPointerMove}
            onpointerup={(e) => onTabPointerUp(e, tab.id)}
            onpointercancel={(e) => onTabPointerUp(e, tab.id)}
          >
            <span class="tab-label">
              <span class="tab-name"
                >{tab.path ? basename(tab.path) : "(無題)"}</span
              >
              {#if tab.dirty}<span class="tab-dirty" aria-hidden="true">●</span>{/if}
            </span>
            <button
              class="tab-close"
              aria-label="タブを閉じる"
              title="閉じる"
              onpointerdown={(e) => e.stopPropagation()}
              onclick={(e) => {
                e.stopPropagation();
                closeTab(tab.id);
              }}>×</button
            >
          </div>
        {/each}
        <button
          class="tab-new"
          aria-label="新規タブ"
          title="新規タブ"
          onclick={addEmptyTab}>＋</button
        >
      </div>

      <div class="edit-preview-row" bind:this={editPreviewEl}>
        <div
          class="editor-pane"
          style:flex={previewVisible
            ? `0 0 ${editorPaneRatio * 100}%`
            : "1 1 100%"}
        >
        {#if Editor}
          <Editor
            value={content}
            mode={editorMode}
            languageMode={isTypstPath(path) ? "typst" : "plain"}
            {lspClient}
            filePath={path}
            onChange={onEditorChange}
            onReady={onEditorReady}
            onTeardown={onEditorTeardown}
            onValueApplied={onEditorValueApplied}
          />
        {:else}
          <div class="placeholder">エディタを読み込み中…</div>
        {/if}
      </div>

      {#if previewVisible}
        <div
          class="splitter"
          class:dragging={splitterTarget === "editor"}
          role="separator"
          aria-orientation="vertical"
          aria-label="エディタとプレビューの境界"
          onpointerdown={(e) => onSplitterDown(e, "editor")}
          onpointermove={onSplitterMove}
          onpointerup={onSplitterUp}
          onpointercancel={onSplitterUp}
        ></div>

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
      {/if}
      </div>
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

  .toolbar-divider {
    width: 1px;
    align-self: stretch;
    background: #3a3a3a;
    margin: 2px 4px;
  }

  .toolbar button.icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 5px 7px;
    line-height: 1;
  }

  .toolbar button.icon-btn :global(svg) {
    display: block;
    stroke-width: 2;
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

  .edit-preview {
    flex: 1 1 0;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .edit-preview-row {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: row;
  }

  .tabbar {
    display: flex;
    align-items: stretch;
    background: #232323;
    border-bottom: 1px solid #2a2a2a;
    overflow-x: auto;
    overflow-y: hidden;
    flex-shrink: 0;
    height: 32px;
  }

  .tab {
    display: flex;
    align-items: center;
    border-right: 1px solid #2a2a2a;
    background: #2a2a2a;
    min-width: 110px;
    max-width: 220px;
    flex-shrink: 0;
    cursor: pointer;
    user-select: none;
    -webkit-user-select: none;
    touch-action: none;
  }

  .tab.active {
    background: #1e1e1e;
  }

  .tab.dragging {
    opacity: 0.4;
  }

  .tab.drag-over {
    box-shadow: inset 2px 0 0 #61afef;
  }

  .tab-label {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    color: #b0b0b0;
    font-size: 12px;
    height: 100%;
    overflow: hidden;
    user-select: none;
  }

  .tab.active .tab-label {
    color: #ffffff;
  }

  .tab-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: none;
  }

  .tab-dirty {
    color: #61afef;
    font-size: 10px;
    flex-shrink: 0;
  }

  .tab-close {
    padding: 0 8px;
    background: transparent;
    border: none;
    color: #777;
    cursor: pointer;
    font-size: 14px;
    height: 100%;
    line-height: 1;
  }

  .tab-close:hover {
    color: #ffffff;
    background: #3a3a3a;
  }

  .tab-new {
    padding: 0 12px;
    height: 100%;
    background: transparent;
    border: none;
    color: #888;
    cursor: pointer;
    font-size: 16px;
    flex-shrink: 0;
  }

  .tab-new:hover {
    background: #2a2a2a;
    color: #ffffff;
  }

  .editor-pane,
  .preview-pane {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .preview-pane {
    flex: 1 1 0;
    background: #2a2a2a;
  }

  .project-pane {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: #232323;
    border-right: 1px solid #2a2a2a;
  }

  .project-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border-bottom: 1px solid #2a2a2a;
    font-size: 12px;
  }

  .folder-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #d0d0d0;
    font-weight: 600;
  }

  .folder-name.muted {
    color: #888;
    font-weight: 400;
  }

  .header-action {
    padding: 2px 8px;
    background: #3a3a3a;
    color: #d0d0d0;
    border: 1px solid #4a4a4a;
    border-radius: 3px;
    font-size: 11px;
    cursor: pointer;
  }

  .header-action:hover {
    background: #4a4a4a;
  }

  .project-body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 4px 0;
  }

  .splitter {
    flex: 0 0 6px;
    align-self: stretch;
    background: #2a2a2a;
    cursor: col-resize;
    user-select: none;
    touch-action: none;
    transition: background 100ms ease;
  }

  .splitter:hover {
    background: #4a4a4a;
  }

  .splitter.dragging {
    background: #61afef;
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
