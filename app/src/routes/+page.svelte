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
    type LocaleMode,
    type PaperSize,
    type ThemeMode,
    loadSettings,
    validateSettingsJson,
    saveFirstRunDone,
    saveWorkspace,
    saveKeybindings,
    saveToolbarItems,
  } from "$lib/settings";
  import type { LSPClient } from "@codemirror/lsp-client";
  import { pathToFileUri, startLspSession, type LspSession } from "$lib/lsp";
  import type { EditorView } from "@codemirror/view";
  import type { EditorState } from "@codemirror/state";
  import {
    saveTabState,
    loadTabState,
    type PersistedTab,
  } from "$lib/tab-persist";
  import { insertBibliography, insertImage } from "$lib/editor-commands";
  import { listDirectory, loadGitStatus, type DirEntry } from "$lib/project";
  import ProjectTree from "$lib/ProjectTree.svelte";
  import TemplateDialog from "$lib/TemplateDialog.svelte";
  import KeybindingsDialog from "$lib/KeybindingsDialog.svelte";
  import ToolbarEditDialog from "$lib/ToolbarEditDialog.svelte";
  import CommandPalette from "$lib/CommandPalette.svelte";
  import FormPanel from "$lib/FormPanel.svelte";
  import { resolveLocale, type Locale } from "$lib/i18n/locale";
  import { setLocale, t } from "$lib/i18n/index.svelte";
  import { listTemplates, resolveTemplate } from "$lib/templates";
  import {
    findWithCall,
    rebuildWithCall,
    type ArgKind,
  } from "$lib/template-args";
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
  type CursorInfo = {
    line: number;
    col: number;
    selected: number;
    selectedNoWs: number;
    total: number;
    totalNoWs: number;
  };
  let Editor = $state<Component<{
    value: string;
    externalState?: EditorState | null;
    mode?: EditorMode;
    languageMode?: "typst" | "plain";
    lspClient?: LSPClient | null;
    filePath?: string | null;
    onChange?: (next: string) => void;
    onCursorChange?: (info: CursorInfo) => void;
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
  let themeMode = $state<ThemeMode>("auto");
  let localeMode = $state<LocaleMode>("auto");
  let paperSize = $state<PaperSize>("auto");
  let firstRunDone = $state(true); // 起動時に loadSettings で本物の値に上書き
  let templateDialogOpen = $state(false);
  let keybindingsDialogOpen = $state(false);
  let toolbarEditDialogOpen = $state(false);
  let commandPaletteOpen = $state(false);
  // settings.json の絶対パス。起動時に Rust から取得して保持し、
  // save() でこのパスに書いたら自動で reloadSettings を呼ぶための比較用。
  let settingsPath = $state<string | null>(null);

  // localeMode → 解決後の "ja" / "en"。テンプレ表示等で使う。
  let resolvedLocale = $derived<Locale>(resolveLocale(localeMode));
  const allTemplates = listTemplates();

  // ツールバーの並び・キーバインドは設定で書き換え可能。
  // 設定読み込み完了までは getDefaultToolbarItems() を表示する。
  let toolbarItems = $state<ToolbarItem[]>(getDefaultToolbarItems());
  let keybindings = $state<KeybindingsSettings>({});

  // ワークスペース表示。preview / プロジェクトビューの有無 + 境界比率はユーザが操作する。
  let previewVisible = $state(true);
  let editorPaneRatio = $state(0.5);
  let projectViewVisible = $state(false);
  let projectPaneRatio = $state(0.18);
  let statusbarVisible = $state(false);
  let charCountMode = $state<"non-whitespace" | "all">("non-whitespace");
  // 左サイドバーのレイアウト(α: split / γ: tabs)。settings から復元する。
  let sidebarMode = $state<"split" | "tabs">("split");
  let sidebarSplitRatio = $state(0.55);
  let sidebarActiveTab = $state<"project" | "form">("project");
  let workspaceEl = $state<HTMLDivElement | null>(null);
  let editPreviewEl = $state<HTMLDivElement | null>(null);
  let projectPaneEl = $state<HTMLElement | null>(null);
  let splitterTarget = $state<"project" | "editor" | "sidebar" | null>(null);

  // プロジェクト状態。currentFolder は永続化、projectTree は起動時に
  // 読み込む。expanded はセッション内のみ(リロードで初期化)。
  let currentFolder = $state<string | null>(null);
  let projectTree = $state<DirEntry | null>(null);
  let projectExpanded = $state<Record<string, boolean>>({});

  // テーマを documentElement に反映する。"auto" は prefers-color-scheme を見て
  // 解決後の dark/light を適用。"auto" 中に OS 側が変わったら matchMedia で追従。
  function resolveTheme(t: ThemeMode): "light" | "dark" {
    if (t === "auto") {
      return window.matchMedia("(prefers-color-scheme: light)").matches
        ? "light"
        : "dark";
    }
    return t;
  }

  function applyTheme(t: ThemeMode) {
    const resolved = resolveTheme(t);
    if (resolved === "light") {
      document.documentElement.dataset.theme = "light";
    } else {
      delete document.documentElement.dataset.theme;
    }
  }

  // タブ状態の永続化:操作の度に小さな debounce 後で書き出す。
  // 開いていたタブの順序・active 位置・無題タブの本文を tabs.json に
  // 保存し、次回起動時の hot exit 復元に使う。
  let persistTabsTimer: ReturnType<typeof setTimeout> | null = null;
  const TAB_PERSIST_DEBOUNCE_MS = 300;

  function schedulePersistTabs() {
    if (persistTabsTimer !== null) clearTimeout(persistTabsTimer);
    persistTabsTimer = setTimeout(() => {
      persistTabsTimer = null;
      void persistTabs();
    }, TAB_PERSIST_DEBOUNCE_MS);
  }

  // 永続化に乗せる per-tab スナップショット。
  // active タブは captureActiveTabState を先に呼び view.state を反映させてから
  // 取り出すので、ここでは tab に控えてあるフィールドをそのまま使う。
  async function persistTabs(): Promise<void> {
    captureActiveTabState();
    const out: PersistedTab[] = [];
    let activeIndex = -1;
    for (const tab of tabs) {
      const isActive = tab.id === activeTabId;
      if (tab.path) {
        // 実ファイルタブは path のみ(content はディスクから読み直す)。
        out.push({
          kind: "file",
          path: tab.path,
          cursorAnchor: tab.cursorAnchor,
          cursorHead: tab.cursorHead,
          scrollTop: tab.scrollTop,
        });
      } else {
        // 無題タブは中身が空ならスキップ(復元する価値が無い)。
        if (tab.content.length === 0) continue;
        out.push({
          kind: "untitled",
          content: tab.content,
          cursorAnchor: tab.cursorAnchor,
          cursorHead: tab.cursorHead,
          scrollTop: tab.scrollTop,
        });
      }
      if (isActive) activeIndex = out.length - 1;
    }
    await saveTabState({ tabs: out, activeIndex });
  }

  // 起動時にタブ状態を復元する。返り値は「復元が走ったか」(走らなかった
  // 時はデフォルトの空タブ 1 枚で起動済みなので何もしない)。
  async function restoreTabs(): Promise<boolean> {
    const state = await loadTabState();
    if (!state || state.tabs.length === 0) return false;

    const restored: Tab[] = [];
    for (const persisted of state.tabs) {
      try {
        if (persisted.kind === "file") {
          const doc = await invoke<FileDoc>("open_file", { path: persisted.path });
          restored.push({
            id: newTabId(),
            path: doc.path,
            content: doc.content,
            dirty: false,
            cursorAnchor: persisted.cursorAnchor,
            cursorHead: persisted.cursorHead,
            scrollTop: persisted.scrollTop,
            editorState: null,
            virtualPath: null,
          });
        } else {
          const id = newTabId();
          // 無題タブは新しい仮想ファイルを作って content を書き込む。
          // 古い仮想ファイルは tab.id ベースで作られていたが起動毎に
          // ID が変わるので、復元時は新規割り当てになる。
          let virtualPath: string | null = null;
          try {
            virtualPath = await invoke<string>("prepare_untitled_path", {
              tabId: id,
            });
            await invoke("save_file", {
              path: virtualPath,
              content: persisted.content,
            });
          } catch (e) {
            console.warn("[tabs] untitled restore (virtual file) failed:", e);
          }
          restored.push({
            id,
            path: null,
            content: persisted.content,
            // 復元時は「未保存の編集が残っていた」状態なので dirty=true
            dirty: true,
            cursorAnchor: persisted.cursorAnchor,
            cursorHead: persisted.cursorHead,
            scrollTop: persisted.scrollTop,
            editorState: null,
            virtualPath,
          });
        }
      } catch (e) {
        // 個別タブの復元失敗(ファイル消失など)は skip して続行
        console.warn("[tabs] restore one tab failed:", e);
      }
    }

    if (restored.length === 0) return false;

    // 既存の初期空タブの仮想ファイルを掃除してから入れ替え
    for (const t of tabs) await disposeVirtualPathFor(t);
    tabs = restored;
    const idx = Math.min(Math.max(state.activeIndex, 0), restored.length - 1);
    activeTabId = restored[idx]?.id ?? null;
    return true;
  }

  // settings.json のエラー表示状態を覚えるフラグ。reloadSettings 成功時に
  // 自動クリアするために使う(ユーザが直したらステータスバーが綺麗になる)。
  let settingsErrorActive = false;

  // 設定ファイル(settings.json)を外部エディタで書き換えた後、
  // Yuhitsu にフォーカスが戻った時点で再読み込みして反映する。
  // 設定 UI 画面ができるまでの暫定手段(Phase 2 で正式 UI を予定)。
  async function reloadSettings() {
    // JSON 構文を先に検証して、エラーがあればステータスバーに表示する。
    // Tauri Store の reload は壊れた JSON を黙ってデフォルトに戻すため、
    // ユーザは「設定を変えたのに反映されない」だけ気付くことになる。
    // 行・列付きのメッセージを出して原因を即座に分かるようにする。
    const jsonError = await validateSettingsJson();
    if (jsonError) {
      setStatus(t("status.settingsJsonError", { error: jsonError }));
      settingsErrorActive = true;
      return;
    }
    try {
      const settings = await loadSettings();
      editorMode = settings.editor.mode;
      themeMode = settings.appearance.theme;
      applyTheme(themeMode);
      localeMode = settings.appearance.locale;
      // i18n 辞書側に locale を伝達(resolvedLocale は derived だが、
      // setLocale は明示呼び出しが必要)
      setLocale(resolveLocale(localeMode));
      paperSize = settings.document.paperSize;
      toolbarItems = settings.toolbar.items;
      keybindings = settings.keybindings;
      statusbarVisible = settings.workspace.statusbarVisible;
      charCountMode = settings.workspace.charCountMode;
      sidebarMode = settings.workspace.sidebarMode;
      sidebarSplitRatio = settings.workspace.sidebarSplitRatio;
      sidebarActiveTab = settings.workspace.sidebarActiveTab;
      // JSON エラー表示が残っていた時だけクリア(他の error メッセージは
      // 触らない)
      if (settingsErrorActive) {
        clearStatus();
        settingsErrorActive = false;
      }
    } catch (e) {
      setStatus(t("status.settingsLoadFailed", { error: String(e) }));
      settingsErrorActive = true;
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
    // タブ切替で復元する CodeMirror の state スナップショット。
    // doc / 選択 / undo redo スタック / scroll などをまるごと保持する。
    // 取得タイミング:タブ切替前(captureActiveTabState で view.state を控える)。
    // 復元タイミング:Editor.svelte の $effect で view.setState(tab.editorState)。
    editorState: EditorState | null;
    // 無題タブで preview / LSP を有効にするための仮想 .typ パス。
    // <app_cache_dir>/untitled/<tab.id>.typ を Rust 側で空ファイルとして
    // 作成し、ここに保持する。tab.path が確定したら(saveAs 後)
    // cleanup_untitled_path で削除し null に戻す。
    virtualPath: string | null;
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
      editorState: null,
      virtualPath: null,
    };
  }

  // 無題タブで preview / LSP を起動する直前に呼び、必要なら仮想ファイルを
  // 作成して virtualPath を入れる。返り値は preview / LSP に渡す path
  // (実 path 優先、なければ仮想 path、Typst として扱えなければ null)。
  async function ensurePreviewablePath(tab: Tab): Promise<string | null> {
    if (tab.path) return isTypstPath(tab.path) ? tab.path : null;
    if (!tab.virtualPath) {
      try {
        tab.virtualPath = await invoke<string>("prepare_untitled_path", {
          tabId: tab.id,
        });
      } catch (e) {
        console.warn("[preview] prepare_untitled_path failed:", e);
        return null;
      }
    }
    return tab.virtualPath;
  }

  async function disposeVirtualPathFor(tab: Tab) {
    if (!tab.virtualPath) return;
    const dead = tab.virtualPath;
    tab.virtualPath = null;
    try {
      await invoke("cleanup_untitled_path", { path: dead });
    } catch (e) {
      console.warn("[preview] cleanup_untitled_path failed:", e);
    }
  }

  function isTypstPath(p: string | null | undefined): boolean {
    return !!p && p.toLowerCase().endsWith(".typ");
  }

  // タブが Typst 機能(preview / LSP / PDF / 構文ハイライト)の対象か判定。
  // 無題タブ(path=null)は新規 Typst ドキュメント前提で常に対象。
  // 既存ファイルを開いたタブは拡張子 .typ のみ対象、それ以外(.md / .csv 等)は対象外。
  function isTypstTab(tab: Tab | null | undefined): boolean {
    if (!tab) return false;
    if (tab.path === null) return true;
    return isTypstPath(tab.path);
  }

  // ファイル選択ダイアログのフィルタ。locale 変更後にも追従させたいので、
  // 使用時に毎回 t() を呼んで生成する(関数化)。
  const filtersTypst = () => [{ name: t("filter.typst"), extensions: ["typ"] }];
  const filtersPdf = () => [{ name: t("filter.pdf"), extensions: ["pdf"] }];
  const IMAGE_EXTS = [
    "png",
    "jpg",
    "jpeg",
    "gif",
    "svg",
    "webp",
    "avif",
  ];
  const filtersImage = () => [{ name: t("filter.image"), extensions: IMAGE_EXTS }];
  const PREVIEW_URL = "http://127.0.0.1:23625/";
  // 編集中バッファの preview 反映 debounce(ms)。短すぎると preview が
  // 過剰に再コンパイル、長すぎると体感遅延。Typst の incremental compile
  // は速いので 150ms は無難な落としどころ。
  const PREVIEW_MEMORY_DEBOUNCE_MS = 150;

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
    // 中クリック(button=1)はタブ閉じ(ブラウザ流儀)。pointerdown 時点で
    // 即発火させる(pointerup を待たずに)— ユーザ体感が速い方が良い
    if (e.button === 1) {
      e.preventDefault();
      void closeTab(id);
      return;
    }
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
    schedulePersistTabs();
  }

  // 既存コードに最小の変更で乗るよう、active tab 由来の値を path/content/dirty
  // として derived 公開する。書き換えは getActiveTab() を直接 mutate する。
  let path = $derived<string | null>(getActiveTab()?.path ?? null);
  let content = $derived(getActiveTab()?.content ?? "");
  let dirty = $derived(getActiveTab()?.dirty ?? false);
  // Editor に渡す filePath は実 path 優先、なければ仮想 path(無題タブで
  // 仮想 .typ を作っている場合)。LSP 機能(補完 / 診断 / hover)は filePath
  // が無いと一切動かないので、無題タブでも virtualPath を渡して有効化する。
  let editorFilePath = $derived<string | null>(
    getActiveTab()?.path ?? getActiveTab()?.virtualPath ?? null,
  );
  // タブ切替時に Editor.svelte が view.setState で復元するための state。
  // ファイル open / 起動直後のタブは null、切替前のタブには captureActiveTabState
  // で設定済み。Editor 側で 1 度適用したら親が同じ参照を渡している間は再適用しない。
  let activeEditorState = $derived<EditorState | null>(
    getActiveTab()?.editorState ?? null,
  );

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
      setStatus(t("status.lspStartFailed", { error: String(e) }));
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
      // Rust 側で起動完了プローブ + control plane WebSocket 接続まで待つ。
      // ここから抜けた時点で preview_update_memory が呼べる状態。
      await invoke("start_preview", { path: forPath });
    } catch (e) {
      previewStatus = "error";
      previewError = String(e);
      return;
    }
    // 起動直後は memory sync を送らない。初回 doc は tinymist が
    // ディスクから読み込んだ内容で broadcast されており、エディタの内容
    // とも一致している(ファイル open 直後)。早期に updateMemoryFiles
    // を送ると tinymist 内部の初回 doc 状態と競合し、iframe からの
    // `current` リクエストに対して SVG が返らなくなる挙動を観測した。
    // ユーザが編集したら onChange の debounce で memory に流れ始める。
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
    // 切替直後の遅延更新で旧パスを送るのを防ぐ
    if (previewMemoryTimer !== null) {
      clearTimeout(previewMemoryTimer);
      previewMemoryTimer = null;
    }
  }

  // 編集中の未保存バッファを preview に注入する debounce タイマー。
  // タイマー発火時点での active タブ内容を control plane WS 越しに
  // tinymist preview の memory file として注入する(updateMemoryFiles)。
  // tinymist は watch ベースのディスク変更にも反応するが、それはファイルを
  // 保存した時のみ。未保存の編集をリアルタイム反映するには memory 注入が要る。
  let previewMemoryTimer: ReturnType<typeof setTimeout> | null = null;

  function schedulePreviewMemoryUpdate() {
    if (previewMemoryTimer !== null) {
      clearTimeout(previewMemoryTimer);
    }
    previewMemoryTimer = setTimeout(async () => {
      previewMemoryTimer = null;
      const tab = getActiveTab();
      if (!tab || !isTypstTab(tab)) return;
      if (previewStatus !== "ready") return;
      // 無題タブは virtualPath、実ファイルタブは tab.path を使う。
      const target = tab.path ?? tab.virtualPath;
      if (!target) return;
      try {
        await invoke("preview_update_memory", {
          path: target,
          content: tab.content,
        });
      } catch (e) {
        // preview が落ちた直後など、送信失敗は黙殺(次回再起動で復活する)
        console.warn("[preview] update_memory failed:", e);
      }
    }, PREVIEW_MEMORY_DEBOUNCE_MS);
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

  // 戻り値: 保存に成功したら true、ユーザがキャンセル / エラーなら false。
  // closeTab 等から「保存できたか」で次の動作を分岐するために使う。
  async function saveAs(): Promise<boolean> {
    const tab = getActiveTab();
    if (!tab) return false;
    try {
      const selected = await saveDialog({
        filters: filtersTypst(),
        defaultPath: tab.path ?? "untitled.typ",
      });
      if (!selected) return false;
      await invoke("save_file", { path: selected, content: tab.content });
      const isNewPath = tab.path !== selected;
      tab.path = selected;
      tab.dirty = false;
      // 無題タブから初めて保存した場合、仮想ファイルを掃除して preview を
      // 新しい実 path に乗せ替える。
      await disposeVirtualPathFor(tab);
      schedulePersistTabs();
      clearStatus();
      if (isNewPath) {
        if (isTypstPath(selected)) {
          await Promise.all([startPreview(selected), ensureLspFor(selected)]);
        } else {
          await Promise.all([stopPreview(), stopLspSession()]);
        }
      }
      return true;
    } catch (e) {
      setStatus(String(e));
      return false;
    }
  }

  async function save(): Promise<boolean> {
    const tab = getActiveTab();
    if (!tab) return false;
    if (!tab.path) {
      return await saveAs();
    }
    try {
      await invoke("save_file", { path: tab.path, content: tab.content });
      tab.dirty = false;
      clearStatus();
      // settings.json を保存したら即時に再読み込み(focus イベント任せだと
      // タブ切替だけでは発火しないので、保存契機で確実に反映させる)
      if (settingsPath && tab.path === settingsPath) {
        await reloadSettings();
      }
      // 保存で変更状態が変わるので、プロジェクトビューの git バッジも更新
      void refreshGitStatus();
      // tinymist がファイル変更を watch しているので start_preview 再起動は不要
      return true;
    } catch (e) {
      setStatus(String(e));
      return false;
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
      setStatus(t("status.pdfNotTypst"));
      return;
    }
    if (!tab.path) {
      setStatus(t("status.pdfNotSaved"));
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
        filters: filtersPdf(),
        defaultPath: defaultPdfPath(tab.path),
      });
      if (!selected) return;
      outputPath = selected;
    } catch (e) {
      setStatus(String(e));
      return;
    }
    setStatus(t("status.pdfWriting"), "info");
    try {
      await invoke("export_pdf", { input: tab.path, output: outputPath });
      const prefix = savedAutomatically ? t("status.pdfPrefixSaved") : "";
      setStatus(t("status.pdfWritten", { prefix, path: outputPath }), "info");
    } catch (e) {
      setStatus(String(e));
    }
  }

  // ステータスバー表示用のカーソル情報。タブ切替時は Editor の onValueApplied
  // → setState 経由で selectionSet が立ち、updateListener から自動更新される。
  let cursor = $state<CursorInfo | null>(null);

  function onEditorChange(next: string) {
    const tab = getActiveTab();
    if (!tab) return;
    // タブ切替時の dispatch でも updateListener が走るが、その時点では
    // tab.content は新値と一致するので no-op で抜ける(dirty フラグを誤らせない)
    if (tab.content === next) return;
    tab.content = next;
    tab.dirty = true;
    // ディスクに書かずに preview だけ最新にする(debounce 経由)
    schedulePreviewMemoryUpdate();
    // 無題タブの本文を hot exit 用に永続化(file タブは content を
    // 書かないが、active 位置の保存のためにも一応呼ぶ)
    schedulePersistTabs();
  }

  function onEditorReady(view: EditorView) {
    editorView = view;
  }

  // FormPanel から呼ばれる書き戻し。doc を最新の状態で再パースしてから差し替え、
  // editorView 経由で transaction を打つ(undo 可能、updateListener も走る)。
  function onFormApply(name: string, value: ArgKind) {
    if (!editorView) return;
    const doc = editorView.state.doc.toString();
    const call = findWithCall(doc);
    if (!call) return;
    const idx = call.args.findIndex((a) => a.name === name);
    const nextArgs =
      idx >= 0
        ? call.args.map((a, i) => (i === idx ? { name, value } : a))
        : [...call.args, { name, value }];
    const change = rebuildWithCall(doc, call, nextArgs);
    editorView.dispatch({ changes: change });
  }

  // 同梱テンプレからの form spec 解決。`#show: <fn>.with(...)` の関数名と
  // 一致するテンプレを探す。一致する同梱テンプレが無ければ汎用フォールバック
  // (FormPanel 側で call.args そのままから入力欄を生成)。
  let activeFormSpec = $derived(resolveFormSpec(content));

  function resolveFormSpec(doc: string) {
    const call = findWithCall(doc);
    if (!call) return null;
    for (const tpl of allTemplates) {
      if (tpl.form && tpl.form.function === call.fn) return tpl.form;
    }
    return null;
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

  // タブ切替前に現在 active の状態(カーソル / スクロール / state 全体)を控える。
  // tab.content は updateListener 経由で常に最新化されているので保存不要。
  // editorState には undo/redo スタックも含まれるので、復元時にタブごとの
  // 履歴が独立する。
  function captureActiveTabState() {
    const tab = getActiveTab();
    if (!tab || !editorView) return;
    const sel = editorView.state.selection.main;
    tab.cursorAnchor = sel.anchor;
    tab.cursorHead = sel.head;
    tab.scrollTop = editorView.scrollDOM.scrollTop;
    tab.editorState = editorView.state;
  }


  async function switchTab(targetId: TabId) {
    if (activeTabId === targetId) return;
    captureActiveTabState();
    activeTabId = targetId;
    const tab = getActiveTab();
    if (!tab) return;
    schedulePersistTabs();
    // doc / カーソル / scroll は Editor.svelte 側の $effect が反映する。
    // Typst タブなら preview / LSP、それ以外は停止しエディタだけ使う。
    if (isTypstTab(tab)) {
      const target = await ensurePreviewablePath(tab);
      if (target) {
        await Promise.all([startPreview(target), ensureLspFor(target)]);
      } else {
        await Promise.all([stopPreview(), stopLspSession()]);
      }
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
        // ファイル open で実 path に切替。無題タブだった時の仮想ファイルが
        // あれば破棄して preview を新 path に乗せ替える前準備。
        await disposeVirtualPathFor(current);
        current.path = doc.path;
        current.content = doc.content;
        current.dirty = false;
        current.cursorAnchor = 0;
        current.cursorHead = 0;
        current.scrollTop = 0;
        // ファイル内容差替なので前タブの history は捨てる
        current.editorState = null;
      } else {
        const tab: Tab = {
          id: newTabId(),
          path: doc.path,
          content: doc.content,
          dirty: false,
          cursorAnchor: 0,
          cursorHead: 0,
          scrollTop: 0,
          editorState: null,
          virtualPath: null,
        };
        tabs = [...tabs, tab];
        activeTabId = tab.id;
      }
      schedulePersistTabs();
      clearStatus();
      const newActive = getActiveTab();
      if (isTypstTab(newActive)) {
        const target = newActive ? await ensurePreviewablePath(newActive) : null;
        if (target) {
          await Promise.all([startPreview(target), ensureLspFor(target)]);
        } else {
          await Promise.all([stopPreview(), stopLspSession()]);
        }
      } else {
        await Promise.all([stopPreview(), stopLspSession()]);
      }
    } catch (e) {
      setStatus(String(e));
    }
  }

  // タブ切替のコマンド経由エントリポイント。Ctrl+Tab / Ctrl+Shift+Tab で
  // 次 / 前のタブに移動。両端は巡回(VSCode と同じ流儀)。
  async function nextTab() {
    if (tabs.length < 2 || !activeTabId) return;
    const idx = tabs.findIndex((t) => t.id === activeTabId);
    if (idx < 0) return;
    const next = tabs[(idx + 1) % tabs.length];
    await switchTab(next.id);
  }

  async function prevTab() {
    if (tabs.length < 2 || !activeTabId) return;
    const idx = tabs.findIndex((t) => t.id === activeTabId);
    if (idx < 0) return;
    const next = tabs[(idx - 1 + tabs.length) % tabs.length];
    await switchTab(next.id);
  }

  async function addEmptyTab() {
    captureActiveTabState();
    const tab = makeEmptyTab();
    tabs = [...tabs, tab];
    activeTabId = tab.id;
    schedulePersistTabs();
    // doc / カーソル / scroll は Editor.svelte 側の $effect が反映する。
    // 無題タブも Typst として扱うので、仮想ファイルを準備して preview / LSP
    // を起動する(空ドキュメントの preview が表示される)。
    const target = await ensurePreviewablePath(tab);
    if (target) {
      await Promise.all([startPreview(target), ensureLspFor(target)]);
    } else {
      await Promise.all([stopPreview(), stopLspSession()]);
    }
  }

  async function closeTab(targetId: TabId) {
    const tab = getTab(targetId);
    if (!tab) return;
    if (tab.dirty) {
      // 無題タブと既存ファイルでメッセージを変える(無題なら "名前をつけて保存")
      const isUntitled = tab.path === null;
      const message = isUntitled
        ? t("dialog.discardTabUntitled")
        : t("dialog.discardTabSaved");
      const wantsSave = await ask(message, {
        title: t("dialog.title"),
        kind: "warning",
        okLabel: t("dialog.buttonSave"),
        cancelLabel: t("dialog.buttonDontSave"),
      });
      if (wantsSave) {
        // save / saveAs は active タブを操作するので、対象タブが active で
        // ない場合は先に切り替える。
        if (activeTabId !== targetId) {
          await switchTab(targetId);
        }
        const saved = isUntitled ? await saveAs() : await save();
        // 保存ダイアログでキャンセルされた等で保存に失敗したらタブを残す
        if (!saved) return;
      }
      // wantsSave === false なら破棄して閉じる(下に進む)
    }
    const idx = tabs.findIndex((t) => t.id === targetId);
    if (idx < 0) return;
    const wasActive = activeTabId === targetId;
    // 閉じるタブが無題タブ(virtualPath を抱えている)場合は先に掃除。
    const closingTab = tabs[idx];
    if (closingTab) await disposeVirtualPathFor(closingTab);
    tabs = tabs.filter((t) => t.id !== targetId);
    schedulePersistTabs();
    if (tabs.length === 0) {
      // 全部閉じたら空タブを 1 枚自動で生成(ようこそ画面相当)。
      // 無題タブは Typst 機能の対象なので仮想ファイル準備 + preview/LSP 起動。
      const fresh = makeEmptyTab();
      tabs = [fresh];
      activeTabId = fresh.id;
      const target = await ensurePreviewablePath(fresh);
      if (target) {
        await Promise.all([startPreview(target), ensureLspFor(target)]);
      } else {
        await Promise.all([stopPreview(), stopLspSession()]);
      }
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
      pickAndInsertBibliography,
      togglePreview,
      toggleProjectView,
      toggleSidebarMode,
      newTab: addEmptyTab,
      newFromTemplate: openTemplateDialog,
      closeActiveTab: () => {
        if (activeTabId) closeTab(activeTabId);
      },
      nextTab,
      prevTab,
      openSettings,
      openKeybindings,
      openToolbarEdit,
      openCommandPalette,
    };
  }

  // ----- テンプレート選択ダイアログ関連 -----
  function openTemplateDialog() {
    templateDialogOpen = true;
  }

  // ダイアログを閉じる時に必ず firstRunDone を立てる(初回起動時の唯一の表示機会
  // を消化したことを記録、二回目以降は出さない)。失敗してもログだけ。
  function markFirstRunDone() {
    if (firstRunDone) return;
    firstRunDone = true;
    saveFirstRunDone(true).catch((e) => {
      console.warn("flags.firstRunDone save failed:", e);
    });
  }

  function onTemplateCancel() {
    templateDialogOpen = false;
    markFirstRunDone();
  }

  // テンプレを選んだ時:active タブが「実質空」なら content だけ差し替え、そう
  // でなければ新規タブを作って差し替える。
  // 「実質空」の条件:
  //   - 無題かつ未編集(従来通り)、または
  //   - 中身が空 or 空白のみ(path 有無を問わない。ディスクに既にある空 .typ
  //     にテンプレを当てたいケースをカバー、保存するまでディスクは変わらない)
  // 差替え後は dirty=true を立てるので、Ctrl+Z で元の空状態に戻せるし、
  // 望まなければ保存しなければよい。
  async function onTemplateSelect(id: string) {
    const tpl = resolveTemplate(id, resolvedLocale, paperSize);
    if (!tpl) {
      setStatus(t("status.templateMissing", { id }), "error");
      templateDialogOpen = false;
      markFirstRunDone();
      return;
    }
    captureActiveTabState();
    const current = getActiveTab();
    const isUntitledClean =
      current !== null && current.path === null && !current.dirty;
    const isEffectivelyEmpty =
      current !== null && current.content.trim() === "";
    const reuseEmpty = isUntitledClean || isEffectivelyEmpty;
    if (reuseEmpty && current) {
      current.content = tpl.body;
      current.dirty = true;
      current.cursorAnchor = 0;
      current.cursorHead = 0;
      current.scrollTop = 0;
      current.editorState = null;
      // virtualPath は流用可(無題のままなら preview 経路は同じ。path 有り
      // タブには元々 virtualPath なし)
    } else {
      const tab: Tab = {
        id: newTabId(),
        path: null,
        content: tpl.body,
        dirty: true,
        cursorAnchor: 0,
        cursorHead: 0,
        scrollTop: 0,
        editorState: null,
        virtualPath: null,
      };
      tabs = [...tabs, tab];
      activeTabId = tab.id;
    }
    schedulePersistTabs();
    templateDialogOpen = false;
    markFirstRunDone();
    clearStatus();
    // 無題タブで preview を起動(または既存 preview に乗せ替え)。
    const newActive = getActiveTab();
    if (newActive && isTypstTab(newActive)) {
      const target = await ensurePreviewablePath(newActive);
      if (target) {
        await Promise.all([startPreview(target), ensureLspFor(target)]);
      }
    }
    // テンプレ本文の差替直後は onEditorChange の dirty=true ガード(tab.content
    // === next)で no-op になり memory update が走らない。未保存でも preview に
    // 反映されるよう、ここで明示的に注入する。
    schedulePreviewMemoryUpdate();
  }

  function togglePreview() {
    previewVisible = !previewVisible;
    persistWorkspace();
  }

  // サイドバーレイアウトを α (split) と γ (tabs) で切替える。tabs に
  // した時に「Project」「Form」のどちらが見えるかは sidebarActiveTab に
  // 従う(直前に Project セクションを使っていたら project、フォーム編集中
  // なら form のまま)。プロジェクトビュー自体が非表示なら表示も同時に on
  // にする(切替えた時に何も見えないと意図不明になるため)。
  function toggleSidebarMode() {
    sidebarMode = sidebarMode === "split" ? "tabs" : "split";
    if (!projectViewVisible) projectViewVisible = true;
    persistWorkspace();
  }

  function toggleProjectView() {
    projectViewVisible = !projectViewVisible;
    // 開かれたタイミングでフォルダ未選択なら自動で Open Folder ダイアログ。
    // ただし tabs モードで form タブを表示中なら、ユーザはフォーム目的で
    // 開いただけかもしれないので Open Folder は呼ばない(Project タブを
    // クリックした時に必要なら別途呼ぶ)。
    if (projectViewVisible && !currentFolder) {
      const formTabActive =
        sidebarMode === "tabs" && sidebarActiveTab === "form";
      if (!formTabActive) openFolder();
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
      statusbarVisible,
      charCountMode,
      sidebarMode,
      sidebarSplitRatio,
      sidebarActiveTab,
    }).catch((e) => {
      // 永続化失敗はログのみ(ボタン操作はそのまま受け付ける)
      console.warn("workspace save failed:", e);
    });
  }

  function clampRatio(v: number, min: number, max: number): number {
    return Math.max(min, Math.min(max, v));
  }

  // スプリッタを掴んだら pointer capture して move/up を listen する。
  // 左:プロジェクトビューと右側全体の境界。中(sidebar):プロジェクトと
  // フォームの境界(α split モード時のみ)。右:エディタとプレビューの境界。
  function onSplitterDown(
    e: PointerEvent,
    which: "project" | "editor" | "sidebar",
  ) {
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
    } else if (splitterTarget === "sidebar") {
      // sidebar splitter は α split モード時の「サイドバー縦の比率」。
      // 上:プロジェクト / 下:フォーム の高さ比。
      if (!projectPaneEl) return;
      const rect = projectPaneEl.getBoundingClientRect();
      const ratio = (e.clientY - rect.top) / rect.height;
      sidebarSplitRatio = clampRatio(ratio, 0.1, 0.9);
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

  // ファイル毎の git status code(? / M / A / D / R / U)。
  // git repo でないフォルダの時は空 map。リフレッシュ・編集後に再読み込みする。
  let gitStatus = $state<Record<string, string>>({});

  async function loadProjectTree(folder: string) {
    try {
      projectTree = await listDirectory(folder);
      // ツリー再読込みに合わせて git status も更新。git 無しでもエラーに
      // しない(loadGitStatus が黙って空 map を返す)
      try {
        const s = await loadGitStatus(folder);
        gitStatus = s.entries;
      } catch (e) {
        gitStatus = {};
        console.warn("[git] status load failed:", e);
      }
    } catch (e) {
      projectTree = null;
      setStatus(t("status.folderLoadFailed", { error: String(e) }));
    }
  }

  async function refreshProjectTree() {
    if (!currentFolder) return;
    await loadProjectTree(currentFolder);
  }

  // 保存後など、ツリー構造は変わらないが git status だけ取り直したい時に
  // 呼ぶ軽量版。listDirectory を再実行しないので体感が良い。
  async function refreshGitStatus() {
    if (!currentFolder) return;
    try {
      const s = await loadGitStatus(currentFolder);
      gitStatus = s.entries;
    } catch (e) {
      console.warn("[git] status refresh failed:", e);
    }
  }

  // ----- プロジェクトビュー: 右クリックメニュー -----

  // 表示中の context menu 情報。null なら閉じている。
  type TreeContextMenu = {
    x: number;
    y: number;
    target: DirEntry;
  };
  let treeMenu = $state<TreeContextMenu | null>(null);

  function openTreeContextMenu(e: MouseEvent, entry: DirEntry) {
    treeMenu = { x: e.clientX, y: e.clientY, target: entry };
  }
  function closeTreeContextMenu() {
    treeMenu = null;
  }

  // 名前入力ダイアログ。新規作成 / リネーム共通で使う簡易プロンプト。
  // 表示中は promptDialog にメタを格納、submit / cancel で resolve する。
  type PromptRequest = {
    title: string;
    initialValue: string;
    placeholder: string;
    okLabel: string;
    resolve: (value: string | null) => void;
  };
  let promptDialog = $state<PromptRequest | null>(null);
  let promptInput = $state("");

  function askName(opts: {
    title: string;
    initialValue?: string;
    placeholder?: string;
    okLabel?: string;
  }): Promise<string | null> {
    return new Promise((resolve) => {
      promptInput = opts.initialValue ?? "";
      promptDialog = {
        title: opts.title,
        initialValue: promptInput,
        placeholder: opts.placeholder ?? "",
        okLabel: opts.okLabel ?? t("dialog.buttonOk"),
        resolve,
      };
    });
  }

  function submitPrompt() {
    const dlg = promptDialog;
    if (!dlg) return;
    promptDialog = null;
    dlg.resolve(promptInput.trim() || null);
  }

  function cancelPrompt() {
    const dlg = promptDialog;
    if (!dlg) return;
    promptDialog = null;
    dlg.resolve(null);
  }

  // 各アクション。エラーは status バーに出してユーザに見せる。
  async function handleNewFile() {
    const target = treeMenu?.target;
    closeTreeContextMenu();
    if (!target) return;
    // ファイルが選ばれていた場合は親ディレクトリ、フォルダなら自身を親に
    const parent = target.is_dir
      ? target.path
      : await dirname(target.path);
    const name = await askName({
      title: t("project.newFileTitle"),
      placeholder: "untitled.typ",
      okLabel: t("project.create"),
    });
    if (!name) return;
    try {
      const created = await invoke<string>("create_file", {
        parent,
        name,
      });
      await refreshProjectTree();
      // typ / テキストファイルなら開く
      await openFileAtPath(created);
    } catch (e) {
      setStatus(String(e));
    }
  }

  async function handleNewFolder() {
    const target = treeMenu?.target;
    closeTreeContextMenu();
    if (!target) return;
    const parent = target.is_dir
      ? target.path
      : await dirname(target.path);
    const name = await askName({
      title: t("project.newFolderTitle"),
      placeholder: "new-folder",
      okLabel: t("project.create"),
    });
    if (!name) return;
    try {
      await invoke("create_folder", { parent, name });
      await refreshProjectTree();
    } catch (e) {
      setStatus(String(e));
    }
  }

  async function handleRename() {
    const target = treeMenu?.target;
    closeTreeContextMenu();
    if (!target) return;
    const newName = await askName({
      title: t("project.renameTitle"),
      initialValue: target.name,
      okLabel: t("project.rename"),
    });
    if (!newName || newName === target.name) return;
    try {
      const newPath = await invoke<string>("rename_path", {
        oldPath: target.path,
        newName,
      });
      // 開いているタブの path を追従させる(リネーム前の path を持つ
      // タブがあれば差し替え、preview / LSP も乗せ替える)
      const t0 = tabs.find((t) => t.path === target.path);
      if (t0) {
        t0.path = newPath;
        if (t0.id === activeTabId && isTypstTab(t0)) {
          await Promise.all([startPreview(newPath), ensureLspFor(newPath)]);
        }
      }
      await refreshProjectTree();
      schedulePersistTabs();
    } catch (e) {
      setStatus(String(e));
    }
  }

  async function handleDelete() {
    const target = treeMenu?.target;
    closeTreeContextMenu();
    if (!target) return;
    const confirmed = await ask(
      t("project.deleteConfirm", { name: target.name }),
      {
        title: t("dialog.title"),
        kind: "warning",
        okLabel: t("project.delete"),
        cancelLabel: t("dialog.buttonDontSave"),
      },
    );
    if (!confirmed) return;
    try {
      await invoke("delete_path", { path: target.path });
      // 削除されたファイルがタブで開いていたらそのタブを閉じる
      const orphan = tabs.find(
        (t) => t.path && (t.path === target.path ||
          t.path.startsWith(target.path + "/")),
      );
      if (orphan) {
        // 削除済みなので dirty 確認をスキップ。closeTab を直接呼ぶと
        // dirty チェックに引っかかるため、自前で外す。
        const idx = tabs.findIndex((tt) => tt.id === orphan.id);
        if (idx >= 0) {
          const wasActive = activeTabId === orphan.id;
          await disposeVirtualPathFor(orphan);
          tabs = tabs.filter((tt) => tt.id !== orphan.id);
          if (wasActive) {
            if (tabs.length === 0) {
              const fresh = makeEmptyTab();
              tabs = [fresh];
              activeTabId = fresh.id;
            } else {
              activeTabId = tabs[Math.max(0, idx - 1)].id;
            }
          }
          schedulePersistTabs();
        }
      }
      await refreshProjectTree();
    } catch (e) {
      setStatus(String(e));
    }
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
      setStatus(t("status.openExternalFailed", { error: String(e) }));
    }
  }

  async function runCommand(id: CommandId) {
    const def = COMMANDS[id];
    if (def.needsEditor && !editorView) return;
    await def.run(commandContext());
  }

  // コマンドに割り当てられているキーバインド一覧を返す。同じコマンドに
  // 複数キーを bind するケース(Ctrl+Tab と Ctrl+PageDown を両方など)に
  // 対応するため常に配列を返す。
  // override(設定で書き換えた値)は単一 string のみ受ける(配列で書き換える
  // のは Phase 2 の設定 UI で対応予定)。override が入っていれば
  // デフォルトキー全体を上書きする。
  function effectiveKeys(id: CommandId): string[] {
    const override = keybindings[id];
    if (typeof override === "string" && override.length > 0) return [override];
    const def = COMMANDS[id].defaultKey;
    if (Array.isArray(def)) return def;
    if (typeof def === "string") return [def];
    return [];
  }

  // "Mod-Shift-b" 形式のキー指定を表示用 ("Ctrl+Shift+B") に整える。
  function displayKey(spec: string): string {
    return spec.replaceAll("Mod", "Ctrl").replaceAll("-", "+");
  }

  // KeyboardEvent を物理キー名(e.code)に正規化する。GTK key theme が
  // Emacs になっている環境では e.key が ArrowLeft 等に化けるため、物理
  // キーを見るのが確実(KeyB → "b"、Digit1 → "1"、それ以外は素のまま)。
  function normalizedKey(e: KeyboardEvent): string {
    const code = e.code;
    if (code.startsWith("Key")) return code.slice(3).toLowerCase();
    if (code.startsWith("Digit")) return code.slice(5);
    return code;
  }

  // KeyboardEvent が "Mod-b" 形式の指定にマッチするかを判定。
  // Mod は Ctrl/Cmd 両対応。最終キーは大文字小文字を無視。e.key と e.code
  // 由来の正規化キー双方を試して、GTK 変換 / レイアウト両対応する。
  function matchKey(e: KeyboardEvent, spec: string): boolean {
    const parts = spec.split("-");
    const last = parts[parts.length - 1].toLowerCase();
    const wantMod = parts.includes("Mod");
    const wantShift = parts.includes("Shift");
    const wantAlt = parts.includes("Alt");
    const evKey = e.key.toLowerCase();
    const evCode = normalizedKey(e).toLowerCase();
    if (evKey !== last && evCode !== last) return false;
    const hasMod = e.ctrlKey || e.metaKey;
    if (wantMod !== hasMod) return false;
    if (wantShift !== e.shiftKey) return false;
    if (wantAlt !== e.altKey) return false;
    return true;
  }

  function buttonTitle(id: CommandId): string {
    const def = COMMANDS[id];
    const keys = effectiveKeys(id);
    const label = t(def.labelKey);
    // ホバーヒントは先頭のキーだけ出す(複数あっても 1 個だけ表示で十分)
    return keys.length > 0 ? `${label} (${displayKey(keys[0])})` : label;
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
        filters: filtersImage(),
      });
      if (typeof selected !== "string") return;
      const rel = await toRelativePath(selected);
      insertImage(editorView, rel);
    } catch (e) {
      setStatus(String(e));
    }
  }

  // 設定ファイル(settings.json)を Yuhitsu 自身のタブで開く。
  // 編集 → Ctrl+S で保存 → save() 内で settingsPath と一致したら自動で
  // reloadSettings が走り、設定変更が即時反映される。
  async function openSettings() {
    try {
      const path = await invoke<string>("get_settings_path");
      settingsPath = path; // 保存時の自動再読み込み判定に使う
      await openFileAtPath(path);
    } catch (e) {
      setStatus(String(e));
    }
  }

  // キーバインド設定ダイアログを開く。中で更新があった時は onUpdate 経由で
  // ここに伝わる(keybindings state を更新 + 永続化)。
  function openKeybindings() {
    keybindingsDialogOpen = true;
  }
  async function applyKeybindingsUpdate(next: Record<string, string>) {
    keybindings = next;
    try {
      await saveKeybindings(next);
    } catch (e) {
      setStatus(t("status.settingsSaveFailed", { error: String(e) }));
    }
  }

  // ツールバー編集ダイアログ。並び替え / 削除 / 追加 / プリセット適用を
  // 行うとここに通知され、toolbarItems を更新 + Tauri Store に永続化する。
  function openToolbarEdit() {
    toolbarEditDialogOpen = true;
  }

  function openCommandPalette() {
    commandPaletteOpen = true;
  }
  function onCommandPaletteSelect(id: CommandId) {
    commandPaletteOpen = false;
    runCommand(id);
  }
  async function applyToolbarUpdate(next: ToolbarItem[]) {
    toolbarItems = next;
    try {
      await saveToolbarItems(next);
    } catch (e) {
      setStatus(t("status.settingsSaveFailed", { error: String(e) }));
    }
  }

  // 参考文献ファイル(.bib / .yml)を選んでドキュメント末尾に
  // #bibliography("...") を挿入する。Typst は両形式をネイティブ対応。
  async function pickAndInsertBibliography() {
    if (!editorView) return;
    try {
      const selected = await openDialog({
        multiple: false,
        filters: [
          { name: t("filter.bibliography"), extensions: ["bib", "yml", "yaml"] },
        ],
      });
      if (typeof selected !== "string") return;
      const rel = await toRelativePath(selected);
      insertBibliography(editorView, rel);
    } catch (e) {
      setStatus(String(e));
    }
  }

  function basename(p: string | null): string {
    if (!p) return t("tab.untitled");
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
    // 修飾キーを伴わないキー入力は基本エディタ本体に渡す(IME / vim 等)。
    // ただし F1-F24 のような function キー単独は通常テキスト入力に使われ
    // ないため、コマンドカタログ側のキーバインド(例: open-command-palette
    // = F1)を発動させる経路は確保する。
    const noMod = !e.ctrlKey && !e.metaKey && !e.altKey;
    const isFunctionKey = /^F\d{1,2}$/.test(e.key);
    if (noMod && !isFunctionKey) return;
    for (const id of COMMAND_IDS) {
      const keys = effectiveKeys(id);
      if (keys.length === 0) continue;
      if (!keys.some((k) => matchKey(e, k))) continue;
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
    let unlistenSystemTheme: (() => void) | undefined;
    let unlistenFocus: (() => void) | undefined;
    (async () => {
      const win = getCurrentWindow();
      unlisten = await win.onCloseRequested(async (event) => {
        if (!dirty) {
          // dirty でなくても preview / LSP の subprocess は止めたい
          await persistTabs();
          await stopPreview();
          await stopLspSession();
          return;
        }
        // preventDefault は同期で呼ぶ必要があるため、await より前に必ず呼ぶ
        event.preventDefault();
        const ok = await ask(t("dialog.exitApp"), {
          title: t("dialog.title"),
          kind: "warning",
        });
        if (ok) {
          // hot exit のため、無題タブの未保存内容を含めて最終 flush
          await persistTabs();
          await stopPreview();
          await stopLspSession();
          await win.destroy();
        }
      });
      // 設定の読み込みは hydration 後に。Tauri Store は async API なので
      // onMount で起動時に1回読む。失敗時はデフォルトを使う。
      // JSON 構文エラーがある場合はステータスバーで通知して、デフォルト
      // で起動を続行する(設定の編集 UI が無い間は外部編集が前提なので、
      // 起動できなくなるのは避ける)。
      const initialJsonError = await validateSettingsJson();
      if (initialJsonError) {
        setStatus(t("status.settingsJsonError", { error: initialJsonError }));
        settingsErrorActive = true;
      }
      try {
        const settings = await loadSettings();
        editorMode = settings.editor.mode;
        themeMode = settings.appearance.theme;
        applyTheme(themeMode);
        localeMode = settings.appearance.locale;
        setLocale(resolveLocale(localeMode));
        paperSize = settings.document.paperSize;
        firstRunDone = settings.flags.firstRunDone;
        toolbarItems = settings.toolbar.items;
        keybindings = settings.keybindings;
        previewVisible = settings.workspace.previewVisible;
        editorPaneRatio = settings.workspace.editorPaneRatio;
        projectViewVisible = settings.workspace.projectViewVisible;
        projectPaneRatio = settings.workspace.projectPaneRatio;
        currentFolder = settings.workspace.currentFolder;
        statusbarVisible = settings.workspace.statusbarVisible;
        charCountMode = settings.workspace.charCountMode;
        sidebarMode = settings.workspace.sidebarMode;
        sidebarSplitRatio = settings.workspace.sidebarSplitRatio;
        sidebarActiveTab = settings.workspace.sidebarActiveTab;
        // settings.json の絶対パスを起動時に取っておく。これがないと、
        // ユーザが「設定を開く」コマンドを通さず別経路(プロジェクトツリー
        // 等)で settings.json を Yuhitsu のタブに開いた場合、save() 内の
        // 「保存先 == settingsPath なら自動 reloadSettings」ガードが
        // 動かず、保存しても設定が即時反映されない。
        try {
          settingsPath = await invoke<string>("get_settings_path");
        } catch (e) {
          console.warn("get_settings_path on startup failed:", e);
        }
        // 前回開いていたフォルダを復元(失敗しても致命的でない)
        if (currentFolder) {
          await loadProjectTree(currentFolder);
        }
        // 起動時のタブ状態を復元(hot exit 経由)。前回開いていたファイル
        // タブと無題タブを再生する。復元できれば templateDialog は出さない。
        const restored = await restoreTabs();
        if (restored) {
          const active = getActiveTab();
          if (active && isTypstTab(active)) {
            const target = await ensurePreviewablePath(active);
            if (target) {
              void Promise.all([startPreview(target), ensureLspFor(target)]);
            }
          }
        } else if (!firstRunDone) {
          // 初回起動はテンプレ選択ダイアログを自動表示
          templateDialogOpen = true;
        } else {
          // 復元対象なし(全タブ閉じた状態で終了した、など)。デフォルト
          // 空タブで preview を立ち上げる。
          const active = getActiveTab();
          if (active && isTypstTab(active)) {
            const target = await ensurePreviewablePath(active);
            if (target) {
              void Promise.all([startPreview(target), ensureLspFor(target)]);
            }
          }
        }
      } catch (e) {
        // JSON エラーは事前 validate で見ているので、ここに来るのは
        // 別の予期せぬエラー(I/O 等)。状態は表示してデフォルト続行する。
        setStatus(t("status.settingsLoadFailed", { error: String(e) }));
        settingsErrorActive = true;
      }
      // "auto" 中に OS のテーマ設定が変わったら追従する
      const mql = window.matchMedia("(prefers-color-scheme: light)");
      const onSystemThemeChange = () => {
        if (themeMode === "auto") applyTheme("auto");
      };
      mql.addEventListener("change", onSystemThemeChange);
      unlistenSystemTheme = () =>
        mql.removeEventListener("change", onSystemThemeChange);
      // 外部エディタで settings.json を編集 → Yuhitsu にフォーカス復帰した
      // タイミングで設定を再読み込みする(設定 UI ができるまでの暫定)
      const onWindowFocus = () => {
        reloadSettings();
      };
      window.addEventListener("focus", onWindowFocus);
      unlistenFocus = () =>
        window.removeEventListener("focus", onWindowFocus);
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
      unlistenSystemTheme?.();
      unlistenFocus?.();
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
          aria-label={t(def.labelKey)}
          title={buttonTitle(def.id)}
          disabled={def.needsEditor && !editorView}
          onclick={() => runCommand(def.id)}
        >
          <Icon size={18} />
        </button>
      {/if}
    {/each}
  </header>

  <div class="workspace" bind:this={workspaceEl}>
    {#if projectViewVisible}
      <aside
        class="project-pane"
        class:tabs-mode={sidebarMode === "tabs"}
        style:flex={`0 0 ${projectPaneRatio * 100}%`}
        bind:this={projectPaneEl}
      >
        {#if sidebarMode === "tabs"}
          <div class="sidebar-tabbar" role="tablist">
            <button
              class="sidebar-tab"
              class:active={sidebarActiveTab === "project"}
              role="tab"
              aria-selected={sidebarActiveTab === "project"}
              onclick={() => {
                sidebarActiveTab = "project";
                persistWorkspace();
              }}>{t("sidebar.tabProject")}</button
            >
            <button
              class="sidebar-tab"
              class:active={sidebarActiveTab === "form"}
              role="tab"
              aria-selected={sidebarActiveTab === "form"}
              onclick={() => {
                sidebarActiveTab = "form";
                persistWorkspace();
              }}>{t("sidebar.tabForm")}</button
            >
          </div>
        {/if}

        {#if sidebarMode === "split" || sidebarActiveTab === "project"}
          <div
            class="sidebar-section project-section"
            style:flex={sidebarMode === "split"
              ? `0 0 calc(${sidebarSplitRatio * 100}% - 3px)`
              : "1 1 0"}
          >
            <div class="project-header">
              {#if currentFolder}
                <span class="folder-name" title={currentFolder}
                  >{basename(currentFolder)}</span
                >
                <button
                  class="header-action"
                  title={t("command.openFolder")}
                  onclick={openFolder}>{t("project.change")}</button
                >
                <button
                  class="header-action"
                  title={t("project.refreshTooltip")}
                  onclick={refreshProjectTree}>{t("project.refresh")}</button
                >
              {:else}
                <span class="folder-name muted">{t("project.noFolder")}</span>
                <button
                  class="header-action"
                  title={t("command.openFolder")}
                  onclick={openFolder}>{t("project.open")}</button
                >
              {/if}
            </div>
            <div
              class="project-body"
              oncontextmenu={(e) => {
                // 既にツリー行で停止されていなければ、ルートフォルダ自身を
                // ターゲットにメニューを出す(空白部分での右クリック対応)。
                if (!projectTree) return;
                e.preventDefault();
                openTreeContextMenu(e, projectTree);
              }}
              role="presentation"
            >
              {#if projectTree && projectTree.children}
                <ProjectTree
                  entries={projectTree.children}
                  activePath={path}
                  onOpenFile={selectFromTree}
                  expanded={projectExpanded}
                  onToggleExpanded={toggleProjectExpanded}
                  onContextMenu={openTreeContextMenu}
                  {gitStatus}
                />
              {:else if currentFolder}
                <div class="placeholder">{t("placeholder.loading")}</div>
              {:else}
                <div class="placeholder">
                  {t("project.openHint")}
                </div>
              {/if}
            </div>
          </div>
        {/if}

        {#if sidebarMode === "split"}
          <div
            class="splitter horizontal"
            class:dragging={splitterTarget === "sidebar"}
            role="separator"
            aria-orientation="horizontal"
            aria-label={t("splitter.sidebarBoundary")}
            onpointerdown={(e) => onSplitterDown(e, "sidebar")}
            onpointermove={onSplitterMove}
            onpointerup={onSplitterUp}
            onpointercancel={onSplitterUp}
          ></div>
        {/if}

        {#if sidebarMode === "split" || sidebarActiveTab === "form"}
          <div
            class="sidebar-section form-section"
            style:flex="1 1 0"
          >
            <FormPanel
              doc={getActiveTab() ? content : null}
              isTypst={isTypstTab(getActiveTab())}
              spec={activeFormSpec}
              locale={resolvedLocale}
              onApply={onFormApply}
            />
          </div>
        {/if}
      </aside>

      <div
        class="splitter"
        class:dragging={splitterTarget === "project"}
        role="separator"
        aria-orientation="vertical"
        aria-label={t("splitter.projectBoundary")}
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
            title={tab.path ?? t("tab.untitled")}
            data-tab-id={tab.id}
            onpointerdown={(e) => onTabPointerDown(e, tab.id)}
            onpointermove={onTabPointerMove}
            onpointerup={(e) => onTabPointerUp(e, tab.id)}
            onpointercancel={(e) => onTabPointerUp(e, tab.id)}
          >
            <span class="tab-label">
              <span class="tab-name"
                >{tab.path ? basename(tab.path) : t("tab.untitled")}</span
              >
              {#if tab.dirty}<span class="tab-dirty" aria-hidden="true">●</span>{/if}
            </span>
            <button
              class="tab-close"
              aria-label={t("command.closeTab")}
              title={t("command.closeTab")}
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
          aria-label={t("command.newTab")}
          title={t("command.newTab")}
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
            externalState={activeEditorState}
            mode={editorMode}
            languageMode={isTypstTab(getActiveTab()) ? "typst" : "plain"}
            {lspClient}
            filePath={editorFilePath}
            onChange={onEditorChange}
            onCursorChange={(info) => (cursor = info)}
            onReady={onEditorReady}
            onTeardown={onEditorTeardown}
            onValueApplied={onEditorValueApplied}
          />
        {:else}
          <div class="placeholder">{t("placeholder.editorLoading")}</div>
        {/if}
      </div>

      {#if previewVisible}
        <div
          class="splitter"
          class:dragging={splitterTarget === "editor"}
          role="separator"
          aria-orientation="vertical"
          aria-label={t("splitter.previewBoundary")}
          onpointerdown={(e) => onSplitterDown(e, "editor")}
          onpointermove={onSplitterMove}
          onpointerup={onSplitterUp}
          onpointercancel={onSplitterUp}
        ></div>

        <div class="preview-pane">
          {#if previewStatus === "idle"}
            <div class="placeholder">
              {t("placeholder.previewIdle")}
            </div>
          {:else if previewStatus === "starting"}
            <div class="placeholder">{t("placeholder.previewLoading")}</div>
          {:else if previewStatus === "error"}
            <div class="placeholder error">
              {t("placeholder.previewFailed")}
              <br />
              <small>{previewError}</small>
            </div>
          {:else if previewStatus === "ready" && previewSrc}
            <iframe class="preview-frame" title={t("preview.frameTitle")} src={previewSrc}
            ></iframe>
          {/if}
        </div>
      {/if}
      </div>
    </div>
  </div>

  {#if statusbarVisible}
    <!--
      ステータスバー(画面下部、設定で on/off 切替)。
      左:status メッセージ / 右:行・列、文字数。
      ワードカウント(Typst コンパイル後の本文字数)は Phase 2 以降に予定。
    -->
    <footer class="statusbar">
      <span class="statusbar-message">
        {#if status}
          <span class="status status-{statusKind}">{status}</span>
        {/if}
      </span>
      <span class="statusbar-counters">
        {#if cursor}
          {@const totalShown =
            charCountMode === "all" ? cursor.total : cursor.totalNoWs}
          {@const selectedShown =
            charCountMode === "all" ? cursor.selected : cursor.selectedNoWs}
          <span class="counter" data-slot="line"
            >{t("statusbar.lineCol", {
              line: String(cursor.line),
              col: String(cursor.col),
            })}</span
          >
          <span class="counter" data-slot="char"
            >{selectedShown > 0
              ? t("statusbar.charsSelected", {
                  selected: String(selectedShown),
                  total: String(totalShown),
                })
              : t("statusbar.chars", { total: String(totalShown) })}</span
          >
        {/if}
        <!-- ワードカウント(Typst コンパイル後)は Phase 2 で実装する空スロット -->
        <span class="counter" data-slot="word"></span>
      </span>
    </footer>
  {/if}

  {#if templateDialogOpen}
    <TemplateDialog
      templates={allTemplates}
      locale={resolvedLocale}
      onSelect={onTemplateSelect}
      onCancel={onTemplateCancel}
    />
  {/if}

  {#if keybindingsDialogOpen}
    <KeybindingsDialog
      {keybindings}
      onUpdate={applyKeybindingsUpdate}
      onClose={() => (keybindingsDialogOpen = false)}
    />
  {/if}

  {#if toolbarEditDialogOpen}
    <ToolbarEditDialog
      items={toolbarItems}
      onUpdate={applyToolbarUpdate}
      onClose={() => (toolbarEditDialogOpen = false)}
    />
  {/if}

  {#if commandPaletteOpen}
    <CommandPalette
      {keybindings}
      editorAvailable={editorView !== null}
      onSelect={onCommandPaletteSelect}
      onClose={() => (commandPaletteOpen = false)}
    />
  {/if}

  {#if treeMenu}
    {@const isRoot = treeMenu.target.path === currentFolder}
    <!-- 画面全体の透明オーバーレイで外側クリックを拾う(menu を閉じる) -->
    <div
      class="ctx-overlay"
      onclick={closeTreeContextMenu}
      onkeydown={(e) => e.key === "Escape" && closeTreeContextMenu()}
      role="presentation"
    ></div>
    <div
      class="ctx-menu"
      style:left={`${treeMenu.x}px`}
      style:top={`${treeMenu.y}px`}
      role="menu"
    >
      <button class="ctx-item" onclick={handleNewFile}
        >{t("project.newFile")}</button
      >
      <button class="ctx-item" onclick={handleNewFolder}
        >{t("project.newFolder")}</button
      >
      {#if !isRoot}
        <!-- ルートフォルダ自身のリネーム / 削除はうっかり事故の元なので
             非表示にする。プロジェクトを移したい時は「フォルダを開く」を使う -->
        <div class="ctx-sep"></div>
        <button class="ctx-item" onclick={handleRename}
          >{t("project.rename")}</button
        >
        <button class="ctx-item danger" onclick={handleDelete}
          >{t("project.delete")}</button
        >
      {/if}
    </div>
  {/if}

  {#if promptDialog}
    <div class="prompt-overlay" role="presentation" onclick={cancelPrompt}></div>
    <div class="prompt-dialog" role="dialog" aria-modal="true">
      <div class="prompt-title">{promptDialog.title}</div>
      <input
        class="prompt-input"
        type="text"
        bind:value={promptInput}
        placeholder={promptDialog.placeholder}
        autofocus
        onkeydown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            submitPrompt();
          } else if (e.key === "Escape") {
            e.preventDefault();
            cancelPrompt();
          }
        }}
      />
      <div class="prompt-actions">
        <button class="prompt-btn" onclick={cancelPrompt}
          >{t("dialog.buttonCancel")}</button
        >
        <button class="prompt-btn primary" onclick={submitPrompt}
          >{promptDialog.okLabel}</button
        >
      </div>
    </div>
  {/if}
</div>

<style>
  :global(html, body) {
    margin: 0;
    height: 100%;
    font-family: system-ui, sans-serif;
    background: var(--bg-base);
    color: var(--text-primary);
  }

  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  /* プロジェクトビューの右クリックメニュー */
  .ctx-overlay {
    position: fixed;
    inset: 0;
    z-index: 200;
    background: transparent;
  }
  .ctx-menu {
    position: fixed;
    z-index: 201;
    min-width: 160px;
    padding: 4px 0;
    background: var(--bg-elevated-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
    color: var(--text-primary);
    font-size: 13px;
  }
  .ctx-item {
    display: block;
    width: 100%;
    padding: 6px 12px;
    background: transparent;
    border: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
    line-height: 1.4;
  }
  .ctx-item:hover {
    background: var(--bg-elevated-3);
  }
  .ctx-item.danger {
    color: var(--status-error-strong);
  }
  .ctx-sep {
    height: 1px;
    margin: 4px 0;
    background: var(--border);
  }

  /* 名前入力プロンプト(新規 / リネーム共通) */
  .prompt-overlay {
    position: fixed;
    inset: 0;
    z-index: 300;
    background: rgba(0, 0, 0, 0.4);
  }
  .prompt-dialog {
    position: fixed;
    z-index: 301;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    min-width: 320px;
    padding: 16px;
    background: var(--bg-elevated-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    color: var(--text-primary);
  }
  .prompt-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-strong);
    margin-bottom: 8px;
  }
  .prompt-input {
    width: 100%;
    padding: 6px 8px;
    background: var(--bg-base);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font: inherit;
    font-size: 13px;
    box-sizing: border-box;
  }
  .prompt-input:focus {
    outline: 1px solid var(--accent-strong);
    outline-offset: -1px;
  }
  .prompt-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 12px;
  }
  .prompt-btn {
    padding: 6px 14px;
    background: var(--bg-elevated-3);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 12px;
    cursor: pointer;
  }
  .prompt-btn:hover {
    background: var(--bg-elevated-1);
  }
  .prompt-btn.primary {
    background: var(--accent-bg-subtle);
    border-color: var(--accent-strong);
    color: var(--text-strong);
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    /* gap は wrap した時の行間にも効く。横方向 8px / 縦 6px で詰まりすぎない */
    gap: 6px 8px;
    padding: 8px 12px;
    background: var(--bg-elevated-2);
    border-bottom: 1px solid var(--border);
  }

  .toolbar button {
    padding: 4px 10px;
    background: var(--border);
    color: var(--text-primary);
    border: 1px solid var(--border-strong);
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
  }

  .toolbar button:hover {
    background: var(--border-strong);
  }

  .toolbar button:disabled {
    color: var(--text-disabled);
    background: var(--bg-elevated-3);
    border-color: var(--border);
    cursor: not-allowed;
  }

  .toolbar button:disabled:hover {
    background: var(--bg-elevated-3);
  }

  .toolbar-divider {
    /* wrap 時に行高に合わせて伸びると違和感があるので、center + 固定高さに */
    width: 1px;
    height: 18px;
    align-self: center;
    background: var(--border);
    margin: 0 4px;
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

  .statusbar {
    flex-shrink: 0;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 0 10px;
    background: var(--bg-elevated-1);
    border-top: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-muted);
    user-select: none;
  }

  .statusbar-message {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .statusbar-counters {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
  }

  .counter {
    /* Phase 2 以降で実装するスロット。中身が空なら何も占めない */
  }
  .counter:empty {
    display: none;
  }

  .status {
    font-size: 11px;
  }

  .status-error {
    color: var(--status-error);
  }

  .status-info {
    color: var(--status-success);
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
    background: var(--bg-elevated-1);
    border-bottom: 1px solid var(--bg-elevated-2);
    overflow-x: auto;
    overflow-y: hidden;
    flex-shrink: 0;
    height: 32px;
  }

  .tab {
    display: flex;
    align-items: center;
    border-right: 1px solid var(--bg-elevated-2);
    background: var(--bg-elevated-2);
    min-width: 110px;
    max-width: 220px;
    flex-shrink: 0;
    cursor: pointer;
    user-select: none;
    -webkit-user-select: none;
    touch-action: none;
  }

  .tab.active {
    /* active タブはエディタ面と同色(地続き)に。これだけだとダークで
       "周囲より暗くなって選択中に見えない" 違和感が出るので、上端に
       accent 線を 2px 入れて選択中を明示する(VSCode 流儀)。
       ライトでも同じ流儀で一貫(active = エディタと同色 + 上 accent 線)。 */
    background: var(--bg-base);
    box-shadow: inset 0 2px 0 var(--accent);
  }

  .tab.dragging {
    opacity: 0.4;
  }

  .tab.drag-over {
    box-shadow: inset 2px 0 0 var(--accent);
  }

  .tab-label {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    color: var(--text-muted);
    font-size: 12px;
    height: 100%;
    overflow: hidden;
    user-select: none;
  }

  .tab.active .tab-label {
    color: var(--text-strong);
  }

  .tab-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: none;
  }

  .tab-dirty {
    color: var(--accent);
    font-size: 10px;
    flex-shrink: 0;
  }

  .tab-close {
    padding: 0 8px;
    background: transparent;
    border: none;
    color: var(--text-faint);
    cursor: pointer;
    font-size: 14px;
    height: 100%;
    line-height: 1;
  }

  .tab-close:hover {
    color: var(--text-strong);
    background: var(--border);
  }

  .tab-new {
    padding: 0 12px;
    height: 100%;
    background: transparent;
    border: none;
    color: var(--text-faint);
    cursor: pointer;
    font-size: 16px;
    flex-shrink: 0;
  }

  .tab-new:hover {
    background: var(--bg-elevated-2);
    color: var(--text-strong);
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
    background: var(--bg-elevated-2);
  }

  .project-pane {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg-elevated-1);
    border-right: 1px solid var(--bg-elevated-2);
  }

  .project-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--bg-elevated-2);
    font-size: 12px;
  }

  .folder-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-secondary);
    font-weight: 600;
  }

  .folder-name.muted {
    color: var(--text-faint);
    font-weight: 400;
  }

  .header-action {
    padding: 2px 8px;
    background: var(--border);
    color: var(--text-secondary);
    border: 1px solid var(--border-strong);
    border-radius: 3px;
    font-size: 11px;
    cursor: pointer;
  }

  .header-action:hover {
    background: var(--border-strong);
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
    background: var(--bg-elevated-2);
    cursor: col-resize;
    user-select: none;
    touch-action: none;
    transition: background 100ms ease;
  }

  .splitter.horizontal {
    /* α split モード時、サイドバー内のプロジェクト/フォーム境界 */
    flex: 0 0 6px;
    width: 100%;
    align-self: auto;
    cursor: row-resize;
  }

  .splitter:hover {
    background: var(--border-strong);
  }

  .splitter.dragging {
    background: var(--accent);
  }

  .sidebar-tabbar {
    display: flex;
    align-items: stretch;
    background: var(--bg-elevated-1);
    border-bottom: 1px solid var(--bg-elevated-2);
  }

  .sidebar-tab {
    flex: 1 1 0;
    padding: 6px 8px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
  }

  .sidebar-tab:hover {
    background: var(--bg-elevated-2);
  }

  .sidebar-tab.active {
    color: var(--text-primary);
    border-bottom-color: var(--accent);
  }

  .sidebar-section {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }

  /* tabs モード時、表示中タブのセクションは縦いっぱい使う(0 basis で
     auto による intrinsic 膨張を避ける) */
  .project-pane.tabs-mode .sidebar-section {
    flex: 1 1 0;
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
    height: 100%;
  }

  .placeholder {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-disabled);
    font-size: 13px;
    padding: 16px;
    text-align: center;
  }

  .placeholder.error {
    color: var(--status-error);
  }

  .placeholder small {
    display: block;
    margin-top: 8px;
    color: var(--text-faint);
    font-size: 11px;
    word-break: break-all;
  }
</style>
