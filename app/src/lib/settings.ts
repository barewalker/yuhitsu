import { LazyStore } from "@tauri-apps/plugin-store";

/**
 * Yuhitsu の永続設定スキーマ。
 *
 * Phase 1 ではエディタモードのみ実装するが、将来 AI 連携で必要になる
 * 設定領域(プロバイダ・API キーなど)を最初から型として確保しておく。
 * これにより設定ファイルの構造を後から組み替える必要がなくなる。
 */
export type EditorMode = "default" | "vim" | "emacs";

export type Settings = {
  editor: {
    mode: EditorMode;
  };
  ai: {
    // Phase 2 以降で利用する想定の領域。今は空のまま。
    // 例: provider: 'anthropic' | 'openai' | 'local', apiKeyRef: string, ...
    [key: string]: unknown;
  };
};

const DEFAULT_SETTINGS: Settings = {
  editor: {
    mode: "default",
  },
  ai: {},
};

const STORE_FILE = "settings.json";

// LazyStore は最初の操作時に初期化される。アプリ起動コストを増やさないため採用。
const store = new LazyStore(STORE_FILE);

function isEditorMode(value: unknown): value is EditorMode {
  return value === "default" || value === "vim" || value === "emacs";
}

export async function loadSettings(): Promise<Settings> {
  const editorMode = await store.get<unknown>("editor.mode");
  const aiRaw = await store.get<unknown>("ai");
  return {
    editor: {
      mode: isEditorMode(editorMode) ? editorMode : DEFAULT_SETTINGS.editor.mode,
    },
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
