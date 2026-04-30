// Rust の stderr(`pnpm tauri dev` のログ)にフロント側メッセージを流す
// 開発用ロガー。WebView の DevTools を開かなくても状況を確認できる。
// リリース前に削除するか、本番では no-op にする想定。

import { invoke } from "@tauri-apps/api/core";

export function dlog(...args: unknown[]) {
  const message = args
    .map((a) => {
      if (typeof a === "string") return a;
      try {
        return JSON.stringify(a);
      } catch {
        return String(a);
      }
    })
    .join(" ");
  invoke("dev_log", { message }).catch(() => {
    // dev_log への送信失敗は致命的でない(本番ビルドで未登録など)。
  });
}
