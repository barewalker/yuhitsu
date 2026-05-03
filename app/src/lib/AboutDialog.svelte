<!--
  About ダイアログ。アプリ情報・ライセンス・同梱フォント・エンジン・
  外部リンクを表示する。リンクは tauri-plugin-opener 経由で OS デフォルト
  ブラウザに渡す。
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { getVersion } from "@tauri-apps/api/app";
  import { t } from "$lib/i18n/index.svelte";

  type Props = {
    onClose: () => void;
  };

  let { onClose }: Props = $props();

  let version = $state("");

  // 外部リンクは OS デフォルトブラウザで開く(Tauri webview 内では開かない)
  const LINKS = {
    repo: "https://github.com/barewalker/yuhitsu",
    author: "https://github.com/barewalker",
    typstDocs: "https://typst.app/docs/",
    haranoAji: "https://github.com/trueroad/HaranoAjiFonts",
    tinymist: "https://github.com/Myriad-Dreamin/tinymist",
  };

  function open(url: string) {
    openUrl(url).catch((e) => {
      console.warn("[about] openUrl failed:", e);
    });
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  }

  onMount(() => {
    getVersion()
      .then((v) => {
        version = v;
      })
      .catch(() => {
        version = "";
      });
    document.addEventListener("keydown", onKeydown);
    return () => document.removeEventListener("keydown", onKeydown);
  });
</script>

<div class="overlay" onclick={onClose} role="presentation">
  <div
    class="dialog"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-label={t("about.ariaLabel")}
    tabindex="-1"
  >
    <h2 class="title">{t("about.title")}</h2>
    <p class="tagline">{t("about.tagline")}</p>

    <div class="meta">
      {#if version}
        <div>{t("about.version", { version })}</div>
      {/if}
      <div>{t("about.license")}</div>
    </div>

    <section class="section">
      <h3 class="section-title">{t("about.authorHeading")}</h3>
      <p class="section-text">{t("about.authorText")}</p>
    </section>

    <section class="section">
      <h3 class="section-title">{t("about.fontsHeading")}</h3>
      <p class="section-text">{t("about.fontsText")}</p>
    </section>

    <section class="section">
      <h3 class="section-title">{t("about.engineHeading")}</h3>
      <p class="section-text">{t("about.engineText")}</p>
    </section>

    <section class="section">
      <h3 class="section-title">{t("about.linksHeading")}</h3>
      <ul class="links">
        <li>
          <button type="button" class="link-btn" onclick={() => open(LINKS.repo)}
            >{t("about.linkRepo")}</button
          >
        </li>
        <li>
          <button type="button" class="link-btn" onclick={() => open(LINKS.author)}
            >{t("about.linkAuthor")}</button
          >
        </li>
        <li>
          <button
            type="button"
            class="link-btn"
            onclick={() => open(LINKS.typstDocs)}>{t("about.linkTypstDocs")}</button
          >
        </li>
        <li>
          <button
            type="button"
            class="link-btn"
            onclick={() => open(LINKS.haranoAji)}>{t("about.linkHaranoAji")}</button
          >
        </li>
        <li>
          <button type="button" class="link-btn" onclick={() => open(LINKS.tinymist)}
            >{t("about.linkTinymist")}</button
          >
        </li>
      </ul>
    </section>

    <div class="actions">
      <button type="button" class="primary-btn" onclick={onClose}
        >{t("about.close")}</button
      >
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 300;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }
  .dialog {
    width: min(440px, 100%);
    max-height: 90vh;
    overflow-y: auto;
    padding: 20px 24px;
    background: var(--bg-elevated-1);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.5);
    font-size: 13px;
    line-height: 1.55;
  }
  .title {
    margin: 0 0 4px;
    font-size: 18px;
    font-weight: 600;
    color: var(--text-strong);
  }
  .tagline {
    margin: 0 0 12px;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .meta {
    margin-bottom: 14px;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .meta > div + div {
    margin-top: 2px;
  }
  .section {
    margin: 14px 0;
  }
  .section-title {
    margin: 0 0 4px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .section-text {
    margin: 0;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .links {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .link-btn {
    background: none;
    border: none;
    padding: 0;
    color: var(--syntax-function);
    text-decoration: underline;
    cursor: pointer;
    font: inherit;
    font-size: 12px;
    text-align: left;
  }
  .link-btn:hover {
    text-decoration: none;
  }
  .actions {
    margin-top: 18px;
    display: flex;
    justify-content: flex-end;
  }
  .primary-btn {
    padding: 5px 16px;
    background: var(--accent);
    color: var(--accent-fg, white);
    border: 1px solid var(--accent);
    border-radius: 4px;
    cursor: pointer;
    font: inherit;
    font-size: 12px;
  }
  .primary-btn:hover {
    background: var(--accent-strong, var(--accent));
  }
</style>
