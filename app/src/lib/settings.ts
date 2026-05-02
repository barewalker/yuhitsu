import { LazyStore } from "@tauri-apps/plugin-store";
import { invoke } from "@tauri-apps/api/core";
import {
  COMMAND_IDS,
  getDefaultToolbarItems,
  isCommandId,
  type CommandId,
  type ToolbarItem,
} from "$lib/commands";

/**
 * Yuhitsu の永続設定スキーマ。
 *
 * Phase 1 ではエディタモード・ツールバー配置・キーバインドを実装する。
 * 将来 AI 連携で必要になる設定領域(プロバイダ・API キーなど)も
 * 最初から型として確保しておき、設定ファイルの構造を後から
 * 組み替える必要をなくす。
 */
export type EditorMode = "default" | "vim" | "emacs";

export type ThemeMode = "auto" | "light" | "dark";

/** UI / テンプレートカード等の表示言語。"auto" は navigator.language から推測、
    未対応なら "en" にフォールバックする。当面は ja/en の 2 言語のみ対応。 */
export type LocaleMode = "auto" | "ja" | "en";

export type AppearanceSettings = {
  /** "auto" は OS の prefers-color-scheme に追従、"light"/"dark" は固定 */
  theme: ThemeMode;
  /** UI とテンプレ表示の言語選択。テンプレート本文は ja.typ/en.typ を切替 */
  locale: LocaleMode;
};

/** ドキュメント生成系の設定。テンプレ展開時の用紙、将来のフォント等。 */
export type PaperSize = "auto" | "a4" | "letter" | "b5";

export type DocumentSettings = {
  /** "auto" は locale から推測(ja→a4、en→letter)、明示指定すれば優先 */
  paperSize: PaperSize;
};

/** 一過性のフラグ群(初回起動済み等)。設定 UI に出さない内部状態。 */
export type FlagsSettings = {
  /** 起動時テンプレ選択ダイアログを 1 回でも消化したか */
  firstRunDone: boolean;
};

export type ToolbarSettings = {
  /** ツールバー上の項目並び。コマンド ID または "divider" */
  items: ToolbarItem[];
};

/** キーバインドの override。空 / 未指定なら commands.ts の defaultKey を使う */
export type KeybindingsSettings = Partial<Record<CommandId, string>>;

export type WorkspaceSettings = {
  /** プレビューペインを表示するか */
  previewVisible: boolean;
  /** エディタペインの「プレビュー含む右側」全体に対する比率 0..1 */
  editorPaneRatio: number;
  /** プロジェクトビュー(サイドバー)を表示するか */
  projectViewVisible: boolean;
  /** プロジェクトビュー(サイドバー)の workspace 幅に対する比率 0..1 */
  projectPaneRatio: number;
  /** 最後に開いていたプロジェクトフォルダ。次回起動時に自動で開き直す */
  currentFolder: string | null;
  /** ステータスバー(画面下部、行数/文字数/ワードカウント等を表示)を表示するか。
      標準は off。行数等の実装は Phase 2 以降で追加する仕込みだけ用意してある */
  statusbarVisible: boolean;
};

export type Settings = {
  editor: {
    mode: EditorMode;
  };
  appearance: AppearanceSettings;
  document: DocumentSettings;
  flags: FlagsSettings;
  toolbar: ToolbarSettings;
  keybindings: KeybindingsSettings;
  workspace: WorkspaceSettings;
  ai: {
    [key: string]: unknown;
  };
};

const DEFAULT_SETTINGS: Settings = {
  editor: {
    mode: "default",
  },
  appearance: {
    theme: "auto",
    locale: "auto",
  },
  document: {
    paperSize: "auto",
  },
  flags: {
    firstRunDone: false,
  },
  toolbar: {
    items: getDefaultToolbarItems(),
  },
  keybindings: {},
  workspace: {
    previewVisible: true,
    editorPaneRatio: 0.5,
    projectViewVisible: false,
    projectPaneRatio: 0.18,
    currentFolder: null,
    statusbarVisible: false,
  },
  ai: {},
};

const STORE_FILE = "settings.json";

const store = new LazyStore(STORE_FILE);

function isEditorMode(value: unknown): value is EditorMode {
  return value === "default" || value === "vim" || value === "emacs";
}

function isThemeMode(value: unknown): value is ThemeMode {
  return value === "auto" || value === "light" || value === "dark";
}

function isLocaleMode(value: unknown): value is LocaleMode {
  return value === "auto" || value === "ja" || value === "en";
}

