<!--
  ツールバー編集ダイアログ。

  - 上段:現在のツールバー(pointer events で D&D 並び替え + × で削除)
  - 中段:追加できる全コマンド一覧 + 区切り線(クリックで末尾追加)
  - 下段:プリセット 3 種(標準 / ミニマル / 論文寄り)
  D&D は HTML5 Drag and Drop API ではなく pointer events で自前実装する。
  WebKitGTK 上で HTML5 D&D の ghost が hit testing を妨げる既知の挙動を
  避けるため(タブ並び替えと同じ流儀)。
-->
<script lang="ts">
  import {
    COMMANDS,
    COMMAND_IDS,
    TOOLBAR_PRESETS,
    type CommandId,
    type ToolbarItem,
  } from "$lib/commands";
  import { t } from "$lib/i18n/index.svelte";
  import X from "@lucide/svelte/icons/x";

  type Props = {
    items: ToolbarItem[];
    onUpdate: (next: ToolbarItem[]) => void;
    onClose: () => void;
  };
  let { items, onUpdate, onClose }: Props = $props();

  // pointer events ベースの D&D 状態。タブ並び替えと同じパターン。
  let draggingIdx = $state<number | null>(null);
  let dragOverIdx = $state<number | null>(null);
  let pendingIdx: number | null = null;
  let pendingPointerId: number | null = null;
  let pendingStartX = 0;
  let pendingStartY = 0;
  const DRAG_THRESHOLD_PX = 5;

  function moveItem(from: number, to: number) {
    if (from === to) return;
    const next = [...items];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    onUpdate(next);
  }

  function removeItem(idx: number) {
    onUpdate(items.filter((_, i) => i !== idx));
  }

  function appendCommand(id: CommandId) {
    onUpdate([...items, id]);
  }

  function appendDivider() {
    onUpdate([...items, "divider"]);
  }

  function applyPreset(presetId: string) {
    const preset = TOOLBAR_PRESETS.find((p) => p.id === presetId);
    if (preset) onUpdate([...preset.items]);
  }

  function onItemPointerDown(e: PointerEvent, idx: number) {
    if (e.button !== 0) return;
    e.preventDefault();
    pendingIdx = idx;
    pendingPointerId = e.pointerId;
    pendingStartX = e.clientX;
    pendingStartY = e.clientY;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onItemPointerMove(e: PointerEvent) {
    if (pendingPointerId === null || e.pointerId !== pendingPointerId) return;
    if (draggingIdx === null) {
      const dx = e.clientX - pendingStartX;
      const dy = e.clientY - pendingStartY;
      if (Math.hypot(dx, dy) < DRAG_THRESHOLD_PX) return;
      draggingIdx = pendingIdx;
    }
    // hover 中の要素を判定して dragOverIdx を更新
    const el = document.elementFromPoint(e.clientX, e.clientY);
    const itemEl = (el as HTMLElement | null)?.closest(
      "[data-item-idx]",
    ) as HTMLElement | null;
    const idxStr = itemEl?.dataset.itemIdx;
    const idx = idxStr !== undefined ? parseInt(idxStr, 10) : null;
    if (idx !== null && idx !== draggingIdx) {
      dragOverIdx = idx;
    } else {
      dragOverIdx = null;
    }
  }

  function onItemPointerUp(e: PointerEvent) {
    if (pendingPointerId === null || e.pointerId !== pendingPointerId) return;
    const wasDragging = draggingIdx !== null;
    const src = draggingIdx;
    const target = dragOverIdx;
    draggingIdx = null;
    dragOverIdx = null;
    pendingIdx = null;
    pendingPointerId = null;
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      // capture 解除に失敗しても致命的でない
    }
    if (wasDragging && src !== null && target !== null) {
      moveItem(src, target);
    }
  }

  // 「× 削除」ボタンの click が親要素 li の pointer 系列を巻き込まないよう
  // stopPropagation する(D&D の判定ロジックを汚さない)。
  function onRemoveClick(e: MouseEvent, idx: number) {
    e.stopPropagation();
    removeItem(idx);
  }
</script>

<div class="overlay" role="presentation" onclick={onClose}></div>
<div
  class="dialog"
  role="dialog"
  aria-modal="true"
  aria-label={t("toolbarEdit.ariaLabel")}
