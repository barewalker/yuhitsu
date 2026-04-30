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
