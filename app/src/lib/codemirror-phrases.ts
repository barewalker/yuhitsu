/**
 * CodeMirror の組み込み UI(検索パネル / Go to line / 等)を多言語化する
 * `EditorState.phrases` 用の辞書。i18n 辞書(JSON)とは別物で、CodeMirror
 * 側のキー文字列(英語のソース)をそのまま key として使う仕様。
 */
import type { Locale } from "$lib/i18n/locale";

const JA_PHRASES: Record<string, string> = {
  // 検索パネル
  Find: "検索",
  Replace: "置換",
  next: "次へ",
  previous: "前へ",
  all: "すべて選択",
  "match case": "大文字小文字を区別",
  regexp: "正規表現",
  "by word": "単語単位",
  replace: "置換",
  "replace all": "すべて置換",
  close: "閉じる",
  // Go to line
  "Go to line": "行に移動",
  go: "移動",
  "current match": "現在の一致",
  "on line": "行",
};

export function phrasesFor(locale: Locale): Record<string, string> {
  if (locale === "ja") return JA_PHRASES;
  // 英語は CodeMirror のソースそのままで OK(空辞書を返すと素通し)
  return {};
}