function isPaperSize(value: unknown): value is PaperSize {
  return (
    value === "auto" || value === "a4" || value === "letter" || value === "b5"
  );
}

function isToolbarItem(value: unknown): value is ToolbarItem {
  return value === "divider" || isCommandId(value);
}

function parseToolbarItems(raw: unknown): ToolbarItem[] {
  if (!Array.isArray(raw)) return getDefaultToolbarItems();
  let filtered = raw.filter(isToolbarItem);
  if (filtered.length === 0) filtered = getDefaultToolbarItems();
  // Sprint 3 後半で追加された新コマンドを既存設定にも自動反映。
  // 意図的に削除したいユーザは設定 UI 完成後に再度外す前提。
  if (!filtered.includes("toggle-preview")) {
    filtered = [...filtered, "divider", "toggle-preview"];
  }
  if (!filtered.includes("toggle-project-view")) {
    // toggle-preview の手前に挟むのが自然(左パネル切替→右パネル切替の並び)
    const idx = filtered.lastIndexOf("toggle-preview");
    if (idx >= 0) {
      filtered = [
        ...filtered.slice(0, idx),
        "toggle-project-view",
        ...filtered.slice(idx),
      ];
    } else {
      filtered = [...filtered, "divider", "toggle-project-view"];
    }
  }
  if (!filtered.includes("open-folder")) {
    // open-file の直後に open-folder を追加(ファイル系の並び)
    const idx = filtered.indexOf("open-file");
    if (idx >= 0) {
      filtered = [
        ...filtered.slice(0, idx + 1),
        "open-folder",
        ...filtered.slice(idx + 1),
      ];
    } else {
      filtered = ["open-folder", ...filtered];
    }
  }
  if (!filtered.includes("new-tab")) {
    // 一番左に置く(VSCode のタブ風 UX に合わせる)
    filtered = ["new-tab", ...filtered];
  }
  if (!filtered.includes("new-from-template")) {
    // new-tab の直後に挟む(テンプレ起動の動線を新規タブの隣に)
    const idx = filtered.indexOf("new-tab");
    if (idx >= 0) {
      filtered = [
        ...filtered.slice(0, idx + 1),
        "new-from-template",
        ...filtered.slice(idx + 1),
      ];
    } else {
      filtered = ["new-from-template", ...filtered];
    }
  }
  if (!filtered.includes("bibliography")) {
    // 表 (table) の直後に。挿入系の末尾に置くのが自然
    const idx = filtered.indexOf("table");
    if (idx >= 0) {
      filtered = [
        ...filtered.slice(0, idx + 1),
        "bibliography",
        ...filtered.slice(idx + 1),
      ];
    } else {
      filtered = [...filtered, "bibliography"];
    }
  }
  if (!filtered.includes("open-settings")) {
    // ツールバーの右端に追加(設定は最も右が VSCode 流)
    filtered = [...filtered, "open-settings"];
  }
  return filtered;
}

function parseKeybindings(raw: unknown): KeybindingsSettings {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return {};
  const result: KeybindingsSettings = {};
  for (const [k, v] of Object.entries(raw)) {
    if (!isCommandId(k)) continue;
    if (typeof v !== "string") continue;
    result[k] = v;
  }
  return result;
}

function clamp(v: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, v));
}

function parseWorkspace(raw: unknown): WorkspaceSettings {
  const def = DEFAULT_SETTINGS.workspace;
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return def;
  const obj = raw as Record<string, unknown>;
  return {
    previewVisible:
      typeof obj.previewVisible === "boolean"
        ? obj.previewVisible
        : def.previewVisible,
    editorPaneRatio:
      typeof obj.editorPaneRatio === "number" &&
      Number.isFinite(obj.editorPaneRatio)
        ? clamp(obj.editorPaneRatio, 0.1, 0.9)
        : def.editorPaneRatio,
    projectViewVisible:
      typeof obj.projectViewVisible === "boolean"
        ? obj.projectViewVisible
        : def.projectViewVisible,
    projectPaneRatio:
      typeof obj.projectPaneRatio === "number" &&
      Number.isFinite(obj.projectPaneRatio)
        ? clamp(obj.projectPaneRatio, 0.1, 0.5)
        : def.projectPaneRatio,
    currentFolder:
      typeof obj.currentFolder === "string" ? obj.currentFolder : null,
    statusbarVisible:
      typeof obj.statusbarVisible === "boolean"
        ? obj.statusbarVisible
        : def.statusbarVisible,
  };
}

