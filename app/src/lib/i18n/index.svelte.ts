/**
 * UI 文字列の多言語対応(自前 i18n)。
 *
 * - 辞書は `ja.json` / `en.json`(ネスト構造、同形)
 * - `i18nState.locale` を `$state` で持ち、コンポーネント側で読むだけで
 *   locale 変更にリアクティブに追従する
 * - `t("command.bold")` のようなドット区切りキーで辞書を引く。値が無ければ
 *   en 辞書にフォールバック → それでも無ければキー文字列をそのまま返す
 * - `t("status.templateMissing", { id: "foo" })` で `{id}` を置換
 *
 * 採用ライブラリは無し(範囲が hover / 確認 / ステータス / プレースホルダに
 * 絞られたため、辞書ルックアップ + シンプルな差し込みで足りる)。
 */
import type { Locale } from "./locale";
import jaDict from "./ja.json";
import enDict from "./en.json";

type Dict = Record<string, unknown>;

const DICTIONARIES: Record<Locale, Dict> = {
  ja: jaDict as Dict,
  en: enDict as Dict,
};

// 共有リアクティブストア。コンポーネントから `i18nState.locale` を読むと、
// その読み取り元は locale 変更時に自動再計算される。
export const i18nState = $state<{ locale: Locale }>({ locale: "en" });

export function setLocale(l: Locale): void {
  i18nState.locale = l;
}

function lookup(dict: Dict, path: string): string | null {
  const parts = path.split(".");
  let cur: unknown = dict;
  for (const p of parts) {
    if (typeof cur !== "object" || cur === null) return null;
    cur = (cur as Record<string, unknown>)[p];
  }
  return typeof cur === "string" ? cur : null;
}

function interpolate(
  template: string,
  params: Record<string, string | number>,
): string {
  return template.replace(/\{(\w+)\}/g, (_, key) =>
    key in params ? String(params[key]) : `{${key}}`,
  );
}

export function t(
  key: string,
  params?: Record<string, string | number>,
): string {
  const primary = lookup(DICTIONARIES[i18nState.locale], key);
  const value = primary ?? lookup(DICTIONARIES.en, key) ?? key;
  return params ? interpolate(value, params) : value;
}
