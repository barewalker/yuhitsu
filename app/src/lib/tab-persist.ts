// 開いていたタブ一覧を `<app_data_dir>/tabs.json` に保存・復元する。
// 保存内容:
//   - file タブ:絶対パス(content はディスクから読み直す)
//   - untitled タブ:content そのまま(dirty=true で復活させる)
// どちらも cursor / scroll を per-tab で持って復元する(hot exit 体験)。
//
// 仮想 path(無題タブ)は再起動毎に振り直されるため永続化対象外。
// 復元時に `prepare_untitled_path` で新規割り当てされる。

import { invoke } from "@tauri-apps/api/core";

export type PersistedTab =
  | {
      kind: "file";
      path: string;
      cursorAnchor: number;
      cursorHead: number;
      scrollTop: number;
    }
  | {
      kind: "untitled";
      content: string;
      cursorAnchor: number;
      cursorHead: number;
      scrollTop: number;
    };

export type PersistedTabState = {
  tabs: PersistedTab[];
  // tabs 配列内のインデックス(0 始まり)。タブが無ければ -1。
  activeIndex: number;
};

export async function saveTabState(state: PersistedTabState): Promise<void> {
  const payload = JSON.stringify(state);
  try {
    await invoke("save_tab_state", { payload });
  } catch (e) {
    // 永続化失敗は致命的でない(次回起動時にデフォルトで立ち上がるだけ)
    console.warn("[tabs] save failed:", e);
  }
}

export async function loadTabState(): Promise<PersistedTabState | null> {
  try {
    const raw = await invoke<string>("load_tab_state");
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    // 最低限の型ガード(壊れた JSON / 旧バージョンを skip)
    if (
      !parsed ||
      typeof parsed !== "object" ||
      !Array.isArray(parsed.tabs) ||
      typeof parsed.activeIndex !== "number"
    ) {
      return null;
    }
    return parsed as PersistedTabState;
  } catch (e) {
    console.warn("[tabs] load failed:", e);
    return null;
  }
}
