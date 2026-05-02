<!--
  キーバインド設定ダイアログ。

  各コマンドに対する override(設定で書き換えたキーバインド)を編集する。
  override が無い時はコマンドの defaultKey が使われる。
  キャプチャ:行のキー欄をクリック → キー入力モード → 押されたキーで
  Mod-X 形式に decode して onUpdate に通知。Escape でキャンセル。
  「クリア」で override を消す(default に戻る)。
-->
<script lang="ts">
  import { COMMANDS, COMMAND_IDS, type CommandId } from "$lib/commands";
  import { t } from "$lib/i18n/index.svelte";

  type Props = {
    keybindings: Record<string, string>;
    onUpdate: (next: Record<string, string>) => void;
    onClose: () => void;
  };
  let { keybindings, onUpdate, onClose }: Props = $props();

  // 現在キャプチャ中のコマンド ID。null なら通常表示。
  let capturing = $state<CommandId | null>(null);
  // 衝突警告メッセージ。設定後 1 回だけ表示し、次の操作でクリア。
  let conflictMessage = $state<string | null>(null);

  // ホバー / 表示用の effective key(override 優先、なければ defaultKey 先頭)。
  function effectiveKeyDisplay(id: CommandId): string {
    const override = keybindings[id];
    if (typeof override === "string" && override.length > 0)
      return formatKey(override);
    const def = COMMANDS[id].defaultKey;
    const raw = Array.isArray(def) ? (def[0] ?? "") : (def ?? "");
    return formatKey(raw);
  }

  // "Mod-Shift-b" → "Ctrl+Shift+B" に整形。Mod は Linux/Win で Ctrl、
  // macOS で Cmd だが、ここでは Linux 表記で一律 "Ctrl"(macOS 対応は
  // navigator.platform を見て切替を将来追加)。
  function formatKey(spec: string): string {
    if (!spec) return "";
    return spec.replaceAll("Mod", "Ctrl").replaceAll("-", "+");
  }

  // 衝突チェック:同じキーを使っている他のコマンドを探して名前を返す。
  function findConflict(id: CommandId, key: string): CommandId | null {
    for (const other of COMMAND_IDS) {
      if (other === id) continue;
      // override が一致 → 衝突
      const otherOverride = keybindings[other];
      if (typeof otherOverride === "string" && otherOverride === key) {
        return other;
      }
      // override が無ければ defaultKey と比較
      if (otherOverride === undefined) {
        const def = COMMANDS[other].defaultKey;
        if (Array.isArray(def)) {
          if (def.includes(key)) return other;
        } else if (def === key) {
          return other;
        }
      }
    }
    return null;
  }

  // e.code(物理キー名)を Yuhitsu 形式の短縮キーに正規化。
  //   KeyB → "b"、Digit1 → "1"、それ以外は素のまま("ArrowLeft" 等)
  // GTK key theme = Emacs の環境で e.key が化ける問題を回避するため、
  // 保存も判定も物理キー側で揃える。
  function normalizeCode(code: string): string {
    if (code.startsWith("Key")) return code.slice(3).toLowerCase();
    if (code.startsWith("Digit")) return code.slice(5);
    return code;
  }

  function onCaptureKeydown(e: KeyboardEvent, id: CommandId) {
    // テキスト入力は許さない。modifier 単独は無視して次のキーを待つ。
    // 注意: OS 側でグローバルなキーバインドテーマ(GNOME の Emacs テーマ
    // 等)が有効だと、Ctrl+B などが OS で先に解釈されて ArrowLeft 等として
    // 届く場合がある。Yuhitsu はユーザの OS 設定を尊重するため上書きせず、
    // そのまま記録する(ユーザが OS テーマを切替えるか、別のキーを選ぶ)。
    e.preventDefault();
    e.stopPropagation();
    // Escape は離脱。code でも key でも一律 "Escape"
    if (e.code === "Escape") {
      capturing = null;
      conflictMessage = null;
      return;
    }
    // modifier 単独入力(まだ次の本キーを待っている状態)は無視
    if (
      e.code === "ControlLeft" ||
      e.code === "ControlRight" ||
      e.code === "ShiftLeft" ||
      e.code === "ShiftRight" ||
      e.code === "AltLeft" ||
      e.code === "AltRight" ||
      e.code === "MetaLeft" ||
      e.code === "MetaRight"
    ) {
      return;
    }
    const parts: string[] = [];
    if (e.ctrlKey || e.metaKey) parts.push("Mod");
    if (e.shiftKey) parts.push("Shift");
    if (e.altKey) parts.push("Alt");
    parts.push(normalizeCode(e.code));
    const key = parts.join("-");
    const conflict = findConflict(id, key);
    if (conflict) {
      conflictMessage = t("keybindings.conflict", {
        label: t(COMMANDS[conflict].labelKey),
      });
    } else {
      conflictMessage = null;
    }
    onUpdate({ ...keybindings, [id]: key });
    capturing = null;
  }

  function startCapture(id: CommandId) {
    capturing = id;
    conflictMessage = null;
  }

  function clearOverride(id: CommandId) {
    const next = { ...keybindings };
    delete next[id];
    onUpdate(next);
    conflictMessage = null;
  }

  // capture 用の <div> をマウント直後に focus するための use:action。
  // Svelte の autofocus 属性は <input> でしか効かないため自前で focus する。
  function autofocus(node: HTMLElement) {
    node.focus();
  }
