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
  import { typst } from "codemirror-lang-typst";
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
    // [Phase 1 暫定 2026-05-02] Typst 用に `codemirror-lang-typst` v0.4.0 を
    // 採用していたが、WASM Typst パーサが「単一 transaction 内に複数 changes」
    // のケースで panic し、エディタの transaction を巻き戻す不具合がある
    // (上流 issue #5、未修正)。具体症状はファイル末尾 #figure の caption 行
    // 削除で「行が復活 + undo 不可」。Phase 1 段階では構文木を必要とする機能
    // は無いので、当面は plain で運用。Phase 2 で StreamLanguage ベースの
    // 簡易ハイライタを自作するか、上流修正待ちで戻すかを判断する。
    return [];
    // return [typst(), Prec.highest(syntaxHighlighting(highlightStyle))];
  }

  const theme = EditorView.theme(
    {
      "&": {
        backgroundColor: "var(--bg-base)",
        color: "var(--text-primary)",
        fontSize: "14px",
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
      if (!update.docChanged) return;
      if (applyingExternal) return;
      onChange?.(update.state.doc.toString());
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
    padding: 4px 8px;
    font-size: 12px;
    line-height: 1.5;
  }

  .cm-host :global(.cm-tooltip-hover img) {
    max-width: 100%;
    height: auto;
  }

  .cm-host :global(.cm-tooltip-hover pre),
  .cm-host :global(.cm-tooltip-hover code) {
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
