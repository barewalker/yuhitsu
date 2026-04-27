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
  import type { EditorMode } from "$lib/settings";

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

  type Props = {
    value: string;
    mode?: EditorMode;
    onChange?: (next: string) => void;
  };

  let { value, mode = "default", onChange }: Props = $props();

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
        lineNumbers(),
        highlightActiveLine(),
        highlightActiveLineGutter(),
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        typst(),
        // typst() の LanguageSupport 内部に同梱されている TypstHighlightSytle
        // (ライト前提・heading が黒)を上書きするため最高優先度で当てる
        Prec.highest(syntaxHighlighting(highlightStyle)),
        theme,
        EditorView.lineWrapping,
        updateListener,
      ],
    });

    view = new EditorView({ state, parent: host });
  });

  onDestroy(() => {
    view?.destroy();
    view = null;
  });

  // 親から value が差し替わった時(ファイル open など)だけドキュメントを置き換える
  $effect(() => {
    if (!view) return;
    const current = view.state.doc.toString();
    if (current === value) return;
    applyingExternal = true;
    try {
      view.dispatch({
        changes: { from: 0, to: current.length, insert: value },
      });
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
</style>
