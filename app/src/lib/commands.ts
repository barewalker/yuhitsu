// Yuhitsu のコマンドカタログ。
// ID をキーにツールバー描画 / キーバインド / 設定永続化 / (将来) MCP ハンドラ
// が共通で参照する。新規コマンドは COMMAND_IDS / COMMANDS の両方に追加する。

import type { EditorView } from "@codemirror/view";
import type { LucideIcon } from "@lucide/svelte";
import Asterisk from "@lucide/svelte/icons/asterisk";
import Bold from "@lucide/svelte/icons/bold";
import BookOpen from "@lucide/svelte/icons/book-open";
import Code from "@lucide/svelte/icons/code";
import CodeXml from "@lucide/svelte/icons/code-xml";
import FileDown from "@lucide/svelte/icons/file-down";
import FilePlus from "@lucide/svelte/icons/file-plus";
import FilePlus2 from "@lucide/svelte/icons/file-plus-2";
import FolderOpen from "@lucide/svelte/icons/folder-open";
import FolderTree from "@lucide/svelte/icons/folder-tree";
import PanelLeft from "@lucide/svelte/icons/panel-left";
import X from "@lucide/svelte/icons/x";
import Heading1 from "@lucide/svelte/icons/heading-1";
import Heading2 from "@lucide/svelte/icons/heading-2";
import Heading3 from "@lucide/svelte/icons/heading-3";
import ImageIcon from "@lucide/svelte/icons/image";
import Italic from "@lucide/svelte/icons/italic";
import LinkIcon from "@lucide/svelte/icons/link";
import List from "@lucide/svelte/icons/list";
import ListOrdered from "@lucide/svelte/icons/list-ordered";
import PanelRight from "@lucide/svelte/icons/panel-right";
import Quote from "@lucide/svelte/icons/quote";
import Save from "@lucide/svelte/icons/save";
import SaveAll from "@lucide/svelte/icons/save-all";
import Sigma from "@lucide/svelte/icons/sigma";
import TableIcon from "@lucide/svelte/icons/table";
import {
  applyHeading,
  insertCodeBlock,
  insertFootnote,
  insertLink,
  insertQuote,
  insertTable,
  toggleBold,
  toggleBulletList,
  toggleInlineCode,
  toggleItalic,
  toggleMath,
  toggleNumberedList,
} from "$lib/editor-commands";

export const COMMAND_IDS = [
  "new-tab",
  "new-from-template",
  "open-file",
  "open-folder",
  "save",
  "save-as",
  "close-tab",
  "export-pdf",
  "bold",
  "italic",
  "heading-1",
  "heading-2",
  "heading-3",
  "bullet-list",
  "numbered-list",
  "math",
  "code-inline",
  "code-block",
  "link",
  "footnote",
  "quote",
  "image",
  "table",
  "bibliography",
  "toggle-project-view",
  "toggle-preview",
] as const;

export type CommandId = (typeof COMMAND_IDS)[number];

export function isCommandId(value: unknown): value is CommandId {
  return (
    typeof value === "string" &&
    (COMMAND_IDS as readonly string[]).includes(value)
  );
}

// ホスト側(+page.svelte)が提供する依存。エディタ操作以外のコマンド
// (ファイル開閉・PDF 出力・画像ピッカー)はここから呼ばれる。
export interface CommandContext {
  view: EditorView | null;
  openFile: () => void | Promise<void>;
  openFolder: () => void | Promise<void>;
  save: () => void | Promise<void>;
  saveAs: () => void | Promise<void>;
  exportPdf: () => void | Promise<void>;
  pickAndInsertImage: () => void | Promise<void>;
  pickAndInsertBibliography: () => void | Promise<void>;
  togglePreview: () => void | Promise<void>;
  toggleProjectView: () => void | Promise<void>;
  newTab: () => void | Promise<void>;
  newFromTemplate: () => void | Promise<void>;
  closeActiveTab: () => void | Promise<void>;
}

