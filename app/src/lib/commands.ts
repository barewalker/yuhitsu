// Yuhitsu のコマンドカタログ。
// ID をキーにツールバー描画 / キーバインド / 設定永続化 / (将来) MCP ハンドラ
// が共通で参照する。新規コマンドは COMMAND_IDS / COMMANDS の両方に追加する。

import type { EditorView } from "@codemirror/view";
import type { LucideIcon } from "@lucide/svelte";
import Asterisk from "@lucide/svelte/icons/asterisk";
import Bold from "@lucide/svelte/icons/bold";
import BookOpen from "@lucide/svelte/icons/book-open";
import ChevronLeft from "@lucide/svelte/icons/chevron-left";
import ChevronRight from "@lucide/svelte/icons/chevron-right";
import Keyboard from "@lucide/svelte/icons/keyboard";
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
import Settings from "@lucide/svelte/icons/settings";
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
  "next-tab",
  "prev-tab",
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
  "open-settings",
  "open-keybindings",
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
  // save / saveAs は呼び出し元(+page.svelte)では「保存できたか」を
  // boolean で返すが、コマンド経由の呼び出しは戻り値を使わないため
  // Promise<unknown> で受ける。
  save: () => void | Promise<unknown>;
  saveAs: () => void | Promise<unknown>;
  exportPdf: () => void | Promise<void>;
  pickAndInsertImage: () => void | Promise<void>;
  pickAndInsertBibliography: () => void | Promise<void>;
  togglePreview: () => void | Promise<void>;
  toggleProjectView: () => void | Promise<void>;
  newTab: () => void | Promise<void>;
  newFromTemplate: () => void | Promise<void>;
  closeActiveTab: () => void | Promise<void>;
  nextTab: () => void | Promise<void>;
  prevTab: () => void | Promise<void>;
  openSettings: () => void | Promise<void>;
  openKeybindings: () => void | Promise<void>;
}

