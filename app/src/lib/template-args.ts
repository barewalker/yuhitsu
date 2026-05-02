/**
 * `#show: <ident>.with(...)` の引数を読み・書き戻すための簡易パーサ。
 *
 * Sprint 3 簡素版ではフォーム入力対象を string / number / boolean に限定する。
 * その他(配列・関数呼び出し・コードブロック等)は raw のまま保持し、フォーム
 * からは編集しない(将来の成熟版で拡張)。
 *
 * 文字列リテラルは Typst のダブルクォート文字列のみ対応。`\\` `\"` `\n` `\r`
 * `\t` のエスケープを read/write 両方で扱う。シングルクォート文字列やネスト
 * された content block(`[...]`)は対象外で raw のまま。
 */

export type ArgKind =
  | { kind: "string"; value: string }
  | { kind: "number"; value: number }
  | { kind: "boolean"; value: boolean }
  | { kind: "raw"; raw: string };

export type WithCall = {
  /** ドキュメント内の `#show:` 行の先頭オフセット。書き戻しの from。 */
  from: number;
  /** 閉じ括弧 `)` の次のオフセット(改行は含めない)。書き戻しの to。 */
  to: number;
  /** 関数名。`#show: business-report.with(...)` の `business-report`。 */
  fn: string;
  /** 引数を順番通りに保持。**未指定キーは含めない**(テンプレ側 default に従う)。 */
  args: { name: string; value: ArgKind }[];
};

const IDENT_RE = /[\p{ID_Start}_-][\p{ID_Continue}_-]*/u;

// doc 全体から最初の `#show: <ident>.with(...)` を見つけて返す。
// 見つからない or 構文異常なら null。
// 行コメント / ブロックコメントは素朴にスキップする(文字列リテラル中の
// コメント風文字列は Typst 側で問題にならないので無視)。
export function findWithCall(doc: string): WithCall | null {
  const showRe = /#show\s*:\s*/g;
  let match: RegExpExecArray | null;
  while ((match = showRe.exec(doc)) !== null) {
    const cursorAfterShow = match.index + match[0].length;
    const identMatch = doc.slice(cursorAfterShow).match(IDENT_RE);
    if (!identMatch || identMatch.index !== 0) continue;
    const fn = identMatch[0];
    let p = cursorAfterShow + fn.length;
    if (doc.slice(p, p + 5) !== ".with") continue;
    p += 5;
    // .with の直後の "(" を期待
    while (p < doc.length && /\s/.test(doc[p])) p++;
    if (doc[p] !== "(") continue;
    const openParen = p;
    const closeParen = scanMatchingParen(doc, openParen);
    if (closeParen < 0) continue;
    const inner = doc.slice(openParen + 1, closeParen);
    const args = parseArgs(inner);
    if (!args) continue;
    return {
      from: match.index,
      to: closeParen + 1,
      fn,
      args,
    };
  }
  return null;
}

/** `(` の位置を受け取り、対応する `)` の位置を返す。見つからなければ -1。 */
function scanMatchingParen(doc: string, openIdx: number): number {
  let depth = 0;
  let i = openIdx;
  while (i < doc.length) {
    const ch = doc[i];
    if (ch === '"') {
      i = skipString(doc, i);
      continue;
    }
    if (ch === "/" && doc[i + 1] === "/") {
      // 行コメント: 改行まで
      const nl = doc.indexOf("\n", i);
      i = nl < 0 ? doc.length : nl + 1;
      continue;
    }
    if (ch === "/" && doc[i + 1] === "*") {
      const end = doc.indexOf("*/", i + 2);
      i = end < 0 ? doc.length : end + 2;
      continue;
    }
    if (ch === "(" || ch === "[" || ch === "{") depth++;
    else if (ch === ")" || ch === "]" || ch === "}") {
      depth--;
      if (depth === 0 && ch === ")") return i;
      if (depth < 0) return -1;
    }
    i++;
  }
  return -1;
}

/** `"..."` の開始位置(`"`)から閉じクォートの **次の** 位置を返す。 */
function skipString(doc: string, openIdx: number): number {
  let i = openIdx + 1;
  while (i < doc.length) {
    const ch = doc[i];
    if (ch === "\\") {
      i += 2;
      continue;
    }
    if (ch === '"') return i + 1;
    i++;
  }
  return doc.length;
}

/**
 * `with(...)` の中身(括弧の中だけ)をパースする。
 * カンマでトップレベル分割 → 各項目を `key:` と value に分ける。
 * 末尾カンマは許容、空白・改行は無視。
 */
function parseArgs(inner: string): { name: string; value: ArgKind }[] | null {
  const out: { name: string; value: ArgKind }[] = [];
  const parts = splitTopLevel(inner);
  for (const part of parts) {
    const trimmed = part.trim();
    if (trimmed.length === 0) continue;
    // `key: value` の形式を期待。pos until ":"(ただし string や bracket 中は除く)
    const colonIdx = findTopLevelColon(trimmed);
    if (colonIdx < 0) return null;
    const name = trimmed.slice(0, colonIdx).trim();
    if (!IDENT_RE.test(name)) return null;
    const valueStr = trimmed.slice(colonIdx + 1).trim();
    const value = parseValue(valueStr);
    out.push({ name, value });
  }
  return out;
}