>
  <div class="header">
    <span class="title">{t("toolbarEdit.title")}</span>
    <button class="close-btn" onclick={onClose} aria-label={t("toolbarEdit.close")}
      >×</button
    >
  </div>
  <div class="body">
    <div class="section-title">{t("toolbarEdit.current")}</div>
    <div class="toolbar-row">
      {#each items as item, idx (idx + ":" + item)}
        {@const isDivider = item === "divider"}
        {@const def = isDivider ? null : COMMANDS[item as CommandId]}
        <div
          class="item"
          class:divider={isDivider}
          class:dragging={draggingIdx === idx}
          class:drag-over={dragOverIdx === idx && draggingIdx !== idx}
          data-item-idx={idx}
          role="button"
          tabindex="0"
          onpointerdown={(e) => onItemPointerDown(e, idx)}
          onpointermove={onItemPointerMove}
          onpointerup={onItemPointerUp}
          onpointercancel={onItemPointerUp}
          title={isDivider ? t("toolbarEdit.divider") : def ? t(def.labelKey) : ""}
        >
          {#if isDivider}
            <span class="divider-mark">|</span>
          {:else if def}
            {@const Icon = def.icon}
            <Icon size={14} class="ic" />
          {/if}
          <button
            class="remove"
            onclick={(e) => onRemoveClick(e, idx)}
            aria-label={t("toolbarEdit.remove")}
          >
            <X size={10} />
          </button>
        </div>
      {/each}
    </div>

    <div class="section-title">{t("toolbarEdit.add")}</div>
    <div class="add-grid">
      <button class="add-btn" onclick={appendDivider}>
        <span class="divider-mark">|</span>
        {t("toolbarEdit.addDivider")}
      </button>
      {#each COMMAND_IDS as id (id)}
        {@const def = COMMANDS[id]}
        {@const Icon = def.icon}
        <button class="add-btn" onclick={() => appendCommand(id)}>
          <Icon size={14} class="ic" />
          {t(def.labelKey)}
        </button>
      {/each}
    </div>

    <div class="section-title">{t("toolbarEdit.presets")}</div>
    <div class="preset-row">
      {#each TOOLBAR_PRESETS as preset (preset.id)}
        <button class="preset-btn" onclick={() => applyPreset(preset.id)}>
          {t(preset.labelKey)}
        </button>
      {/each}
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 250;
    background: rgba(0, 0, 0, 0.4);
  }
  .dialog {
    position: fixed;
    z-index: 251;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    /* current toolbar を一段で並べやすくするため横幅を広めに取る */
    width: min(95vw, 1100px);
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-elevated-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    color: var(--text-primary);
  }
  .header {
    display: flex;
    align-items: center;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .title {
    flex: 1;
    font-weight: 600;
    font-size: 13px;
    color: var(--text-strong);
  }
  .close-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--text-muted);
    font-size: 18px;
    padding: 0 6px;
    line-height: 1;
  }
  .close-btn:hover {
    color: var(--text-primary);
  }
  .body {
    padding: 8px 12px 12px;
    overflow: auto;
    font-size: 12px;
  }
  .section-title {
    margin-top: 8px;
    margin-bottom: 4px;
    color: var(--text-tertiary);
    font-size: 11px;
    font-weight: 500;
  }

  /* 実際のツールバーと同じく横並び。1 段で見たいので nowrap + 横スクロール
     にして、項目が多くても折り返さずに全体を眺められるようにする。 */
  .toolbar-row {
    display: flex;
    flex-wrap: nowrap;
    gap: 4px;
    padding: 6px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-base);
    min-height: 36px;
    overflow-x: auto;
  }
  /* current toolbar 内はアイコンのみ(実ツールバーと同じスリムさ)。
     × 削除ボタンは hover 時だけ姿を見せて、通常時は幅 0 で隠れる。 */
  .item {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 4px 5px;
    background: var(--bg-elevated-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: grab;
    user-select: none;
    -webkit-user-select: none;
    touch-action: none;
  }
  .item.divider {
    background: transparent;
    padding: 4px 3px;
  }
  .item:active {
    cursor: grabbing;
  }
  .item.dragging {
    opacity: 0.4;
  }
  /* 横並びでは「左に挿入」が直感的なので左ボーダーで drop 位置を示す */
  .item.drag-over {
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .divider-mark {
    color: var(--text-faint);
    font-weight: 600;
    line-height: 14px;
    width: 8px;
    text-align: center;
  }
  .item :global(.ic) {
    color: var(--text-secondary);
    flex-shrink: 0;
  }
  .remove {
    background: transparent;
    border: none;
    color: var(--text-faint);
    cursor: pointer;
    padding: 0;
    border-radius: 3px;
    display: inline-flex;
    align-items: center;
    width: 0;
    overflow: hidden;
    transition: width 0.1s;
  }
  .item:hover .remove {
    width: 14px;
  }
  .remove:hover {
    color: var(--status-error-strong);
  }

  .add-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 4px;
  }
  .add-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--bg-base);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px 8px;
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .add-btn:hover {
    background: var(--bg-elevated-3);
    color: var(--text-primary);
  }
  .add-btn :global(.ic) {
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  .preset-row {
    display: flex;
    gap: 6px;
  }
  .preset-btn {
    background: var(--bg-base);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px 12px;
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
  }
  .preset-btn:hover {
    background: var(--accent-bg-subtle);
    border-color: var(--accent);
    color: var(--text-primary);
  }
</style>
