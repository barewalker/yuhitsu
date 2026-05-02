<!--
  コマンドパレット。VSCode の Ctrl+Shift+P / F1 と同等。
  - コマンドカタログ($lib/commands.ts)を fuzzy 検索 → 選択 → 実行
  - 各コマンドの effective key(override 優先、なければ defaultKey 先頭)を
    右側に表示
  - needsEditor のコマンドは editor が無い時にグレーアウトして実行不可
  - キーボードのみで完結:↓↑ で選択移動、Enter で実行、Esc で閉じる
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { COMMANDS, COMMAND_IDS, type CommandId } from "$lib/commands";
  import { t, tEn, tAlias } from "$lib/i18n/index.svelte";

  type Props = {
    /** override キーバインド(設定値)。表示用に right column に出す */
    keybindings: Record<string, string>;
    /** コマンド実行時のエディタ依存判定。editor 不在時に needsEditor: true を弾く */
    editorAvailable: boolean;
    onSelect: (id: CommandId) => void;
    onClose: () => void;
  };

  let {
    keybindings,
    editorAvailable,
    onSelect,
    onClose,
  }: Props = $props();

  let query = $state("");
  let activeIndex = $state(0);
  let inputEl = $state<HTMLInputElement | null>(null);
  let listEl = $state<HTMLDivElement | null>(null);

  type Item = {
    id: CommandId;
    /** 表示用ラベル(現在 locale) */
    label: string;
    /** 検索用の英語ラベル。日本語 locale でも英単語(open / save / bold 等)で
        引けるよう、表示とは別に保持する */
    labelEn: string;
    /** 検索エイリアス(かな・ローマ字)。i18n の commandAlias.<key> に
        スペース区切りで定義。漢字変換前のかなや訓令式ローマ字で引ける。 */
    alias: string;
    keyDisplay: string;
    enabled: boolean;
    /** 検索マッチのスコア(0 なら除外、大きいほど優先) */
    score: number;
  };

  // 全コマンドの基本情報。filter/scoring は別 derived で行う。
  const baseItems = $derived<Item[]>(
    COMMAND_IDS.filter(
      // ボタンとして意味のないものをパレットから除外する余地はあるが、
      // 現状すべて含める(過剰な除外は発見性を下げる)
      (id) => COMMANDS[id] !== undefined,
    ).map((id) => {
      const def = COMMANDS[id];
      const enabled = !def.needsEditor || editorAvailable;
      // labelKey は "command.openFile" 形式。alias 用キーは "commandAlias.openFile"
      const aliasKey = def.labelKey.replace(/^command\./, "commandAlias.");
      return {
        id,
        label: t(def.labelKey),
        labelEn: tEn(def.labelKey),
        alias: tAlias(aliasKey),
        keyDisplay: effectiveKeyDisplay(id),
        enabled,
        score: 0,
      };
    }),
  );

  const filtered = $derived<Item[]>(filterAndScore());

  function filterAndScore(): Item[] {
    const q = query.trim().toLowerCase();
    if (q.length === 0) {
      // 検索なし:そのまま全件(enabled が先頭、disabled が末尾)
      return baseItems
        .map((it) => ({ ...it, score: it.enabled ? 1 : 0 }))
        .sort((a, b) => Number(b.enabled) - Number(a.enabled));
    }
    // 検索あり:tokens 全部にヒットすればスコア計算、ヒットしなければ除外。
    const tokens = q.split(/\s+/).filter(Boolean);
    return baseItems
      .map((it) => ({ ...it, score: scoreItem(it, tokens) }))
      .filter((it) => it.score > 0)
      .sort((a, b) => {
        // enabled 優先、次に score 降順
        if (a.enabled !== b.enabled) return Number(b.enabled) - Number(a.enabled);
        return b.score - a.score;
      });
  }

  function scoreItem(it: Item, tokens: string[]): number {
    const lower = it.label.toLowerCase();
    const lowerEn = it.labelEn.toLowerCase();
    const idLower = it.id.toLowerCase();
    const aliasLower = it.alias.toLowerCase();
    let score = 0;
    for (const token of tokens) {
      const inLabel = lower.indexOf(token);
      const inLabelEn = lowerEn.indexOf(token);
      const inAlias = aliasLower.length > 0 ? aliasLower.indexOf(token) : -1;
      const inId = idLower.indexOf(token);
      if (inLabel < 0 && inLabelEn < 0 && inAlias < 0 && inId < 0) return 0;
      // 優先順位:表示ラベル先頭 > かな/ローマ字 alias > 英語ラベル > 部分一致 > ID
      // alias は IME 不要で打てる重要経路なので英語ラベル先頭より少し先に置く。
      if (inLabel === 0) score += 100;
      else if (inAlias >= 0) score += 80 - Math.min(60, inAlias);
      else if (inLabelEn === 0) score += 75;
      else if (inLabel > 0) score += 55 - Math.min(45, inLabel);
      else if (inLabelEn > 0) score += 45 - Math.min(35, inLabelEn);
      else if (inId === 0) score += 35;
      else score += 15 - Math.min(10, inId);
    }
    return score;
  }

  function effectiveKeyDisplay(id: CommandId): string {
    const override = keybindings[id];
    if (typeof override === "string" && override.length > 0)
      return formatKey(override);
    const def = COMMANDS[id].defaultKey;
    const raw = Array.isArray(def) ? (def[0] ?? "") : (def ?? "");
    return formatKey(raw);
  }

  // "Mod-Shift-b" → "Ctrl+Shift+B"(KeybindingsDialog と同じ流儀)
  function formatKey(spec: string): string {
    if (!spec) return "";
    return spec.replaceAll("Mod", "Ctrl").replaceAll("-", "+");
  }

  // フィルタ結果が変わったら activeIndex を範囲内に。query 変更直後は 0 に。
  $effect(() => {
    void filtered;
    void query;
    if (activeIndex >= filtered.length) activeIndex = 0;
  });

  function clamp(v: number): number {
    if (filtered.length === 0) return 0;
    return Math.max(0, Math.min(filtered.length - 1, v));
  }

  function moveActive(delta: number) {
    if (filtered.length === 0) return;
    activeIndex = clamp(activeIndex + delta);
    scrollActiveIntoView();
  }

  function scrollActiveIntoView() {
    queueMicrotask(() => {
      const el = listEl?.querySelector<HTMLElement>(`[data-idx="${activeIndex}"]`);
      el?.scrollIntoView({ block: "nearest" });
    });
  }

  function commit(item: Item | undefined) {
    if (!item || !item.enabled) return;
    onSelect(item.id);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      moveActive(1);
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      moveActive(-1);
      return;
    }
    if (e.key === "PageDown") {
      e.preventDefault();
      moveActive(8);
      return;
    }
    if (e.key === "PageUp") {
      e.preventDefault();
      moveActive(-8);
      return;
    }
    if (e.key === "Home") {
      e.preventDefault();
      activeIndex = 0;
      scrollActiveIntoView();
      return;
    }
    if (e.key === "End") {
      e.preventDefault();
      activeIndex = Math.max(0, filtered.length - 1);
      scrollActiveIntoView();
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      commit(filtered[activeIndex]);
      return;
    }
  }

  onMount(() => {
    inputEl?.focus();
    inputEl?.select();
  });