function findTopLevelColon(s: string): number {
  let i = 0;
  let depth = 0;
  while (i < s.length) {
    const ch = s[i];
    if (ch === '"') {
      i = skipString(s, i);
      continue;
    }
    if (depth === 0 && ch === ":") return i;
    if (ch === "(" || ch === "[" || ch === "{") depth++;
    else if (ch === ")" || ch === "]" || ch === "}") depth--;
    i++;
  }
  return -1;
}

/** 中身をカンマ区切りでトップレベル分割。 */
function splitTopLevel(s: string): string[] {
  const out: string[] = [];
  let depth = 0;
  let start = 0;
  let i = 0;
  while (i < s.length) {
    const ch = s[i];
    if (ch === '"') {
      i = skipString(s, i);
      continue;
    }
    if (ch === "/" && s[i + 1] === "/") {
      const nl = s.indexOf("\n", i);
      i = nl < 0 ? s.length : nl + 1;
      continue;
    }
    if (ch === "/" && s[i + 1] === "*") {
      const end = s.indexOf("*/", i + 2);
      i = end < 0 ? s.length : end + 2;
      continue;
    }
    if (ch === "(" || ch === "[" || ch === "{") depth++;
    else if (ch === ")" || ch === "]" || ch === "}") depth--;
    else if (ch === "," && depth === 0) {
      out.push(s.slice(start, i));
      start = i + 1;
    }
    i++;
  }
  out.push(s.slice(start));
  return out;
}

function parseValue(s: string): ArgKind {
  if (s.length === 0) return { kind: "raw", raw: "" };
  if (s[0] === '"') {
    const closeIdx = skipString(s, 0);
    // 文字列の後ろに余計な文字がついていたら raw 扱い
    if (closeIdx === s.length) {
      const decoded = decodeTypstString(s.slice(1, closeIdx - 1));
      return { kind: "string", value: decoded };
    }
    return { kind: "raw", raw: s };
  }
  if (s === "true") return { kind: "boolean", value: true };
  if (s === "false") return { kind: "boolean", value: false };
  // 数値: 簡素版は整数 / 小数のみ。単位付き(`12pt` 等)は raw。
  if (/^-?\d+(\.\d+)?$/.test(s)) {
    const n = Number(s);
    if (Number.isFinite(n)) return { kind: "number", value: n };
  }
  return { kind: "raw", raw: s };
}

function decodeTypstString(s: string): string {
  return s.replace(/\\(.)/g, (_, ch: string) => {
    switch (ch) {
      case "n":
        return "\n";
      case "r":
        return "\r";
      case "t":
        return "\t";
      case "\\":
        return "\\";
      case '"':
        return '"';
      default:
        return ch;
    }
  });
}

export function encodeTypstString(s: string): string {
  let out = '"';
  for (const ch of s) {
    if (ch === "\\") out += "\\\\";
    else if (ch === '"') out += '\\"';
    else if (ch === "\n") out += "\\n";
    else if (ch === "\r") out += "\\r";
    else if (ch === "\t") out += "\\t";
    else out += ch;
  }
  return out + '"';
}

/** 単一の引数値を Typst 表現にエンコード。 */
export function encodeArgValue(v: ArgKind): string {
  switch (v.kind) {
    case "string":
      return encodeTypstString(v.value);
    case "number":
      return String(v.value);
    case "boolean":
      return v.value ? "true" : "false";
    case "raw":
      return v.raw;
  }
}

/**
 * `#show: <fn>.with(...)` 全体を再構築する。1 引数 / 行のスタイルで揃える。
 * 既存ドキュメントのインデント幅(`#show:` 行の手前の空白)に合わせる。
 */
export function formatWithCall(
  call: { fn: string; args: { name: string; value: ArgKind }[] },
  indent: string,
): string {
  if (call.args.length === 0) {
    return `${indent}#show: ${call.fn}.with()`;
  }
  const lines: string[] = [`${indent}#show: ${call.fn}.with(`];
  for (const a of call.args) {
    lines.push(`${indent}  ${a.name}: ${encodeArgValue(a.value)},`);
  }
  lines.push(`${indent})`);
  return lines.join("\n");
}

/**
 * doc の `from..to` を新しい `#show:` 表現で差し替える文字列を返す。
 * 行頭インデントは元の `#show:` 行に合わせる(壊れていれば空)。
 */
export function rebuildWithCall(
  doc: string,
  call: WithCall,
  newArgs: { name: string; value: ArgKind }[],
): { from: number; to: number; insert: string } {
  // call.from が `#show` の `#` 位置。直前の連続空白(インデント)を拾う。
  let lineStart = call.from;
  while (lineStart > 0 && doc[lineStart - 1] !== "\n") lineStart--;
  const indent = doc.slice(lineStart, call.from);
  return {
    from: call.from,
    to: call.to,
    insert: formatWithCall({ fn: call.fn, args: newArgs }, indent),
  };
}
