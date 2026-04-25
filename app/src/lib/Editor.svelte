<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorState } from "@codemirror/state";
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

  type Props = {
    value: string;
    onChange?: (next: string) => void;
  };

  let { value, onChange }: Props = $props();

  let host: HTMLDivElement;
  let view: EditorView | null = null;
  // 外部 value 反映と updateListener のループを防ぐためのフラグ
  let applyingExternal = false;

  const theme = EditorView.theme(
    {
      "&": {
        height: "100%",
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
</script>

<div bind:this={host} class="cm-host"></div>

<style>
  .cm-host {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  .cm-host :global(.cm-editor) {
    flex: 1;
    min-height: 0;
    outline: none;
  }
</style>
