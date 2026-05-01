/**
 * 同梱テンプレートのローダー。
 *
 * Vite の import.meta.glob で `templates/<id>/{meta.json, ja.typ, en.typ}` を
 * ビルド時に静的解決し、バンドルに含める。Tauri ファイルシステムへのアクセスは
 * 不要(将来 yuhitsu-ja に切り出す時はここのパスだけ書き換える)。
 *
 * 用紙サイズは本文中の "{{paper}}" プレースホルダで保持し、resolveTemplate 時に
 * settings.document.paperSize から実際の値を流し込む。
 */
import type { Locale } from "$lib/i18n/locale";
import type { PaperSize } from "$lib/settings";

export type TemplateMeta = {
  id: string;
  icon: string;
  category: string;
  title: Record<Locale, string>;
  description: Record<Locale, string>;
};

export type Template = {
  meta: TemplateMeta;
  /** 解決済みの本文(プレースホルダ展開後)。エディタにそのまま流し込める。 */
  body: string;
};

// meta.json を eager 読み込み(JSON は Vite が型付きで返す)
const metaModules = import.meta.glob<{ default: Omit<TemplateMeta, "id"> }>(
  "./*/meta.json",
  { eager: true },
);
// .typ はテキストとして読む(?raw)
const bodyModules = import.meta.glob<string>("./*/*.typ", {
  query: "?raw",
  import: "default",
  eager: true,
});

/** "./empty/meta.json" → "empty" */
function pathToId(path: string): string {
  const m = path.match(/^\.\/([^/]+)\//);
  return m ? m[1] : path;
}

const metaCatalog: Map<string, TemplateMeta> = new Map();
for (const [path, mod] of Object.entries(metaModules)) {
  const id = pathToId(path);
  metaCatalog.set(id, { id, ...mod.default });
}

// 表示順の制御:この配列の順で出す。catalog にあって ORDER に無い id は末尾に。
const ORDER: readonly string[] = [
  "empty",
  "business-report",
  "technical-report",
  "meeting-minutes",
  "letter",
  "slides",
];

export function listTemplates(): TemplateMeta[] {
  const ordered: TemplateMeta[] = [];
  const seen = new Set<string>();
  for (const id of ORDER) {
    const m = metaCatalog.get(id);
    if (m) {
      ordered.push(m);
      seen.add(id);
    }
  }
  for (const [id, m] of metaCatalog) {
    if (!seen.has(id)) ordered.push(m);
  }
  return ordered;
}

/** locale → 解決後の用紙サイズへのマッピング(auto 解決用) */
function resolvePaper(setting: PaperSize, locale: Locale): string {
  if (setting !== "auto") return setting;
  // ja は A4、en は US Letter が自然なデフォルト
  return locale === "ja" ? "a4" : "us-letter";
}

/** Typst の paper 識別子に正規化(letter → us-letter) */
function normalizePaper(paper: string): string {
  if (paper === "letter") return "us-letter";
  return paper;
}

/**
 * テンプレートを実体化する。
 * - 該当 locale の本文が無ければ "en" にフォールバック
 * - {{paper}} を実際の用紙識別子に置換
 */
export function resolveTemplate(
  id: string,
  locale: Locale,
  paperSize: PaperSize,
): Template | null {
  const meta = metaCatalog.get(id);
  if (!meta) return null;
  const tryLocale = (l: Locale): string | null => {
    const path = `./${id}/${l}.typ`;
    return bodyModules[path] ?? null;
  };
  const raw = tryLocale(locale) ?? tryLocale("en");
  if (raw === null) return null;
  const paper = normalizePaper(resolvePaper(paperSize, locale));
  const body = raw.replace(/\{\{paper\}\}/g, paper);
  return { meta, body };
}