</script>

<div class="overlay" onclick={onClose} role="presentation">
  <div
    class="palette"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-label={t("commandPalette.ariaLabel")}
    tabindex="-1"
  >
    <input
      bind:this={inputEl}
      bind:value={query}
      onkeydown={onKeydown}
      placeholder={t("commandPalette.placeholder")}
      class="palette-input"
      type="text"
      aria-label={t("commandPalette.placeholder")}
    />
    <div class="palette-list" bind:this={listEl} role="listbox">
      {#each filtered as item, i (item.id)}
        <button
          type="button"
          class="palette-item"
          class:active={i === activeIndex}
          class:disabled={!item.enabled}
          data-idx={i}
          disabled={!item.enabled}
          onclick={() => commit(item)}
          onmousemove={() => (activeIndex = i)}
        >
          <span class="palette-label">{item.label}</span>
          {#if item.keyDisplay}
            <span class="palette-key">{item.keyDisplay}</span>
          {/if}
        </button>
      {:else}
        <div class="palette-empty">{t("commandPalette.empty")}</div>
      {/each}
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 300;
    background: rgba(0, 0, 0, 0.35);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 80px;
  }
  .palette {
    width: min(640px, 92vw);
    max-height: 60vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-elevated-1);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    overflow: hidden;
  }
  .palette-input {
    width: 100%;
    box-sizing: border-box;
    padding: 8px 12px;
    background: var(--bg-base);
    color: var(--text-primary);
    border: none;
    border-bottom: 1px solid var(--border);
    font: inherit;
    font-size: 14px;
  }
  .palette-input:focus {
    outline: none;
  }
  .palette-list {
    flex: 1 1 auto;
    overflow-y: auto;
    min-height: 0;
    padding: 4px 0;
  }
  .palette-item {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 6px 12px;
    background: transparent;
    color: var(--text-primary);
    border: none;
    text-align: left;
    cursor: pointer;
    font: inherit;
    font-size: 13px;
  }
  .palette-item.active:not(.disabled) {
    background: var(--accent-bg-subtle);
  }
  .palette-item.disabled {
    color: var(--text-disabled);
    cursor: not-allowed;
  }
  .palette-label {
    flex: 1 1 auto;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .palette-key {
    flex: 0 0 auto;
    color: var(--text-tertiary);
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 11px;
  }
  .palette-empty {
    padding: 16px 12px;
    color: var(--text-disabled);
    text-align: center;
    font-size: 12px;
  }
</style>
