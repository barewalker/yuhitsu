// CodeMirror の EditorView を直接操作する Typst 編集コマンド群。
// ツールバー / キーバインド / 将来のコマンドパレット / MCP ハンドラから
// 共通で呼ぶことを想定し、UI 非依存の純粋関数として閉じる。

import { EditorSelection, type EditorState } from "@codemirror/state";
import type { EditorView } from "@codemirror/view";

export type HeadingLevel = 1 | 2 | 3;

// Typst の行頭ブロックマーカー(見出し `=...` / 箇条書き `-` / 番号付き `+`)。
// これらは行頭の同一スロットを取り合う相互排他なので、1 本のパターンで
// まとめて検出し、付け替え時は既存マーカーを剥がしてから付与する。
// インデントのある行も拾えるよう、先頭の空白を許容する。
const LINE_PREFIX_PATTERN = /^(\s*)(=+\s|[-+]\s)/;

// 空行(改行のみ・テキストなし)に prefix を挿入する場合、デフォルトの
// changes 適用ではカーソルが行頭のまま残り「|= 」のように見える。
// 既存テキストがある行では「テキスト先頭」=「prefix の直後」へ自然に
// 押し出されるので、空行だけ別途カーソルを補正する。
function adjustCursorForEmptyLines(
  view: EditorView,
  preState: EditorState,
  targetLineNumbers: number[],
  prefixLength: number,
) {
  const emptyTargets = targetLineNumbers.filter(
    (n) => preState.doc.line(n).text.length === 0,
  );
  if (emptyTargets.length === 0) return;
  const postState = view.state;
  const newRanges = postState.selection.ranges.map((range, i) => {
    const orig = preState.selection.ranges[i];
    if (!orig?.empty) return range;
    const origLine = preState.doc.lineAt(orig.from);
    if (!emptyTargets.includes(origLine.number)) return range;
    const postLine = postState.doc.line(origLine.number);
    return EditorSelection.cursor(postLine.from + prefixLength);
  });
  view.dispatch({ selection: EditorSelection.create(newRanges) });
}

export function toggleBold(view: EditorView) {
  toggleInlineWrap(view, "*");
}

export function toggleItalic(view: EditorView) {
  toggleInlineWrap(view, "_");
}

export function toggleMath(view: EditorView) {
  toggleInlineWrap(view, "$");
}

export function toggleInlineCode(view: EditorView) {
  toggleInlineWrap(view, "`");
}

export function applyHeading(view: EditorView, level: HeadingLevel) {
  toggleLinePrefix(view, "=".repeat(level) + " ");
}

export function toggleBulletList(view: EditorView) {
  toggleLinePrefix(view, "- ");
}

export function toggleNumberedList(view: EditorView) {
  toggleLinePrefix(view, "+ ");
}

// ` ``` ` で囲んだ言語指定なしのフェンスドコードを挿入する。
// 選択範囲があれば中身に入れ、なければ空のブロックでカーソルを内側に置く。
export function insertCodeBlock(view: EditorView) {
  const { state } = view;
  const range = state.selection.main;
  const selected = state.doc.sliceString(range.from, range.to);
  const fence = "```";
  const body = selected;
  const insertText = `${fence}\n${body}\n${fence}`;
  // 選択あり → コードブロックの後ろにカーソル、なし → 中央行頭
  const cursor = selected
    ? range.from + insertText.length
    : range.from + fence.length + 1;
  view.dispatch({
    changes: { from: range.from, to: range.to, insert: insertText },
    selection: EditorSelection.cursor(cursor),
    scrollIntoView: true,
  });
  view.focus();
}

// `#link("URL")[テキスト]` を挿入。選択ありなら表示テキスト部に流し、
// 編集しやすいよう URL プレースホルダ部分を選択状態にする。
export function insertLink(view: EditorView) {
  const { state } = view;
  const range = state.selection.main;
  const selected = state.doc.sliceString(range.from, range.to);
  const placeholder = "https://";
  const before = `#link("`;
  const middle = `")[`;
  const after = `]`;
  const insertText = `${before}${placeholder}${middle}${selected}${after}`;
  const urlStart = range.from + before.length;
  const urlEnd = urlStart + placeholder.length;
  view.dispatch({
    changes: { from: range.from, to: range.to, insert: insertText },
    selection: EditorSelection.range(urlStart, urlEnd),
    scrollIntoView: true,
  });
  view.focus();
}

