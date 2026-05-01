/**
 * locale 解決ヘルパー。
 *
 * settings.appearance.locale は "auto" | "ja" | "en" のいずれかで、
 * "auto" は実行時の navigator.language から推測する。未対応 locale は
 * "en" にフォールバックする(英語が最も汎用的に通じる前提)。
 */
import type { LocaleMode } from "$lib/settings";

export type Locale = "ja" | "en";
export const SUPPORTED_LOCALES: readonly Locale[] = ["ja", "en"];

/**
 * 設定値("auto" 含む)を実 locale に解決する。
 * SSR / hydration 前など navigator が存在しない場合は "en" を返す。
 */
export function resolveLocale(setting: LocaleMode): Locale {
  if (setting !== "auto") return setting;
  if (typeof navigator === "undefined") return "en";
  const raw = navigator.language ?? "en";
  // "ja-JP" / "en-US" 等を主言語タグだけに正規化
  const primary = raw.toLowerCase().split("-")[0];
  return (SUPPORTED_LOCALES as readonly string[]).includes(primary)
    ? (primary as Locale)
    : "en";
}
