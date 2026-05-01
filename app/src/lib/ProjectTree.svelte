<!--
  プロジェクトビュー用の再帰ツリー描画コンポーネント。
  - フォルダはクリックで折りたたみ / 展開
  - ファイルはクリックで親に通知(親がエディタに開く)
  - 現在開いているファイルはハイライト
-->
<script lang="ts">
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import Folder from "@lucide/svelte/icons/folder";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import File from "@lucide/svelte/icons/file";
  import FileText from "@lucide/svelte/icons/file-text";
  import FileType from "@lucide/svelte/icons/file-type";
  import ImageIcon from "@lucide/svelte/icons/image";
  import type { DirEntry } from "$lib/project";
  import Self from "./ProjectTree.svelte";

  type Props = {
    entries: DirEntry[];
    activePath: string | null;
    onOpenFile: (path: string) => void;
    /** 折り畳み状態は親で集約管理(再マウント時にも保たせるため) */
    expanded: Record<string, boolean>;
    onToggleExpanded: (path: string) => void;
    depth?: number;
  };

  let {
    entries,
    activePath,
    onOpenFile,
    expanded,
    onToggleExpanded,
    depth = 0,
  }: Props = $props();

  const IMAGE_EXT = /\.(png|jpe?g|gif|svg|webp|avif)$/i;
  const TYPST_EXT = /\.typ$/i;
  const PDF_EXT = /\.pdf$/i;

  function fileIcon(name: string) {
    if (TYPST_EXT.test(name)) return FileText;
    if (PDF_EXT.test(name)) return FileType;
    if (IMAGE_EXT.test(name)) return ImageIcon;
    return File;
  }
</script>

<ul class="tree" style:--depth={depth}>
  {#each entries as entry (entry.path)}
    <li>
      {#if entry.is_dir}
        {@const isOpen = expanded[entry.path] ?? false}
        <button
          class="row dir"
          style:padding-left={`${8 + depth * 12}px`}
          onclick={() => onToggleExpanded(entry.path)}
        >
          {#if isOpen}
            <ChevronDown size={14} class="chev" />
          {:else}
            <ChevronRight size={14} class="chev" />
          {/if}
          {#if isOpen}
            <FolderOpen size={14} class="ic" />
          {:else}
            <Folder size={14} class="ic" />
          {/if}
          <span class="label">{entry.name}</span>
        </button>
        {#if isOpen && entry.children && entry.children.length > 0}
          <Self
            entries={entry.children}
            {activePath}
            {onOpenFile}
            {expanded}
            {onToggleExpanded}
            depth={depth + 1}
          />
        {/if}
      {:else}
        {@const Icon = fileIcon(entry.name)}
        <button
          class="row file"
          class:active={activePath === entry.path}
          style:padding-left={`${8 + depth * 12 + 14}px`}
          onclick={() => onOpenFile(entry.path)}
        >
          <Icon size={14} class="ic" />
          <span class="label">{entry.name}</span>
        </button>
      {/if}
    </li>
  {/each}
</ul>

<style>
  .tree {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    padding: 3px 8px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 12px;
    text-align: left;
    cursor: pointer;
    line-height: 1.4;
  }

  .row:hover {
    background: var(--bg-elevated-3);
  }

  .row.active {
    background: var(--accent-bg-subtle);
    color: var(--text-strong);
  }

  .row :global(.chev) {
    color: var(--text-faint);
    flex-shrink: 0;
  }

  .row :global(.ic) {
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  .row.active :global(.ic) {
    color: var(--text-strong);
  }

  .label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
