// プロジェクトビュー(サイドバーのファイルツリー)用の最小データ層。
// Rust 側 `list_directory` の返値形をそのまま薄くラップする。

import { invoke } from "@tauri-apps/api/core";

export type DirEntry = {
  name: string;
  path: string;
  is_dir: boolean;
  children?: DirEntry[] | null;
};

export async function listDirectory(path: string): Promise<DirEntry> {
  return invoke<DirEntry>("list_directory", { path });
}

// ファイル毎の git status。値は 1 文字 status code:
//   "?" untracked, "M" modified, "A" added, "D" deleted, "R" renamed, "U" unmerged
// is_repo=false の時は entries も空。
export type GitStatus = {
  is_repo: boolean;
  entries: Record<string, string>;
};

export async function loadGitStatus(folder: string): Promise<GitStatus> {
  return invoke<GitStatus>("git_status", { folder });
}
