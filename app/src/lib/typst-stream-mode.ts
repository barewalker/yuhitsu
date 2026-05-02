/**
 * Typst の最小構文ハイライタ(StreamLanguage 実装)。
 *
 * Sprint 3 で `codemirror-lang-typst` を一旦剥がした(WASM panic、上流 issue #5)
 * 後の代替。LSP(tinymist)が完全な構文認識を担当しているので、エディタ側は
 * 表面的な色付けで足りる。AST が要らない範囲(コメント / 文字列 / 数値 /
 * キーワード / 関数呼び出し / 見出し / リスト / 強調 / 数式 / インラインコード /
 * 識別子・プロパティ)を行単位で正規表現マッチして tag を割り当てる。
 *
 * 限界:
 * - 強調 `*...*` `_..._` は単純なパターンで判定するため、テキスト中の
 *   アスタリスク/アンダースコアは誤検出する場合あり(完璧を期さない)
 * - 関数引数の中まで完全には追わない(propertyName は同じ行内の `key:` のみ)
 * - 数式・raw block は行を跨いで保持
 */
import { StreamLanguage, type StreamParser } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";

type State = {
  /** ブロックコメント `/* ... *\/` の途中か */
  inBlockComment: boolean;
  /** raw block ` ``` ... ``` ` の途中か */
  inRawBlock: boolean;
  /** 数式 `$...$` の途中か(行をまたぐ場合に保持) */
  inMath: boolean;
};

const KEYWORDS = new Set([
  "let",
  "set",
  "show",
  "if",
  "else",
  "while",
  "for",
  "in",
  "as",
  "import",
  "include",
  "return",
  "break",
  "continue",
  "context",
]);

const ATOMS = new Set(["true", "false", "none", "auto"]);

const tokenTable = {
  funcName: t.function(t.variableName),
};

const parser: StreamParser<State> = {
  startState() {
    return { inBlockComment: false, inRawBlock: false, inMath: false };
  },
  copyState(s) {
    return { ...s };
  },
  token(stream, state) {
    // --- 継続中のスパン処理 ---
    if (state.inBlockComment) {
      while (!stream.eol()) {
        if (stream.match("*/")) {
          state.inBlockComment = false;
          return "comment";
        }
        stream.next();
      }
      return "comment";
    }
    if (state.inRawBlock) {
      if (stream.match("```")) {
        state.inRawBlock = false;
        return "monospace";
      }
      while (!stream.eol()) stream.next();
      return "monospace";
    }
    if (state.inMath) {
      while (!stream.eol()) {
        if (stream.peek() === "$") {
          stream.next();
          state.inMath = false;
          return "literal";
        }
        stream.next();
      }
      return "literal";
    }

    // --- 行頭マーカー ---
    if (stream.sol()) {
      // 先頭の空白(インデント)はスキップしない方が見出しの判定が単純
      const indent = stream.match(/^[ \t]+/, true);
      if (indent) return null;

      // 見出し: 1〜6 個の `=` + 空白
      const heading = stream.match(/^(={1,6})[ \t]+/, true);
      if (heading) {
        // 行末まで heading の色で塗る
        stream.skipToEnd();
        const level = (heading as RegExpMatchArray)[1].length;
        if (level === 1) return "heading1";
        if (level === 2) return "heading2";
        if (level === 3) return "heading3";
        if (level === 4) return "heading4";
        return "heading";
      }
      // リストマーカー(行頭の `-` `+` `数値.`)
      if (stream.match(/^[-+][ \t]/, true)) return "list";
      if (stream.match(/^\d+\.[ \t]/, true)) return "list";
      // term marker `/ key:`
      if (stream.match(/^\/[ \t]/, true)) return "list";
    }

    // --- コメント ---
    if (stream.match("//")) {
      stream.skipToEnd();
      return "comment";
    }
    if (stream.match("/*")) {
      state.inBlockComment = true;
      return "comment";
    }

    // --- raw block / inline code ---
    if (stream.match("```")) {
      state.inRawBlock = true;
      return "monospace";
    }
    if (stream.match(/`[^`\n]*`/)) {
      return "monospace";
    }

    // --- 数式 $...$ ---
    if (stream.match("$")) {
      state.inMath = true;
      return "literal";
    }

    // --- 文字列リテラル ---
    if (stream.match(/"(?:\\.|[^"\\\n])*"/)) {
      return "string";
    }

    // --- # で始まる識別子(関数呼び出し / 変数参照) ---
    // `#name(` なら関数呼び出し、それ以外は変数参照。
    // hyphen 含み(`#business-report` 等)も拾う
    const hashIdent = stream.match(/#([a-zA-Z_][\w-]*)/, false);
    if (hashIdent) {
      // 一度 `#` だけ消費して、続きを別 token として返したいが、
      // StringStream は per-token なので識別子全体を 1 token として扱う
      stream.match(/#[a-zA-Z_][\w-]*/, true);
      // 直後が `(` なら function 呼び出し
      if (stream.peek() === "(") return "funcName";
      // 予約語(`#let` `#show` 等)はキーワード扱い
      const word = (hashIdent as RegExpMatchArray)[1];
      if (KEYWORDS.has(word)) return "keyword";
      if (ATOMS.has(word)) return "atom";
      return "variableName";
    }

    // --- 強調(同じ行内で閉じるシンプルパターン) ---
    // *bold* — アスタリスクの前後が空白でなく、同行内で閉じる
    if (stream.match(/\*[^\s*][^\n*]*\*/)) return "strong";
    // _italic_
    if (stream.match(/_[^\s_][^\n_]*_/)) return "emphasis";

    // --- 数値(単位付きも) ---
    if (stream.match(/-?\d+(\.\d+)?(em|pt|cm|mm|in|fr|%|deg)?/)) {
      return "number";
    }

    // --- 識別子(裸):キーワード / atom / プロパティ判定 ---
    const ident = stream.match(/[a-zA-Z_][\w-]*/, true);
    if (ident) {
      const word = (ident as RegExpMatchArray)[0];
      if (KEYWORDS.has(word)) return "keyword";
      if (ATOMS.has(word)) return "atom";
      // 直後が `:` なら named argument のキー扱い(`size: 12pt` 等)
      if (stream.peek() === ":") return "propertyName";
      return null;
    }

    // --- 演算子 / 記号 ---
    if (stream.match(/[=!<>]=|->|=>|[+\-*/=<>!]/)) return "operator";
    if (stream.match(/[(){}\[\]]/)) return "punctuation";

    // どのパターンにも当たらない 1 文字
    stream.next();
    return null;
  },
  languageData: {
    commentTokens: { line: "//", block: { open: "/*", close: "*/" } },
  },
  tokenTable,
};

/**
 * StreamLanguage として export。Editor.svelte の langExtension で
 * `[typstStreamLanguage, syntaxHighlighting(highlightStyle)]` を返せば動く。
 */
export const typstStreamLanguage = StreamLanguage.define(parser);
