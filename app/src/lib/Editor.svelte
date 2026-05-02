<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    Compartment,
    EditorState,
    Prec,
    Transaction,
    type Extension,
  } from "@codemirror/state";
  import {
    EditorView,
    keymap,
    lineNumbers,
    highlightActiveLine,
    highlightActiveLineGutter,
  } from "@codemirror/view";
  import {
    defaultKeymap,
    history,
    historyKeymap,
  } from "@codemirror/commands";
  import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
  import { tags as t } from "@lezer/highlight";
  import { typstStreamLanguage } from "$lib/typst-stream-mode";
  import { vim } from "@replit/codemirror-vim";
  import { emacs } from "@replit/codemirror-emacs";
  import {
    LSPClient,
    languageServerSupport,
  } from "@codemirror/lsp-client";
  import type { EditorMode } from "$lib/settings";
  import { pathToFileUri } from "$lib/lsp";

  // ダーク背景向けに One Dark 風の配色を自前定義
  // (パッケージ同梱の TypstHighlightSytle はライト背景前提で黒に埋没するため不採用)
  const highlightStyle = HighlightStyle.define([
    { tag: t.heading, color: "var(--syntax-keyword)", fontWeight: "bold" },
    { tag: t.heading1, color: "var(--syntax-keyword)", fontWeight: "bold" },
    { tag: t.heading2, color: "var(--syntax-keyword)", fontWeight: "bold" },
    { tag: t.heading3, color: "var(--syntax-keyword)", fontWeight: "bold" },
    { tag: t.heading4, color: "var(--syntax-keyword)", fontWeight: "bold" },
    { tag: t.strong, color: "var(--text-primary)", fontWeight: "bold" },
    { tag: t.emphasis, color: "var(--text-primary)", fontStyle: "italic" },
    { tag: t.link, color: "var(--syntax-function)", textDecoration: "underline" },
    { tag: t.url, color: "var(--syntax-function)", textDecoration: "underline" },
    { tag: t.monospace, color: "var(--syntax-string)" },
    { tag: t.literal, color: "var(--syntax-string)" },
    { tag: t.string, color: "var(--syntax-string)" },
    { tag: [t.keyword, t.controlKeyword, t.definitionKeyword, t.modifier], color: "var(--syntax-operator)" },
    { tag: t.function(t.variableName), color: "var(--syntax-function)" },
    { tag: t.variableName, color: "var(--syntax-constant)" },
    { tag: t.propertyName, color: "var(--syntax-constant)" },
    { tag: t.labelName, color: "var(--syntax-number)" },
    { tag: t.number, color: "var(--syntax-number)" },
    { tag: t.bool, color: "var(--syntax-number)" },
    { tag: [t.atom, t.null], color: "var(--syntax-heading)" },
    { tag: t.operator, color: "var(--syntax-heading)" },
    { tag: [t.punctuation, t.bracket, t.brace, t.paren], color: "var(--syntax-heading)" },
    { tag: t.comment, color: "var(--syntax-comment)", fontStyle: "italic" },
    { tag: t.escape, color: "var(--syntax-heading)" },
    { tag: t.typeName, color: "var(--syntax-keyword)" },
    { tag: t.tagName, color: "var(--syntax-constant)" },
    { tag: t.attributeName, color: "var(--syntax-number)" },
    { tag: t.meta, color: "var(--syntax-comment)" },
    { tag: t.invalid, color: "var(--status-error-strong)" },
    // codemirror-lang-typst 固有のタグマッピング:
    //   ListMarker(`-`) / EnumMarker(`+`)    → t.list
    //   TermMarker(`/`)                       → t.definitionOperator
    //   見出しの `=` 等                        → t.processingInstruction
    { tag: t.list, color: "var(--syntax-heading)" },
    { tag: t.definitionOperator, color: "var(--syntax-heading)" },
    { tag: t.processingInstruction, color: "var(--syntax-heading)" },
    // 同パッケージは識別子に t.name も使う(関数呼び出しではない裸の参照など)
    { tag: t.name, color: "var(--syntax-text)" },
    { tag: [t.moduleKeyword, t.operatorKeyword], color: "var(--syntax-operator)" },
  ]);

  export type LanguageMode = "typst" | "plain";

  // ステータスバー用のカーソル情報。line / col は 1-origin、
  //   selected      = 選択範囲の長さ(全 code unit、選択なしなら 0)
  //   selectedNoWs  = 選択範囲から空白を除いた文字数(原稿カウント用)
  //   total         = doc 全体の文字数(空白・改行含む。VSCode 流儀)
  //   totalNoWs     = doc 全体から空白(改行 / 半角・全角スペース / タブ等)
  //                   を除いた文字数。原稿の字数感覚に近い
  // どちらを表示するかは親側(設定 charCountMode)で切替する。
  export type CursorInfo = {
    line: number;
    col: number;
    selected: number;
    selectedNoWs: number;
    total: number;
    totalNoWs: number;
  };

  // doc 全体から空白を除いた文字数を数える。`/\s/g` は ECMAScript 仕様で
  // Zs カテゴリ全般を含むため、半角・全角スペース・タブ・改行をすべて
  // 一括で取り除ける。
  function countNonWhitespace(s: string): number {
    return s.replace(/\s/g, "").length;
  }

  type Props = {
    value: string;
    /** タブ切替で復元する state スナップショット。指定があれば
        view.setState で完全置換し undo/redo スタックも引き継ぐ。
        指定なしの時は value だけ反映して history をリセットする
        (ファイル open / 新規タブの初期化用)。*/
    externalState?: EditorState | null;
    mode?: EditorMode;
    /** "typst" なら Typst 構文ハイライト、それ以外は plain */
    languageMode?: LanguageMode;
    /** LSP セッションが立ち上がっていればそのクライアント、なければ null */
    lspClient?: LSPClient | null;
    /** 現在編集中ファイルの絶対パス。LSP に渡す URI 構築に使う */
    filePath?: string | null;
    onChange?: (next: string) => void;
    /** カーソル位置 / 選択範囲 / doc 全体の文字数を通知。
     *  ステータスバーの "行 X 列 Y" / "文字数" 等に使う。
     *  doc 変化時 + 選択変化時の両方で発火する。 */
    onCursorChange?: (info: CursorInfo) => void;
    /** view 構築完了時に通知。親はこれを通じてコマンドを呼ぶ */
    onReady?: (view: EditorView) => void;
    /** view 破棄時に通知。親側のキャッシュを切るために使う */
    onTeardown?: () => void;
    /** 外部由来 value 変更が doc に適用された直後に呼ぶ。親が
        カーソル / スクロール位置を復元するためのフック(タブ切替で使う) */
    onValueApplied?: (view: EditorView) => void;
  };

  let {
    value,
    externalState = null,
    mode = "default",
    languageMode = "typst",
    lspClient = null,
    filePath = null,
    onChange,
    onCursorChange,
    onReady,
    onTeardown,
    onValueApplied,
  }: Props = $props();

  let host: HTMLDivElement;
  let view: EditorView | null = null;
  // 外部 value 反映と updateListener のループを防ぐためのフラグ
  let applyingExternal = false;
  // 直近に適用した externalState への参照。同じ参照(同タブのまま)で
  // $effect が走った時の不要な setState を避ける。
  let lastAppliedExternalState: EditorState | null = null;

  // mode を切り替えた時に extension を再構成なしで差し替えるための入れ物。
  // vim/emacs プラグインは optional で、default モードでは何も入れない。
  const modeCompartment = new Compartment();

  function modeExtension(target: EditorMode): Extension {
    switch (target) {
      case "vim":
        // vim プラグインは ex コマンドや Normal/Insert モードを定義するため、
        // 他のキーマップより優先される必要がある(プラグイン側で済ませている)
        return vim();
      case "emacs":
        return emacs();
      default:
        return [];
    }
  }

  // LSP の有効/無効と対象ファイルの切替に追従するための Compartment。
  // languageServerSupport は LSPClient + URI + languageId が揃って初めて有効。
  const lspCompartment = new Compartment();

  function lspExtension(
    client: LSPClient | null,
    file: string | null,
  ): Extension {
    if (!client || !file) return [];
    return languageServerSupport(client, pathToFileUri(file), "typst");
  }

  // Typst 言語拡張(構文ハイライト + Lezer parser)を on/off するための
  // Compartment。タブで Typst 以外を開いた時にプレーンテキストとして扱う。
  const langCompartment = new Compartment();

  // history extension を入れる Compartment。タブ切替や file open での
  // doc 全置換時に、reconfigure で history を作り直してリセットする。
  // これがないと、タブ A の編集 → タブ B に切替 → タブ B で undo を押すと、
  // 1 個の view を共有しているため A の状態にまで遡及してしまう。
  const historyCompartment = new Compartment();

  function langExtension(target: LanguageMode): Extension {
    if (target === "plain") return [];
    // StreamLanguage ベースの簡易ハイライタ($lib/typst-stream-mode.ts)。
    // codemirror-lang-typst の WASM panic を避けつつ、表面的な色付け
    // (コメント / 文字列 / 数値 / キーワード / 関数呼び出し / 見出し /
    //  リスト / 強調 / 数式 / インラインコード)を行ベースで提供する。
    // 完全な構文認識は LSP(tinymist)が担うので二重持ちは不要
    return [
      typstStreamLanguage,
      Prec.highest(syntaxHighlighting(highlightStyle)),
    ];
  }

  const theme = EditorView.theme(
    {
      "&": {
        backgroundColor: "var(--bg-base)",
        color: "var(--text-primary)",
        fontSize: "14px",
        height: "100%",
      },
      ".cm-scroller": {
        fontFamily:
          "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
        lineHeight: "1.5",
      },
      ".cm-content": {
        caretColor: "var(--text-primary)",
        padding: "8px 0",
      },
      ".cm-gutters": {
        backgroundColor: "var(--bg-editor-gutter)",
        color: "var(--text-faint)",
        border: "none",
      },
      ".cm-activeLine": {
        backgroundColor: "var(--bg-editor-active-line)",
      },
      ".cm-activeLineGutter": {
        backgroundColor: "var(--bg-elevated-2)",
        color: "var(--text-tertiary)",
      },
      ".cm-cursor, .cm-dropCursor": {
        borderLeftColor: "var(--text-primary)",
      },
      "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": {
        backgroundColor: "var(--accent-bg-subtle)",
      },
    },
    { dark: true },
  );

  onMount(() => {
    const updateListener = EditorView.updateListener.of((update) => {
      if (applyingExternal) return;
      // doc 変化はテキスト永続化 + preview memory file 注入につなぐ。
      if (update.docChanged) {
        onChange?.(update.state.doc.toString());
      }
      // ステータスバー用に「カーソル位置 + 選択範囲 + 全文字数」を通知。
      // doc 変化と選択変化の両方で発火させる(選択だけ動いた時にも追従)。
      if (update.docChanged || update.selectionSet) {
        const sel = update.state.selection.main;
        const line = update.state.doc.lineAt(sel.head);
        const docStr = update.state.doc.toString();
        const selStr =
          sel.from < sel.to ? update.state.sliceDoc(sel.from, sel.to) : "";
        onCursorChange?.({
          line: line.number,
          col: sel.head - line.from + 1,
          selected: sel.to - sel.from,
          selectedNoWs: countNonWhitespace(selStr),
          total: update.state.doc.length,
          totalNoWs: countNonWhitespace(docStr),
        });
      }
    });

    const state = EditorState.create({
      doc: value,
      extensions: [
        // mode 切替用 Compartment は他の extension より前に置く。
        // vim/emacs はキーマップを高い優先度で要求するため。
        modeCompartment.of(modeExtension(mode)),
        lspCompartment.of(lspExtension(lspClient, filePath)),
        // Typst 言語拡張(構文ハイライト)を切替可能に。typst() の同梱
        // ハイライトはライト前提・heading が黒なので、自前 highlightStyle
        // を最高優先度で当てる(plain モードでは丸ごと抜く)。
        langCompartment.of(langExtension(languageMode)),
        lineNumbers(),
        highlightActiveLine(),
        highlightActiveLineGutter(),
        historyCompartment.of(history()),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        theme,
        EditorView.lineWrapping,
        updateListener,
      ],
    });

    view = new EditorView({ state, parent: host });
    onReady?.(view);
    // 初期カーソル情報を通知。updateListener は docChanged / selectionSet
    // が立った時しか走らないため、view 構築直後の値はここで明示的に流す。
    const sel = state.selection.main;
    const initLine = state.doc.lineAt(sel.head);
    const initSelStr =
      sel.from < sel.to ? state.sliceDoc(sel.from, sel.to) : "";
    onCursorChange?.({
      line: initLine.number,
      col: sel.head - initLine.from + 1,
      selected: sel.to - sel.from,
      selectedNoWs: countNonWhitespace(initSelStr),
      total: state.doc.length,
      totalNoWs: countNonWhitespace(state.doc.toString()),
    });
  });

  onDestroy(() => {
    view?.destroy();
    view = null;
    onTeardown?.();
  });

  // 親から value が差し替わった時(ファイル open / タブ切替など)に
  // ドキュメントを置き換え、その後フック onValueApplied を呼ぶ。
  //
  // 注意:codemirror-lang-typst v0.4.0 の WASM パーサは「Typst 言語拡張が
  // 有効な状態で全置換 edit」を処理できず Unreachable で落ちることがある。
  // そのため言語拡張を一旦外してから doc を入れ替え、その後で再有効化する。
  $effect(() => {
    if (!view) return;
    // タブ切替で per-tab の state を復元するルート。externalState には
    // doc / 選択 / undo redo / scroll などが全部入っているので、
    // view.setState で丸ごと差し替えれば前タブの履歴が独立に保たれる。
    if (externalState && externalState !== lastAppliedExternalState) {
      applyingExternal = true;
      try {
        view.setState(externalState);
        lastAppliedExternalState = externalState;
        onValueApplied?.(view);
      } finally {
        applyingExternal = false;
      }
      return;
    }
    // externalState 経路を使わない時(ファイル open / 新規タブ / テンプレ)。
    // doc を全置換して history をリセットする。
    const current = view.state.doc.toString();
    if (current === value) return;
    applyingExternal = true;
    try {
      view.dispatch({ effects: langCompartment.reconfigure([]) });
      // 全置換 transaction は history に乗せない。これがないと、後で
      // history Compartment を reconfigure しても直前 1 ステップが
      // 残ってしまう環境がある(再現性は不安定だが安全側に倒す)。
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: value },
        annotations: Transaction.addToHistory.of(false),
      });
      // 新タブ・ファイル open での history クリア:Compartment を一度
      // 空にしてから再投入することで history extension のインスタンスを
      // 作り直し、前回のスタックを断ち切る。
      view.dispatch({ effects: historyCompartment.reconfigure([]) });
      view.dispatch({ effects: historyCompartment.reconfigure(history()) });
      view.dispatch({
        effects: langCompartment.reconfigure(langExtension(languageMode)),
      });
      // setState ルートと違い externalState は使っていないので、
      // 次回の比較でも外部復元が走らないように保持を null に揃える。
      lastAppliedExternalState = null;
      onValueApplied?.(view);
    } finally {
      applyingExternal = false;
    }
  });

  // mode prop が変わったら Compartment を reconfigure して即座に反映
  $effect(() => {
    if (!view) return;
    view.dispatch({
      effects: modeCompartment.reconfigure(modeExtension(mode)),
    });
  });

  // LSP クライアントや対象ファイルの切替を Compartment.reconfigure で反映
  $effect(() => {
    if (!view) return;
    view.dispatch({
      effects: lspCompartment.reconfigure(lspExtension(lspClient, filePath)),
    });
  });

  // languageMode 変更時に Typst 言語拡張を切替
  $effect(() => {
    if (!view) return;
    view.dispatch({
      effects: langCompartment.reconfigure(langExtension(languageMode)),
    });
  });