// `#footnote[本文]` を挿入。選択を本文に流し、本文末尾にカーソル。
export function insertFootnote(view: EditorView) {
  const { state } = view;
  const range = state.selection.main;
  const selected = state.doc.sliceString(range.from, range.to);
  const prefix = "#footnote[";
  const suffix = "]";
  const insertText = prefix + selected + suffix;
  const cursor = range.from + prefix.length + selected.length;
  view.dispatch({
    changes: { from: range.from, to: range.to, insert: insertText },
    selection: EditorSelection.cursor(cursor),
    scrollIntoView: true,
  });
  view.focus();
}

// `#quote(block: true)[...]` を改行込みで挿入。引用ブロック向け。
export function insertQuote(view: EditorView) {
  const { state } = view;
  const range = state.selection.main;
  const selected = state.doc.sliceString(range.from, range.to);
  const opening = "#quote(block: true)[\n";
  const closing = "\n]";
  const insertText = opening + selected + closing;
  const cursor = range.from + opening.length + selected.length;
  view.dispatch({
    changes: { from: range.from, to: range.to, insert: insertText },
    selection: EditorSelection.cursor(cursor),
    scrollIntoView: true,
  });
  view.focus();
}

// 最小の `#table(columns: 2, ...)` 2x2 テンプレを挿入。
// 後で「列数指定モーダル」を増設できるよう、テンプレ生成は引数化しておく。
export function insertTable(
  view: EditorView,
  options: { columns?: number; rows?: number } = {},
) {
  const columns = options.columns ?? 2;
  const rows = options.rows ?? 2;
  const lines: string[] = [`#table(`, `  columns: ${columns},`];
  for (let r = 0; r < rows; r++) {
    const cells: string[] = [];
    for (let c = 0; c < columns; c++) {
      // 1 行目はヘッダ、それ以外はセル。MVP のため簡素なラベル。
      const label = r === 0 ? `見出し ${c + 1}` : `セル ${r}-${c + 1}`;
      cells.push(`[${label}]`);
    }
    lines.push(`  ${cells.join(", ")},`);
  }
  lines.push(`)`);
  const tableText = lines.join("\n");
  const { state } = view;
  const range = state.selection.main;
  // 行頭以外で挿入するときは前段に改行を補い、ブロック構文として独立させる
  const lineStart = state.doc.lineAt(range.from);
  const charsBeforeCursor = state.doc
    .sliceString(lineStart.from, range.from)
    .trim();
  const needsLeadingNewline = charsBeforeCursor.length > 0;
  const insertText = (needsLeadingNewline ? "\n" : "") + tableText;
  const cursor = range.from + insertText.length;
  view.dispatch({
    changes: { from: range.from, to: range.to, insert: insertText },
    selection: EditorSelection.cursor(cursor),
    scrollIntoView: true,
  });
  view.focus();
}

// `#figure(image("path"), caption: [])` を挿入し、caption 内にカーソルを置く。
// 画像と説明をひとまとまりに扱える Typst 流儀に合わせ、テンプレート側で
// `#show figure: ...` でスタイルを上書きできる形を保つ。パス引数は
// ファイル選択ダイアログ / D&D 経由で確定したものをそのまま受ける。
export function insertImage(view: EditorView, path: string) {
  const escaped = path.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
  const before = `#figure(\n  image("${escaped}"),\n  caption: [`;
  const after = `],\n)`;
  const insertText = before + after;
  const { state } = view;
  const range = state.selection.main;
  // 行頭以外で挿入する場合は前段に改行を補い、ブロック構文として独立させる
  const lineStart = state.doc.lineAt(range.from);
  const charsBeforeCursor = state.doc
    .sliceString(lineStart.from, range.from)
    .trim();
  const needsLeadingNewline = charsBeforeCursor.length > 0;
  const finalInsert = (needsLeadingNewline ? "\n" : "") + insertText;
  // caption の `[]` 内にカーソルを置く(挿入末尾から after の長さ分戻る)
  const cursor = range.from + finalInsert.length - after.length;
  view.dispatch({
    changes: { from: range.from, to: range.to, insert: finalInsert },
    selection: EditorSelection.cursor(cursor),
    scrollIntoView: true,
  });
  view.focus();
}

