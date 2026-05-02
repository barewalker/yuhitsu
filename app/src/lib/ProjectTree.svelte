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
    /** 行の右クリックを親に通知。親が context menu を表示してファイル操作を行う。
     *  座標(client x,y)とエントリ自身を渡す。 */
    onContextMenu?: (e: MouseEvent, entry: DirEntry) => void;
    /** 絶対パス → 1 文字 git status code(?, M, A, D, R, U)。
     *  未指定時 / 空 map なら git バッジは出さない(non-repo フォルダ)。 */
    gitStatus?: Record<string, string>;
    depth?: number;
  };

  let {
    entries,
    activePath,
    onOpenFile,
    expanded,
    onToggleExpanded,
    onContextMenu,
    gitStatus = {},
    depth = 0,
  }: Props = $props();

  // status code → CSS クラス。色は CSS 側で var(...) で解決する。
  function statusClass(code: string | undefined): string {
    if (!code) return "";
    switch (code) {
      case "?":
        return "git-untracked";
      case "M":
        return "git-modified";
      case "A":
        return "git-added";
      case "D":
        return "git-deleted";
      case "R":
        return "git-renamed";
      case "U":
        return "git-unmerged";
      default:
        return "git-modified";
    }
  }

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
          oncontextmenu={(e) => {
            e.preventDefault();
            // row 自身でメニューを出すので、親 .project-body の
            // contextmenu(空白部分用)に bubble させない。bubble させると
            // 親側がルートフォルダで上書きしてしまう。
            e.stopPropagation();
            onContextMenu?.(e, entry);
          }}
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
            {onContextMenu}
            {gitStatus}
            depth={depth + 1}
          />
        {/if}
      {:else}
        {@const Icon = fileIcon(entry.name)}
        {@const code = gitStatus[entry.path]}
        <button
          class={`row file ${statusClass(code)}`}
          class:active={activePath === entry.path}
          style:padding-left={`${8 + depth * 12 + 14}px`}
          onclick={() => onOpenFile(entry.path)}
          oncontextmenu={(e) => {
            e.preventDefault();
            // row 自身でメニューを出すので、親 .project-body の
            // contextmenu(空白部分用)に bubble させない。bubble させると
            // 親側がルートフォルダで上書きしてしまう。
            e.stopPropagation();
            onContextMenu?.(e, entry);
          }}
        >
          <Icon size={14} class="ic" />
          <span class="label">{entry.name}</span>
          {#if code}
            <span class="git-badge" title={`git: ${code}`}>{code}</span>
          {/if}
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
    flex: 1;
  }

  /* git status バッジ。ラベル右に小さな 1 文字。色は CSS 変数で
     ライト/ダーク両対応。 */
  .git-badge {
    flex-shrink: 0;
    min-width: 12px;
    padding: 0 4px;
    font-size: 10px;
    font-weight: 700;
    line-height: 1.4;
    color: var(--text-tertiary);
    text-align: center;
  }

  /* status 別の色。各色は app.html の CSS 変数を使う。
     untracked=緑(新規追加候補)、modified=オレンジ(変更)、
     added=緑、deleted=赤、renamed=青、unmerged=赤。 */
  .row.git-untracked {
    color: var(--syntax-string);
  }
  .row.git-untracked .git-badge {
    color: var(--syntax-string);
  }
  .row.git-modified {
    color: var(--syntax-number);
  }
  .row.git-modified .git-badge {
    color: var(--syntax-number);
  }
  .row.git-added {
    color: var(--syntax-string);
  }
  .row.git-added .git-badge {
    color: var(--syntax-string);
  }
  .row.git-deleted .git-badge {
    color: var(--status-error-strong);
  }
  .row.git-deleted .label {
    text-decoration: line-through;
    color: var(--status-error-strong);
  }
  .row.git-renamed {
    color: var(--syntax-function);
  }
  .row.git-renamed .git-badge {
    color: var(--syntax-function);
  }
  .row.git-unmerged {
    color: var(--status-error-strong);
  }
  .row.git-unmerged .git-badge {
    color: var(--status-error-strong);
  }
</style>