</script>

<div bind:this={host} class="cm-host"></div>

<style>
  /* CodeMirror の高さ伸長定石: host が高さを持ち、.cm-editor を 100% で満たす */
  .cm-host {
    flex: 1;
    min-height: 0;
    height: 100%;
  }

  .cm-host :global(.cm-editor) {
    height: 100%;
    outline: none;
  }

  .cm-host :global(.cm-scroller) {
    overflow: auto;
  }

  /*
   * LSP の hover / signature help / completion ツールチップが長い時、
   * 画面外まで伸びてスクロールできない問題を回避する。
   */
  .cm-host :global(.cm-tooltip) {
    max-width: 60vw;
    max-height: 50vh;
    overflow: auto;
    background: var(--bg-elevated-1);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    border-radius: 4px;
  }

  .cm-host :global(.cm-tooltip-hover) {
    padding: 6px 10px;
    font-size: 12px;
    line-height: 1.55;
  }

  /* hover tooltip 内の Markdown を読みやすくする(@codemirror/lsp-client が
     `marked` で HTML 化したもの。.cm-lsp-hover-tooltip 配下に通常の
     <h1..6>, <p>, <code>, <pre>, <ul>, <ol>, <a>, <hr> などが出てくる) */
  .cm-host :global(.cm-lsp-hover-tooltip) {
    color: var(--text-primary);
  }
  .cm-host :global(.cm-lsp-hover-tooltip > *:first-child) {
    margin-top: 0;
  }
  .cm-host :global(.cm-lsp-hover-tooltip > *:last-child) {
    margin-bottom: 0;
  }
  .cm-host :global(.cm-lsp-hover-tooltip h1),
  .cm-host :global(.cm-lsp-hover-tooltip h2),
  .cm-host :global(.cm-lsp-hover-tooltip h3),
  .cm-host :global(.cm-lsp-hover-tooltip h4),
  .cm-host :global(.cm-lsp-hover-tooltip h5),
  .cm-host :global(.cm-lsp-hover-tooltip h6) {
    margin: 8px 0 4px;
    font-weight: 600;
    color: var(--text-strong);
  }
  .cm-host :global(.cm-lsp-hover-tooltip h1) { font-size: 14px; }
  .cm-host :global(.cm-lsp-hover-tooltip h2) { font-size: 13px; }
  .cm-host :global(.cm-lsp-hover-tooltip h3),
  .cm-host :global(.cm-lsp-hover-tooltip h4),
  .cm-host :global(.cm-lsp-hover-tooltip h5),
  .cm-host :global(.cm-lsp-hover-tooltip h6) { font-size: 12px; }

  .cm-host :global(.cm-lsp-hover-tooltip p) {
    margin: 4px 0;
  }
  .cm-host :global(.cm-lsp-hover-tooltip ul),
  .cm-host :global(.cm-lsp-hover-tooltip ol) {
    margin: 4px 0;
    padding-left: 20px;
  }
  .cm-host :global(.cm-lsp-hover-tooltip li) {
    margin: 1px 0;
  }
  .cm-host :global(.cm-lsp-hover-tooltip a) {
    color: var(--syntax-function);
    text-decoration: underline;
  }
  .cm-host :global(.cm-lsp-hover-tooltip hr) {
    border: none;
    border-top: 1px solid var(--border);
    margin: 6px 0;
  }
  .cm-host :global(.cm-lsp-hover-tooltip code) {
    padding: 0 4px;
    background: var(--bg-elevated-2);
    border-radius: 3px;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 11.5px;
    color: var(--syntax-string);
  }
  .cm-host :global(.cm-lsp-hover-tooltip pre) {
    padding: 6px 8px;
    margin: 4px 0;
    background: var(--bg-elevated-2);
    border-radius: 3px;
    overflow-x: auto;
    line-height: 1.45;
  }
  /* pre 内の code は背景を二重にしないため透明化 */
  .cm-host :global(.cm-lsp-hover-tooltip pre code) {
    padding: 0;
    background: transparent;
    color: inherit;
    font-size: 11.5px;
  }
  .cm-host :global(.cm-lsp-hover-tooltip blockquote) {
    margin: 4px 0;
    padding: 2px 8px;
    border-left: 2px solid var(--border-strong);
    color: var(--text-secondary);
  }
  .cm-host :global(.cm-lsp-hover-tooltip table) {
    border-collapse: collapse;
    margin: 4px 0;
  }
  .cm-host :global(.cm-lsp-hover-tooltip th),
  .cm-host :global(.cm-lsp-hover-tooltip td) {
    border: 1px solid var(--border);
    padding: 2px 6px;
  }
  .cm-host :global(.cm-lsp-hover-tooltip img) {
    max-width: 100%;
    height: auto;
  }

  /* hover 以外の tooltip(signature help / completion 説明文)も
     pre / code は折り返す(横スクロールが出ないように) */
  .cm-host :global(.cm-tooltip pre),
  .cm-host :global(.cm-tooltip code) {
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
