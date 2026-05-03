<!--
  三点リーダー(メインメニュー)。ツールバー左端に常駐するボタンを押すと、
  下にカテゴリ別のドロップダウンメニューが開く。
  - グループ:File / Edit / View / Tools(commands.ts の MAIN_MENU_GROUPS)
  - 各項目に label + 効くキーバインド表示
  - needsEditor のコマンドは editor 不在時にグレーアウト
  - 外側クリック / Esc / 項目選択 で閉じる
  全 OS 共通。macOS のネイティブメニューバーとは独立して動く(将来は併設可)。
-->
<script lang="ts">
  import { onMount } from "svelte";
  import Menu from "@lucide/svelte/icons/menu";
  import {
    COMMANDS,
    MAIN_MENU_GROUPS,
    type CommandId,
  } from "$lib/commands";
  import { t } from "$lib/i18n/index.svelte";

  type Props = {
    keybindings: Record<string, string>;
    editorAvailable: boolean;
    onSelect: (id: CommandId) => void;
  };

  let { keybindings, editorAvailable, onSelect }: Props = $props();

  let open = $state(false);
  let buttonEl = $state<HTMLButtonElement | null>(null);
  let menuEl = $state<HTMLDivElement | null>(null);

  function toggle() {
    open = !open;
  }

  function close() {
    open = false;
    queueMicrotask(() => buttonEl?.focus());
  }

  function effectiveKeyDisplay(id: CommandId): string {
    const override = keybindings[id];
    if (typeof override === "string" && override.length > 0)
      return formatKey(override);
    const def = COMMANDS[id].defaultKey;
    const raw = Array.isArray(def) ? (def[0] ?? "") : (def ?? "");
    return formatKey(raw);
  }

  function formatKey(spec: string): string {
    if (!spec) return "";
    return spec.replaceAll("Mod", "Ctrl").replaceAll("-", "+");
  }

  function isEnabled(id: CommandId): boolean {
    const def = COMMANDS[id];
    return !def.needsEditor || editorAvailable;
  }

  function commit(id: CommandId) {
    if (!isEnabled(id)) return;
    open = false;
    onSelect(id);
  }

  // 外側クリック検知。capture 段階で受け取り、ボタン自身とメニュー内は無視。
  function onDocPointerDown(e: PointerEvent) {
    if (!open) return;
    const t = e.target as Node | null;
    if (!t) return;
    if (buttonEl?.contains(t)) return;
    if (menuEl?.contains(t)) return;
    close();
  }

  function onDocKeydown(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    }
  }

  onMount(() => {
    document.addEventListener("pointerdown", onDocPointerDown, true);
    document.addEventListener("keydown", onDocKeydown);
    return () => {
      document.removeEventListener("pointerdown", onDocPointerDown, true);
      document.removeEventListener("keydown", onDocKeydown);
    };
  });
</script>

<div class="main-menu">
  <button
    bind:this={buttonEl}
    class="icon-btn menu-trigger"
    class:open
    type="button"
    aria-label={t("menu.ariaLabel")}
    aria-haspopup="menu"
    aria-expanded={open}
    title={t("menu.open")}
    onclick={toggle}
  >
    <Menu size={18} />
  </button>

  {#if open}
    <div
      bind:this={menuEl}
      class="menu-dropdown"
      role="menu"
      aria-label={t("menu.ariaLabel")}
    >
      {#each MAIN_MENU_GROUPS as group, gi (group.id)}
        {#if gi > 0}
          <div class="menu-divider" role="separator"></div>
        {/if}
        <div class="menu-group-label">{t(group.labelKey)}</div>
        {#each group.items as id (id)}
          {@const def = COMMANDS[id]}
          {@const enabled = isEnabled(id)}
          {@const keyDisplay = effectiveKeyDisplay(id)}
          <button
            type="button"
            class="menu-item"
            class:disabled={!enabled}
            disabled={!enabled}
            role="menuitem"
            onclick={() => commit(id)}
          >
            <span class="menu-label">{t(def.labelKey)}</span>
            {#if keyDisplay}
              <span class="menu-key">{keyDisplay}</span>
            {/if}
          </button>
        {/each}
      {/each}
    </div>
  {/if}
</div>

<style>
  .main-menu {
    position: relative;
  }

  .menu-trigger.open {
    background: var(--bg-elevated-2);
  }

  .menu-dropdown {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 250;
    min-width: 240px;
    max-height: 70vh;
    overflow-y: auto;
    padding: 4px 0;
    background: var(--bg-elevated-1);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.35);
    font-size: 13px;
  }

  .menu-group-label {
    padding: 4px 12px 2px;
    color: var(--text-tertiary);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .menu-divider {
    height: 1px;
    margin: 4px 0;
    background: var(--border);
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 5px 12px;
    background: transparent;
    color: inherit;
    border: none;
    text-align: left;
    cursor: pointer;
    font: inherit;
  }

  .menu-item:hover:not(.disabled) {
    background: var(--accent-bg-subtle);
  }

  .menu-item.disabled {
    color: var(--text-disabled);
    cursor: not-allowed;
  }

  .menu-label {
    flex: 1 1 auto;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .menu-key {
    flex: 0 0 auto;
    color: var(--text-tertiary);
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 11px;
  }
</style>
