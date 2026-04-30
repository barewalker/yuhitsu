<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    Compartment,
    EditorState,
    Prec,
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
    { tag: t.heading, color: "#e5c07b", fontWeight: "bold" },
    { tag: t.heading1, color: "#e5c07b", fontWeight: "bold" },
    { tag: t.heading2, color: "#e5c07b", fontWeight: "bold" },
    { tag: t.heading3, color: "#e5c07b", fontWeight: "bold" },
    { tag: t.heading4, color: "#e5c07b", fontWeight: "bold" },
    { tag: t.strong, color: "#e6e6e6", fontWeight: "bold" },
    { tag: t.emphasis, color: "#e6e6e6", fontStyle: "italic" },
    { tag: t.link, color: "#61afef", textDecoration: "underline" },
    { tag: t.url, color: "#61afef", textDecoration: "underline" },
    { tag: t.monospace, color: "#98c379" },
    { tag: t.literal, color: "#98c379" },
    { tag: t.string, color: "#98c379" },
    { tag: [t.keyword, t.controlKeyword, t.definitionKeyword, t.modifier], color: "#c678dd" },
    { tag: t.function(t.variableName), color: "#61afef" },
    { tag: t.variableName, color: "#e06c75" },
    { tag: t.propertyName, color: "#e06c75" },
    { tag: t.labelName, color: "#d19a66" },
    { tag: t.number, color: "#d19a66" },
    { tag: t.bool, color: "#d19a66" },
    { tag: [t.atom, t.null], color: "#56b6c2" },
    { tag: t.operator, color: "#56b6c2" },
    { tag: [t.punctuation, t.bracket, t.brace, t.paren], color: "#56b6c2" },
    { tag: t.comment, color: "#7f848e", fontStyle: "italic" },
    { tag: t.escape, color: "#56b6c2" },
    { tag: t.typeName, color: "#e5c07b" },
    { tag: t.tagName, color: "#e06c75" },
    { tag: t.attributeName, color: "#d19a66" },
    { tag: t.meta, color: "#7f848e" },
    { tag: t.invalid, color: "#ff5555" },
    // codemirror-lang-typst 固有のタグマッピング:
    //   ListMarker(`-`) / EnumMarker(`+`)    → t.list
    //   TermMarker(`/`)                       → t.definitionOperator
    //   見出しの `=` 等                        → t.processingInstruction
    { tag: t.list, color: "#56b6c2" },
    { tag: t.definitionOperator, color: "#56b6c2" },
    { tag: t.processingInstruction, color: "#56b6c2" },
    // 同パッケージは識別子に t.name も使う(関数呼び出しではない裸の参照など)
    { tag: t.name, color: "#abb2bf" },
    { tag: [t.moduleKeyword, t.operatorKeyword], color: "#c678dd" },
  ]);

  export type LanguageMode = "typst" | "plain";

  type Props = {
    value: string;
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

  function langExtension(target: LanguageMode): Extension {
    if (target === "plain") return [];
    return [typst(), Prec.highest(syntaxHighlighting(highlightStyle))];
  }

  const theme = EditorView.theme(
    {
      "&": {
        backgroundColor: "#1e1e1e",
        color: "#e6e6e6",
        fontSize: "14px",
      },
      ".cm-scroller": {
        fontFamily:
          "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
        lineHeight: "1.5",
      },
      ".cm-content": {
        caretColor: "#e6e6e6",
        padding: "8px 0",
      },
      ".cm-gutters": {
        backgroundColor: "#252525",
        color: "#7a7a7a",
        border: "none",
      },
      ".cm-activeLine": {
        backgroundColor: "#262626",
      },
      ".cm-activeLineGutter": {
        backgroundColor: "#2a2a2a",
        color: "#c0c0c0",
      },
      ".cm-cursor, .cm-dropCursor": {
        borderLeftColor: "#e6e6e6",
      },
      "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": {
        backgroundColor: "#3a4a66",
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
        history(),
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
    const current = view.state.doc.toString();
    if (current === value) return;
    applyingExternal = true;
    try {
      view.dispatch({ effects: langCompartment.reconfigure([]) });
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: value },
      });
      view.dispatch({
        effects: langCompartment.reconfigure(langExtension(languageMode)),
      });
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
    background: #232323;
    border: 1px solid #3a3a3a;
    color: #d0d0d0;
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