</script>

<div class="overlay" role="presentation" onclick={onClose}></div>
<div
  class="dialog"
  role="dialog"
  aria-modal="true"
  aria-label={t("keybindings.ariaLabel")}
>
  <div class="header">
    <span class="title">{t("keybindings.title")}</span>
    <button class="close-btn" onclick={onClose} aria-label={t("keybindings.close")}
      >×</button
    >
  </div>
  <div class="body">
    <table>
      <thead>
        <tr>
          <th class="col-cmd">{t("keybindings.command")}</th>
          <th class="col-key">{t("keybindings.binding")}</th>
          <th class="col-act"></th>
        </tr>
      </thead>
      <tbody>
        {#each COMMAND_IDS as id (id)}
          {@const def = COMMANDS[id]}
          {@const Icon = def.icon}
          {@const display = effectiveKeyDisplay(id)}
          {@const isOverride = typeof keybindings[id] === "string"}
          <tr>
            <td class="cmd">
              <Icon size={14} class="ic" />
              <span class="label">{t(def.labelKey)}</span>
            </td>
            <td>
              {#if capturing === id}
                <!-- <input> を使うと GTK の emacs キーバインド(Ctrl-B が
                     左移動など)が先に走って key が ArrowLeft 等に化ける。
                     div + tabindex で keydown だけ取る方式に切替。 -->
                <div
                  class="capture-input"
                  role="textbox"
                  tabindex="0"
                  use:autofocus
                  onkeydown={(e) => onCaptureKeydown(e, id)}
                  onblur={() => (capturing = null)}
                >
                  {t("keybindings.captureHint")}
                </div>
              {:else}
                <button
                  class="binding"
                  class:override={isOverride}
                  onclick={() => startCapture(id)}
                >
                  {display || t("keybindings.unset")}
                </button>
              {/if}
            </td>
            <td>
              {#if isOverride}
                <button class="action" onclick={() => clearOverride(id)}
                  >{t("keybindings.reset")}</button
                >
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#if conflictMessage}
      <div class="conflict">{conflictMessage}</div>
    {/if}
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
    width: 640px;
    max-width: 90vw;
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
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }
  th {
    text-align: left;
    color: var(--text-tertiary);
    font-weight: 500;
    padding: 6px 4px;
    border-bottom: 1px solid var(--border);
  }
  td {
    padding: 4px;
    border-bottom: 1px solid var(--bg-elevated-1);
    vertical-align: middle;
  }
  .col-cmd {
    width: 60%;
  }
  .col-key {
    width: 30%;
  }
  .col-act {
    width: 10%;
  }
  .cmd {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .cmd .label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .binding {
    background: var(--bg-base);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px 8px;
    color: var(--text-secondary);
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 11px;
    cursor: pointer;
    min-width: 140px;
    text-align: left;
  }
  .binding:hover {
    background: var(--bg-elevated-3);
    color: var(--text-primary);
  }
  .binding.override {
    color: var(--text-primary);
    border-color: var(--accent);
  }
  .capture-input {
    background: var(--accent-bg-subtle);
    border: 1px solid var(--accent);
    border-radius: 4px;
    padding: 4px 8px;
    color: var(--text-primary);
    font-size: 11px;
    width: 100%;
    box-sizing: border-box;
    outline: none;
  }
  .action {
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 2px 8px;
    color: var(--text-tertiary);
    font-size: 11px;
    cursor: pointer;
  }
  .action:hover {
    color: var(--text-primary);
    background: var(--bg-elevated-3);
  }
  .conflict {
    margin-top: 8px;
    padding: 6px 8px;
    background: var(--bg-elevated-1);
    border: 1px solid var(--status-error-strong);
    border-radius: 4px;
    color: var(--status-error-strong);
    font-size: 11px;
  }
</style>
