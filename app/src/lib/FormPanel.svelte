<!--
  フォーム型テンプレート入力パネル(差別化ポイント #2)。

  - active タブの doc を受け取り、最初の `#show: <fn>.with(...)` を解析
  - 同梱テンプレの form spec(meta.json の `form.fields`)があれば、ラベル翻訳・
    型・並び順をそれに従う。無ければ call site の引数キーをそのまま label にして
    汎用テキスト入力欄を出す(P3 — ユーザ自作ドキュメントへの応用)
  - 各 input は **focus 中は draft、blur で onApply** が原則(打鍵ごとに doc を
    書き換えると undo 履歴が肥大するため)。boolean は即時反映
-->
<script lang="ts">
  import type { Locale } from "$lib/i18n/locale";
  import { t } from "$lib/i18n/index.svelte";
  import {
    encodeArgValue,
    findWithCall,
    type ArgKind,
    type WithCall,
  } from "$lib/template-args";
  import type { FormField, FormSpec } from "$lib/templates";

  type Props = {
    /** 現在編集中のドキュメントテキスト。null なら active タブなし */
    doc: string | null;
    /** active タブが Typst 言語モードか */
    isTypst: boolean;
    /** 同梱テンプレ由来のフォーム仕様(あれば fn 名 + fields ラベル翻訳) */
    spec: FormSpec | null;
    /** 表示言語(label / placeholder の解決用) */
    locale: Locale;
    /** フォーム値が変わった時のコールバック。doc を再パースして書き戻すのは親側 */
    onApply: (name: string, value: ArgKind) => void;
  };

  let { doc, isTypst, spec, locale, onApply }: Props = $props();

  // 現在の `#show: ~.with(...)` 呼び出し情報。doc 変更で自動再計算。
  const call = $derived<WithCall | null>(doc ? findWithCall(doc) : null);

  // 表示するフィールド一覧。spec があれば spec.fields の順、無ければ call.args
  // のキーから生成(P3 汎用フォールバック)。spec.fn と call.fn が一致しない
  // 場合は spec を捨ててフォールバック扱い(本来は同じはずだが、ユーザが
  // テンプレ起源のドキュメントの関数名を変えるケースを許容)。
  type DisplayField = {
    name: string;
    type: FormField["type"];
    label: string;
    placeholder: string;
    /** 現在 doc 上で持っている値(無ければ未指定としての null) */
    current: ArgKind | null;
  };

  const displayFields = $derived<DisplayField[]>(buildDisplayFields());

  function buildDisplayFields(): DisplayField[] {
    if (!call) return [];
    const argMap = new Map(call.args.map((a) => [a.name, a.value]));
    const useSpec = spec && spec.function === call.fn ? spec : null;
    if (useSpec) {
      return useSpec.fields.map((f) => ({
        name: f.name,
        type: f.type,
        label: localized(f.label) || f.name,
        placeholder: localized(f.placeholder ?? {}),
        current: argMap.get(f.name) ?? null,
      }));
    }
    // 汎用フォールバック: call site にあるキーをそのまま並べる
    return call.args.map((a) => ({
      name: a.name,
      type: inferType(a.value),
      label: a.name,
      placeholder: "",
      current: a.value,
    }));
  }

  function localized(value: Record<string, string>): string {
    if (!value) return "";
    return value[locale] ?? value.en ?? Object.values(value)[0] ?? "";
  }

  function inferType(v: ArgKind): FormField["type"] {
    switch (v.kind) {
      case "boolean":
        return "boolean";
      case "number":
        return "number";
      default:
        return "string";
    }
  }

  function asString(v: ArgKind | null): string {
    if (!v) return "";
    if (v.kind === "string") return v.value;
    if (v.kind === "number") return String(v.value);
    if (v.kind === "boolean") return v.value ? "true" : "false";
    return v.raw;
  }

  function asNumber(v: ArgKind | null): string {
    if (v && v.kind === "number") return String(v.value);
    if (v && v.kind === "raw") return v.raw;
    return "";
  }

  function asBoolean(v: ArgKind | null): boolean {
    return !!(v && v.kind === "boolean" && v.value);
  }

  function isRawValue(v: ArgKind | null): boolean {
    return v?.kind === "raw";
  }

  function applyString(name: string, raw: string) {
    onApply(name, { kind: "string", value: raw });
  }

  function applyNumber(name: string, raw: string) {
    const trimmed = raw.trim();
    if (trimmed.length === 0) {
      // 空文字は string ではなく "" の string として書き戻すのが穏当
      onApply(name, { kind: "string", value: "" });
      return;
    }
    const n = Number(trimmed);
    if (Number.isFinite(n)) onApply(name, { kind: "number", value: n });
    else onApply(name, { kind: "string", value: trimmed });
  }

  function applyBoolean(name: string, value: boolean) {
    onApply(name, { kind: "boolean", value });
  }
</script>

<div class="form-panel" role="region" aria-label={t("form.title")}>
  {#if doc === null}
    <div class="placeholder">{t("form.noTab")}</div>
  {:else if !isTypst}
    <div class="placeholder">{t("form.notTypst")}</div>
  {:else if !call}
    <div class="placeholder">{t("form.noWithCall")}</div>
  {:else}
    <div class="form-header">
      <span class="fn-label">{t("form.fnLabel")}</span>
      <code class="fn-name">{call.fn}</code>
    </div>
    {#if displayFields.length === 0}
      <div class="placeholder">{t("form.noFields")}</div>
    {:else}
      <div class="fields">
        {#each displayFields as f (f.name)}
          <label class="field" class:raw={isRawValue(f.current)}>
            <span class="field-label">{f.label}</span>
            {#if isRawValue(f.current)}
              <input
                type="text"
                class="field-input"
                value={asString(f.current)}
                disabled
                title={t("form.rawNotEditable")}
              />
            {:else if f.type === "boolean"}
              <input
                type="checkbox"
                class="field-checkbox"
                checked={asBoolean(f.current)}
                onchange={(e) =>
                  applyBoolean(f.name, (e.currentTarget as HTMLInputElement).checked)}
              />
            {:else if f.type === "number"}
              <input
                type="number"
                class="field-input"
                value={asNumber(f.current)}
                placeholder={f.placeholder}
                onblur={(e) =>
                  applyNumber(f.name, (e.currentTarget as HTMLInputElement).value)}
              />
            {:else if f.type === "text"}
              <textarea
                class="field-textarea"
                value={asString(f.current)}
                placeholder={f.placeholder}
                rows="3"
                onblur={(e) =>
                  applyString(
                    f.name,
                    (e.currentTarget as HTMLTextAreaElement).value,
                  )}
              ></textarea>
            {:else}
              <input
                type="text"
                class="field-input"
                value={asString(f.current)}
                placeholder={f.placeholder}
                onblur={(e) =>
                  applyString(f.name, (e.currentTarget as HTMLInputElement).value)}
              />
            {/if}
          </label>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .form-panel {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 8px 10px;
    box-sizing: border-box;
    background: var(--color-bg);
    color: var(--color-fg);
    font-size: 13px;
  }

  .placeholder {
    color: var(--color-fg-muted);
    font-size: 12px;
    line-height: 1.5;
    padding: 8px 4px;
  }

  .form-header {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 4px 4px 8px;
    border-bottom: 1px solid var(--color-border);
    margin-bottom: 8px;
  }

  .fn-label {
    color: var(--color-fg-muted);
    font-size: 11px;
  }

  .fn-name {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 12px;
    color: var(--color-fg);
  }

  .fields {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .field-label {
    font-size: 11px;
    color: var(--color-fg-muted);
    line-height: 1.2;
  }

  .field-input,
  .field-textarea {
    width: 100%;
    box-sizing: border-box;
    padding: 4px 6px;
    background: var(--color-bg-input, var(--color-bg-elevated));
    color: var(--color-fg);
    border: 1px solid var(--color-border);
    border-radius: 3px;
    font: inherit;
    font-size: 13px;
  }

  .field-input:focus,
  .field-textarea:focus {
    outline: none;
    border-color: var(--color-accent);
  }

  .field-textarea {
    resize: vertical;
    min-height: 60px;
  }

  .field-checkbox {
    margin: 0;
    align-self: flex-start;
  }

  .field.raw .field-input {
    color: var(--color-fg-muted);
    font-style: italic;
    cursor: not-allowed;
  }
</style>