export interface CommandDef {
  id: CommandId;
  /** i18n 辞書キー。表示時は t(def.labelKey) を呼んで現在 locale で解決する */
  labelKey: string;
  /** ツールバーに表示するアイコン(Lucide コンポーネント) */
  icon: LucideIcon;
  /** ボタンに当てる装飾クラス。装飾系は `fmt` 等 */
  buttonClass?: string;
  /** デフォルトキーバインド。"Mod-b" 形式(Mod は Ctrl/Cmd)。
   * 同じコマンドに複数キーを bind したい時は配列で渡す。
   * 表示(ホバーヒント等)では先頭のキーを優先。 */
  defaultKey?: string | string[];
  /** EditorView を必要とするか。null のとき disabled / キーは無視 */
  needsEditor: boolean;
  // 戻り値は使わないので unknown を許容(save / saveAs が boolean を返す等)
  run: (ctx: CommandContext) => void | Promise<unknown>;
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
    labelKey: "command.newTab",
    icon: FilePlus,
    defaultKey: "Mod-t",
    needsEditor: false,
    run: (ctx) => ctx.newTab(),
  },
  "new-from-template": {
    id: "new-from-template",
    labelKey: "command.newFromTemplate",
    icon: FilePlus2,
    defaultKey: "Mod-Shift-t",
    needsEditor: false,
    run: (ctx) => ctx.newFromTemplate(),
  },
  "open-file": {
    id: "open-file",
    labelKey: "command.open",
    icon: FolderOpen,
    defaultKey: "Mod-o",
    needsEditor: false,
    run: (ctx) => ctx.openFile(),
  },
  "open-folder": {
    id: "open-folder",
    labelKey: "command.openFolder",
    icon: FolderTree,
    defaultKey: "Mod-Shift-o",
    needsEditor: false,
    run: (ctx) => ctx.openFolder(),
  },
  save: {
    id: "save",
    labelKey: "command.save",
    icon: Save,
    defaultKey: "Mod-s",
    needsEditor: false,
    run: (ctx) => ctx.save(),
  },
  "save-as": {
    id: "save-as",
    labelKey: "command.saveAs",
    icon: SaveAll,
    defaultKey: "Mod-Shift-s",
    needsEditor: false,
    run: (ctx) => ctx.saveAs(),
  },
  "close-tab": {
    id: "close-tab",
    labelKey: "command.closeTab",
    icon: X,
    defaultKey: "Mod-w",
    needsEditor: false,
    run: (ctx) => ctx.closeActiveTab(),
  },
  "next-tab": {
    id: "next-tab",
    labelKey: "command.nextTab",
    icon: ChevronRight,
    // Ctrl+Tab はブラウザ流儀(Yuhitsu の Tauri WebView でも動作する)、
    // Ctrl+PageDown は VSCode 流儀のフォールバック。
    defaultKey: ["Mod-Tab", "Mod-PageDown"],
    needsEditor: false,
    run: (ctx) => ctx.nextTab(),
  },
  "prev-tab": {
    id: "prev-tab",
    labelKey: "command.prevTab",
    icon: ChevronLeft,
    // Ctrl+Shift+Tab は WebKitGTK の focus traversal 予約で JS まで届かない
    // ため、Ctrl+PageUp が実用上のメイン。Ctrl+Shift+Tab も将来 Webview の
    // 仕様変更で動くようになった時のために残しておく。
    defaultKey: ["Mod-Shift-Tab", "Mod-PageUp"],
    needsEditor: false,
    run: (ctx) => ctx.prevTab(),
  },
  "export-pdf": {
    id: "export-pdf",
    labelKey: "command.exportPdf",
    icon: FileDown,
    defaultKey: "Mod-e",
    needsEditor: false,
    run: (ctx) => ctx.exportPdf(),
  },
  bold: {
    id: "bold",
    labelKey: "command.bold",
    icon: Bold,
    defaultKey: "Mod-b",
    needsEditor: true,
    run: editorRun(toggleBold),
  },
  italic: {
    id: "italic",
    labelKey: "command.italic",
    icon: Italic,
    defaultKey: "Mod-i",
    needsEditor: true,
    run: editorRun(toggleItalic),
  },
  "heading-1": {
    id: "heading-1",
    labelKey: "command.heading1",
    icon: Heading1,
    needsEditor: true,
    run: editorRun((v) => applyHeading(v, 1)),
  },
  "heading-2": {
    id: "heading-2",
    labelKey: "command.heading2",
    icon: Heading2,
    needsEditor: true,
    run: editorRun((v) => applyHeading(v, 2)),
  },
  "heading-3": {
    id: "heading-3",
    labelKey: "command.heading3",
    icon: Heading3,
    needsEditor: true,
    run: editorRun((v) => applyHeading(v, 3)),
  },
  "bullet-list": {
    id: "bullet-list",
    labelKey: "command.bulletList",
    icon: List,
    needsEditor: true,
    run: editorRun(toggleBulletList),
  },
  "numbered-list": {
    id: "numbered-list",
    labelKey: "command.numberedList",
    icon: ListOrdered,
    needsEditor: true,
    run: editorRun(toggleNumberedList),
  },
  math: {
    id: "math",
    labelKey: "command.math",
    icon: Sigma,
    needsEditor: true,
    run: editorRun(toggleMath),
  },
  "code-inline": {
    id: "code-inline",
    labelKey: "command.codeInline",
    icon: Code,
    needsEditor: true,
    run: editorRun(toggleInlineCode),
  },
  "code-block": {
    id: "code-block",
    labelKey: "command.codeBlock",
    icon: CodeXml,
    needsEditor: true,
    run: editorRun(insertCodeBlock),
  },
  link: {
    id: "link",
    labelKey: "command.link",
    icon: LinkIcon,
    needsEditor: true,
    run: editorRun(insertLink),
  },
  footnote: {
    id: "footnote",
    labelKey: "command.footnote",
    icon: Asterisk,
    needsEditor: true,
    run: editorRun(insertFootnote),
  },
  quote: {
    id: "quote",
    labelKey: "command.quote",
    icon: Quote,
    needsEditor: true,
    run: editorRun(insertQuote),
  },
  image: {
    id: "image",
    labelKey: "command.image",
    icon: ImageIcon,
    needsEditor: true,
    run: (ctx) => ctx.pickAndInsertImage(),
  },
  table: {
    id: "table",
    labelKey: "command.table",
    icon: TableIcon,
    needsEditor: true,
    run: editorRun((v) => insertTable(v)),
  },
  bibliography: {
    id: "bibliography",
    labelKey: "command.bibliography",
    icon: BookOpen,
    needsEditor: true,
    run: (ctx) => ctx.pickAndInsertBibliography(),
  },
  "toggle-project-view": {
    id: "toggle-project-view",
    labelKey: "command.toggleProjectView",
    icon: PanelLeft,
    // Ctrl+Shift+E は WebKitGTK が呑み込むため、IntelliJ 流儀の Ctrl+1 を
    // 代替に併設する(将来 Webview の制約が解ければ自動で Shift+E も効く)。
    defaultKey: ["Mod-Shift-e", "Mod-1"],
    needsEditor: false,
    run: (ctx) => ctx.toggleProjectView(),
  },
  "toggle-preview": {
    id: "toggle-preview",
    labelKey: "command.togglePreview",
    icon: PanelRight,
    defaultKey: "Mod-Shift-p",
    needsEditor: false,
    run: (ctx) => ctx.togglePreview(),
  },
  "open-settings": {
    id: "open-settings",
    labelKey: "command.openSettings",
    icon: Settings,
    defaultKey: "Mod-,",
    needsEditor: false,
    run: (ctx) => ctx.openSettings(),
  },
  "open-keybindings": {
    id: "open-keybindings",
    labelKey: "command.openKeybindings",
    icon: Keyboard,
    // ショートカットは default 無し(頻繁に開かない、ユーザが必要なら
    // 設定 UI で自分で割り当てる)
    needsEditor: false,
    run: (ctx) => ctx.openKeybindings(),
  },
};

// ツールバー上の項目。コマンド ID か区切り。
export type ToolbarItem = CommandId | "divider";

// プリセット定義。設定 UI ができるまでは loadSettings 時のフォールバックや
// 「リセット」操作の参照値として使う。
export interface ToolbarPreset {
  id: string;
  /** i18n 辞書キー(例: "preset.standard")。表示時に t() を呼ぶ */
  labelKey: string;
  items: ToolbarItem[];
}

export const TOOLBAR_PRESETS: ToolbarPreset[] = [
  {
    id: "standard",
    labelKey: "preset.standard",
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
      "open-keybindings",
      "open-settings",
    ],
  },
  {
    id: "minimal",
    labelKey: "preset.minimal",
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
      "open-keybindings",
      "open-settings",
    ],
  },
  {
    id: "academic",
    labelKey: "preset.academic",
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
      "open-keybindings",
      "open-settings",
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