// 選択範囲を delim でラップする。既にラップされていれば外す。
// 複数選択(矩形選択含む)も changeByRange で個別に処理。
function toggleInlineWrap(view: EditorView, delim: string) {
  const len = delim.length;
  view.dispatch(
    view.state.changeByRange((range) => {
      const doc = view.state.doc;
      const beforeFrom = Math.max(0, range.from - len);
      const afterTo = Math.min(doc.length, range.to + len);
      const before = doc.sliceString(beforeFrom, range.from);
      const after = doc.sliceString(range.to, afterTo);
      if (range.from !== range.to && before === delim && after === delim) {
        return {
          changes: [
            { from: range.from - len, to: range.from, insert: "" },
            { from: range.to, to: range.to + len, insert: "" },
          ],
          range: EditorSelection.range(range.from - len, range.to - len),
        };
      }
      if (range.from === range.to) {
        return {
          changes: [{ from: range.from, insert: delim + delim }],
          range: EditorSelection.cursor(range.from + len),
        };
      }
      return {
        changes: [
          { from: range.from, insert: delim },
          { from: range.to, insert: delim },
        ],
        range: EditorSelection.range(range.from + len, range.to + len),
      };
    }),
  );
  view.focus();
}

// 選択範囲が掛かっている行をすべて拾う(行番号で重複除去)。
function collectSelectedLines(view: EditorView) {
  const { state } = view;
  const numbers = new Set<number>();
  for (const range of state.selection.ranges) {
    const startLine = state.doc.lineAt(range.from).number;
    const endLine = state.doc.lineAt(range.to).number;
    for (let n = startLine; n <= endLine; n++) {
      numbers.add(n);
    }
  }
  return [...numbers].sort((a, b) => a - b).map((n) => state.doc.line(n));
}

// 行頭ブロックマーカー(見出し / 箇条書き / 番号付き)の付与・除去・置換を
// 一手に扱うトグル。これら3者は相互排他なので、`desiredMarker` と完全一致する
// マーカーが全行に既に付いていれば除去、別ファミリー(別レベル含む)が
// 付いていれば置換、何もなければ付与する。
function toggleLinePrefix(view: EditorView, desiredMarker: string) {
  const preState = view.state;
  const targetLines = collectSelectedLines(view);
  const allMatch = targetLines.every((line) => {
    const m = LINE_PREFIX_PATTERN.exec(line.text);
    return m !== null && m[2] === desiredMarker;
  });
  const changes = targetLines.map((line) => {
    const m = LINE_PREFIX_PATTERN.exec(line.text);
    const indent = m ? m[1] : "";
    const indentLen = indent.length;
    if (allMatch && m) {
      // 全行が同じ → インデントは残してマーカー部分だけ除去
      return {
        from: line.from + indentLen,
        to: line.from + m[0].length,
        insert: "",
      };
    }
    if (m) {
      // 別ファミリー / 別レベル → 置換
      return {
        from: line.from + indentLen,
        to: line.from + m[0].length,
        insert: desiredMarker,
      };
    }
    // 行頭にマーカーなし → インデントの後ろに付与
    const leadingWs = /^\s*/.exec(line.text)?.[0] ?? "";
    return {
      from: line.from + leadingWs.length,
      to: line.from + leadingWs.length,
      insert: desiredMarker,
    };
  });
  view.dispatch({ changes, scrollIntoView: true });
  if (!allMatch) {
    adjustCursorForEmptyLines(
      view,
      preState,
      targetLines.map((l) => l.number),
      desiredMarker.length,
    );
  }
  view.focus();
}