export interface CommandDef {
  id: CommandId;
  /** メニュー / ホバー / 設定画面で出す日本語ラベル */
  label: string;
  /** ツールバーに表示するアイコン(Lucide コンポーネント) */
  icon: LucideIcon;
  /** ボタンに当てる装飾クラス。装飾系は `fmt` 等 */
  buttonClass?: string;
  /** デフォルトキーバインド。"Mod-b" 形式(Mod は Ctrl/Cmd) */
  defaultKey?: string;
  /** EditorView を必要とするか。null のとき disabled / キーは無視 */
  needsEditor: boolean;
  run: (ctx: CommandContext) => void | Promise<void>;
}

// EditorView が必須のコマンド向けヘルパー。run 内での null チェックを
// 1 箇所に閉じ込め、各コマンド定義を `editorRun(toggleBold)` のように
// 簡潔に書けるようにする。
function editorRun(
  fn: (view: EditorView) => void,
): (ctx: CommandContext) => void {
  return (ctx) => {
    if (!ctx.view) return;
    fn(ctx.view);
  };
}

export const COMMANDS: Record<CommandId, CommandDef> = {
  "new-tab": {
    id: "new-tab",
    label: "新規タブ",
    icon: FilePlus,
    defaultKey: "Mod-t",
    needsEditor: false,
    run: (ctx) => ctx.newTab(),
  },
  "new-from-template": {
    id: "new-from-template",
    label: "テンプレートから新規",
    icon: FilePlus2,
    defaultKey: "Mod-Shift-t",
    needsEditor: false,
    run: (ctx) => ctx.newFromTemplate(),
  },
  "open-file": {
    id: "open-file",
    label: "開く",
    icon: FolderOpen,
    defaultKey: "Mod-o",
    needsEditor: false,
    run: (ctx) => ctx.openFile(),
  },
  "open-folder": {
    id: "open-folder",
    label: "フォルダを開く",
    icon: FolderTree,
    defaultKey: "Mod-Shift-o",
    needsEditor: false,
    run: (ctx) => ctx.openFolder(),
  },
  save: {
    id: "save",
    label: "保存",
    icon: Save,
    defaultKey: "Mod-s",
    needsEditor: false,
    run: (ctx) => ctx.save(),
  },
  "save-as": {
    id: "save-as",
    label: "名前を付けて保存",
    icon: SaveAll,
    defaultKey: "Mod-Shift-s",
    needsEditor: false,
    run: (ctx) => ctx.saveAs(),
  },
  "close-tab": {
    id: "close-tab",
    label: "タブを閉じる",
    icon: X,
    defaultKey: "Mod-w",
    needsEditor: false,
    run: (ctx) => ctx.closeActiveTab(),
  },
  "export-pdf": {
    id: "export-pdf",
    label: "PDF 出力",
    icon: FileDown,
    defaultKey: "Mod-e",
    needsEditor: false,
    run: (ctx) => ctx.exportPdf(),
  },
  bold: {
    id: "bold",
    label: "太字",
    icon: Bold,
    defaultKey: "Mod-b",
    needsEditor: true,
    run: editorRun(toggleBold),
  },
  italic: {
    id: "italic",
    label: "斜体",
    icon: Italic,
    defaultKey: "Mod-i",
    needsEditor: true,
    run: editorRun(toggleItalic),
  },
  "heading-1": {
    id: "heading-1",
    label: "見出し 1",
    icon: Heading1,
    needsEditor: true,
    run: editorRun((v) => applyHeading(v, 1)),
  },
  "heading-2": {
    id: "heading-2",
    label: "見出し 2",
    icon: Heading2,
    needsEditor: true,
    run: editorRun((v) => applyHeading(v, 2)),
  },
  "heading-3": {
    id: "heading-3",
    label: "見出し 3",
    icon: Heading3,
    needsEditor: true,
    run: editorRun((v) => applyHeading(v, 3)),
  },
  "bullet-list": {
    id: "bullet-list",
    label: "箇条書きリスト",
    icon: List,
    needsEditor: true,
    run: editorRun(toggleBulletList),
  },
  "numbered-list": {
    id: "numbered-list",
    label: "番号付きリスト",
    icon: ListOrdered,
    needsEditor: true,
    run: editorRun(toggleNumberedList),
  },
  math: {
    id: "math",
    label: "数式",
    icon: Sigma,
    needsEditor: true,
    run: editorRun(toggleMath),
  },
  "code-inline": {
    id: "code-inline",
    label: "コード(インライン)",
    icon: Code,
    needsEditor: true,
    run: editorRun(toggleInlineCode),
  },
  "code-block": {
    id: "code-block",
    label: "コード(ブロック)",
    icon: CodeXml,
    needsEditor: true,
    run: editorRun(insertCodeBlock),
  },
  link: {
    id: "link",
    label: "リンク",
    icon: LinkIcon,
    needsEditor: true,
    run: editorRun(insertLink),
  },
  footnote: {
    id: "footnote",
    label: "脚注",
    icon: Asterisk,
    needsEditor: true,
    run: editorRun(insertFootnote),
  },
  quote: {
    id: "quote",
    label: "引用",
    icon: Quote,
    needsEditor: true,
    run: editorRun(insertQuote),
  },
  image: {
    id: "image",
    label: "画像",
    icon: ImageIcon,
    needsEditor: true,
    run: (ctx) => ctx.pickAndInsertImage(),
  },
  table: {
    id: "table",
    label: "表",
    icon: TableIcon,
    needsEditor: true,
    run: editorRun((v) => insertTable(v)),
  },
  bibliography: {
    id: "bibliography",
    label: "参考文献ファイルを挿入",
    icon: BookOpen,
    needsEditor: true,
    run: (ctx) => ctx.pickAndInsertBibliography(),
  },
  "toggle-project-view": {
    id: "toggle-project-view",
    label: "プロジェクトビュー表示切替",
    icon: PanelLeft,
    defaultKey: "Mod-Shift-e",
    needsEditor: false,
    run: (ctx) => ctx.toggleProjectView(),
  },
  "toggle-preview": {
    id: "toggle-preview",
    label: "プレビュー表示切替",
    icon: PanelRight,
    defaultKey: "Mod-Shift-p",
    needsEditor: false,
    run: (ctx) => ctx.togglePreview(),
  },
};

