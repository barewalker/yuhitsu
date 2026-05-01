<!--
  起動時テンプレ選択ダイアログ。
  - カードグリッド(アイコン + タイトル + 短い説明、hover でフル説明)
  - ESC / 背景クリックで onCancel(空ドキュメントで進む)
  - カードクリックで onSelect(id) を呼ぶ
-->
<script lang="ts">
  import { onMount } from "svelte";
  import type { Component } from "svelte";
  import type { TemplateMeta } from "$lib/templates";
  import type { Locale } from "$lib/i18n/locale";
  import { t } from "$lib/i18n/index.svelte";

  // Lucide アイコンを meta.icon の文字列で動的に解決
  import File from "@lucide/svelte/icons/file";
  import FileText from "@lucide/svelte/icons/file-text";
  import FlaskConical from "@lucide/svelte/icons/flask-conical";
  import Users from "@lucide/svelte/icons/users";
  import Mail from "@lucide/svelte/icons/mail";
  import Presentation from "@lucide/svelte/icons/presentation";

  type Props = {
    templates: TemplateMeta[];
    locale: Locale;
    onSelect: (id: string) => void;
    onCancel: () => void;
  };

  let { templates, locale, onSelect, onCancel }: Props = $props();

  const ICONS: Record<string, Component> = {
    File,
    FileText,
    FlaskConical,
    Users,
    Mail,
    Presentation,
  };

  function iconFor(name: string): Component {
    return ICONS[name] ?? File;
  }

  function localized(value: Record<string, string>): string {
    return value[locale] ?? value.en ?? Object.values(value)[0] ?? "";
  }

  onMount(() => {
    const onKeydown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onCancel();
      }
    };
    window.addEventListener("keydown", onKeydown);
    return () => window.removeEventListener("keydown", onKeydown);
  });
</script>

<div
  class="backdrop"
  role="presentation"
  onclick={onCancel}
  onkeydown={() => {}}
>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label={t("templateDialog.ariaLabel")}
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
    onkeydown={() => {}}
  >
    <div class="grid">
      {#each templates as t (t.id)}
        {@const Icon = iconFor(t.icon)}
        <button
          class="card"
          title={localized(t.description)}
          onclick={() => onSelect(t.id)}
        >
          <span class="card-icon">
            <Icon size={32} />
          </span>
          <span class="card-title">{localized(t.title)}</span>
        </button>
      {/each}
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dialog {
    background: var(--bg-elevated-1);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 24px;
    max-width: min(720px, 90vw);
    max-height: 80vh;
    overflow: auto;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 12px;
  }

  .card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 18px 12px;
    background: var(--bg-elevated-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    cursor: pointer;
    transition:
      background 100ms ease,
      border-color 100ms ease,
      transform 100ms ease;
  }

  .card:hover {
    background: var(--bg-elevated-3);
    border-color: var(--accent);
    transform: translateY(-1px);
  }

  .card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .card-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
  }

  .card-title {
    font-size: 13px;
    text-align: center;
    line-height: 1.3;
  }
</style>