// settings.json の JSON 構文を検証する。null = 問題なし(パース可能 or
// ファイル未作成)、文字列 = 人間可読なエラー(行・列付き)。Tauri Store
// は reload 時のパース詳細を返さないので、ユーザに具体的な位置を伝える
// ためにここで明示的にチェックする。
export async function validateSettingsJson(): Promise<string | null> {
  try {
    const result = await invoke<string | null>("validate_settings_json");
    return result;
  } catch (e) {
    return String(e);
  }
}

export async function loadSettings(): Promise<Settings> {
  // Yuhitsu 内で settings.json を直接 fs::write した場合、Tauri Store は
  // メモリにキャッシュした古い値を返す。Yuhitsu 経由・外部エディタ経由
  // どちらの編集も拾うため、毎回 reload で強制再読み込みする。
  try {
    await store.reload();
  } catch {
    // 初回など、まだファイルが無い場合は reload が失敗する。無視して
    // 後段の get でデフォルトに落とす。
  }
  const editorMode = await store.get<unknown>("editor.mode");
  const themeMode = await store.get<unknown>("appearance.theme");
  const localeMode = await store.get<unknown>("appearance.locale");
  const paperSize = await store.get<unknown>("document.paperSize");
  const firstRunDone = await store.get<unknown>("flags.firstRunDone");
  const toolbarItems = await store.get<unknown>("toolbar.items");
  const keybindings = await store.get<unknown>("keybindings");
  const workspace = await store.get<unknown>("workspace");
  const aiRaw = await store.get<unknown>("ai");
  return {
    editor: {
      mode: isEditorMode(editorMode)
        ? editorMode
        : DEFAULT_SETTINGS.editor.mode,
    },
    appearance: {
      theme: isThemeMode(themeMode)
        ? themeMode
        : DEFAULT_SETTINGS.appearance.theme,
      locale: isLocaleMode(localeMode)
        ? localeMode
        : DEFAULT_SETTINGS.appearance.locale,
    },
    document: {
      paperSize: isPaperSize(paperSize)
        ? paperSize
        : DEFAULT_SETTINGS.document.paperSize,
    },
    flags: {
      firstRunDone:
        typeof firstRunDone === "boolean"
          ? firstRunDone
          : DEFAULT_SETTINGS.flags.firstRunDone,
    },
    toolbar: {
      items: parseToolbarItems(toolbarItems),
    },
    keybindings: parseKeybindings(keybindings),
    workspace: parseWorkspace(workspace),
    ai:
      aiRaw && typeof aiRaw === "object" && !Array.isArray(aiRaw)
        ? (aiRaw as Settings["ai"])
        : DEFAULT_SETTINGS.ai,
  };
}

export async function saveEditorMode(mode: EditorMode): Promise<void> {
  await store.set("editor.mode", mode);
  await store.save();
}

export async function saveTheme(theme: ThemeMode): Promise<void> {
  await store.set("appearance.theme", theme);
  await store.save();
}

export async function saveLocale(locale: LocaleMode): Promise<void> {
  await store.set("appearance.locale", locale);
  await store.save();
}

export async function savePaperSize(paperSize: PaperSize): Promise<void> {
  await store.set("document.paperSize", paperSize);
  await store.save();
}

export async function saveFirstRunDone(done: boolean): Promise<void> {
  await store.set("flags.firstRunDone", done);
  await store.save();
}

export async function saveToolbarItems(items: ToolbarItem[]): Promise<void> {
  await store.set("toolbar.items", items);
  await store.save();
}

export async function saveKeybindings(
  keybindings: KeybindingsSettings,
): Promise<void> {
  // CommandId 以外のキーが混ざらないようサニタイズしてから保存
  const sanitized: KeybindingsSettings = {};
  for (const id of COMMAND_IDS) {
    const v = keybindings[id];
    if (typeof v === "string" && v.length > 0) sanitized[id] = v;
  }
  await store.set("keybindings", sanitized);
  await store.save();
}

export async function saveWorkspace(workspace: WorkspaceSettings): Promise<void> {
  await store.set("workspace", {
    previewVisible: workspace.previewVisible,
    editorPaneRatio: clamp(workspace.editorPaneRatio, 0.1, 0.9),
    projectViewVisible: workspace.projectViewVisible,
    projectPaneRatio: clamp(workspace.projectPaneRatio, 0.1, 0.5),
    currentFolder: workspace.currentFolder,
    statusbarVisible: workspace.statusbarVisible,
  });
  await store.save();
}