// ツールバー上の項目。コマンド ID か区切り。
export type ToolbarItem = CommandId | "divider";

// プリセット定義。設定 UI ができるまでは loadSettings 時のフォールバックや
// 「リセット」操作の参照値として使う。
export interface ToolbarPreset {
  id: string;
  label: string;
  items: ToolbarItem[];
}

export const TOOLBAR_PRESETS: ToolbarPreset[] = [
  {
    id: "standard",
    label: "標準",
    items: [
      "new-tab",
      "new-from-template",
      "open-file",
      "open-folder",
      "save",
      "save-as",
      "export-pdf",
      "divider",
      "bold",
      "italic",
      "heading-1",
      "heading-2",
      "heading-3",
      "bullet-list",
      "numbered-list",
      "divider",
      "math",
      "code-inline",
      "code-block",
      "link",
      "footnote",
      "quote",
      "image",
      "table",
      "bibliography",
      "divider",
      "toggle-project-view",
      "toggle-preview",
    ],
  },
  {
    id: "minimal",
    label: "ミニマル",
    items: [
      "new-tab",
      "open-file",
      "open-folder",
      "save",
      "export-pdf",
      "divider",
      "bold",
      "italic",
      "heading-1",
      "bullet-list",
      "numbered-list",
      "divider",
      "toggle-project-view",
      "toggle-preview",
    ],
  },
  {
    id: "academic",
    label: "論文寄り",
    items: [
      "new-tab",
      "new-from-template",
      "open-file",
      "open-folder",
      "save",
      "save-as",
      "export-pdf",
      "divider",
      "bold",
      "italic",
      "heading-1",
      "heading-2",
      "heading-3",
      "divider",
      "math",
      "code-inline",
      "footnote",
      "quote",
      "link",
      "image",
      "table",
      "bibliography",
      "divider",
      "toggle-project-view",
      "toggle-preview",
    ],
  },
];

export const DEFAULT_TOOLBAR_PRESET_ID = "standard";

export function getDefaultToolbarItems(): ToolbarItem[] {
  const preset =
    TOOLBAR_PRESETS.find((p) => p.id === DEFAULT_TOOLBAR_PRESET_ID) ??
    TOOLBAR_PRESETS[0];
  return [...preset.items];
}
