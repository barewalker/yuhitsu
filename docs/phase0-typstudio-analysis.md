# Phase 0 — Typstudio 分析レポート

調査日: 2026-04-23
対象: [Cubxity/typstudio](https://github.com/Cubxity/typstudio)
調査用 clone: `~/Projects/yuhitsu-refs/typstudio/` (shallow, depth=50)

## 結論サマリ

| 観点 | 判定 |
|---|---|
| ライセンス互換性(Apache-2.0 Yuhitsu への組込) | ❌ **非互換**(GPLv3) |
| メンテナンス状況 | ❌ **実質停止**、本人が「近い将来アーカイブ予定」と明言 |
| Typst 追従性 | ⚠️ v0.11.0 固定(現行 v0.14 系から2世代遅れ) |
| 即時ビルド可否 | ❌ `cargo check` が transitive dep(time)で失敗 |
| アーキテクチャ参考価値 | ⭕ Tauri + IPC 構成、Monaco 統合、Typst World 実装は参考になる |

→ **fork は不可。参考実装として読むに留める。**

## 1. ライセンス

- ルート直下 `COPYING` に **GNU General Public License Version 3** 全文。
- `Cargo.toml` / `package.json` 双方で `"license": "GPL-3.0-or-later"` と明記。
- NOTICE 等の追加ライセンスファイルは無し。

### 互換性判定

Yuhitsu は Apache-2.0 を採用方針(`CLAUDE.md`)。GPLv3 のコピーレフト条項により、GPLv3 派生物は全体を GPLv3 で公開する義務を負う。Typstudio を fork すると Yuhitsu も GPLv3 に縛られ、Apache-2.0 方針と両立不可能。**コード片レベルの参照**(アルゴリズムの理解や API 呼び出しパターンの参考)は fair use の範囲で許容されるが、ファイル単位・関数単位でのコピーは不可。

## 2. 依存関係

### Rust (`src-tauri/Cargo.toml`)

| crate | version | 備考 |
|---|---|---|
| `typst` | **0.11.0** (git tag) | 現行 v0.14 系から 2 世代遅れ |
| `typst-ide` | 0.11.0 (git tag) | 補完・診断の参照実装 |
| `typst-pdf` | 0.11.0 (git tag) | |
| `typst-render` | 0.11.0 (git tag) | ラスタライザ |
| `comemo` | 0.4.0 | Typst のメモ化基盤 |
| `tauri` | **1.6** | Tauri 2 系未対応 |
| `tauri-build` | 1.5 | |
| `tokio` | 1 (multi-thread) | |
| `notify` | 6.1 | ファイル変更監視 |
| `png` | 0.17 | preview 配信用 PNG エンコード |
| `arboard` | 3.3 | クリップボード |

Rust edition 2021、MSRV 1.57。

### フロントエンド (`package.json`)

| パッケージ | version | 役割 |
|---|---|---|
| `monaco-editor` | **0.47.0** | エディタ本体 |
| `vscode-textmate` | 9.0.0 | シンタックスハイライト |
| `vscode-oniguruma` | 2.0.1 | regex エンジン(WASM) |
| `svelte` | 4.2.12 | UI フレームワーク |
| `@sveltejs/kit` | 2.5.3 | SvelteKit(SSG adapter) |
| `tailwindcss` | 3.4.1 | CSS ユーティリティ |
| `typescript` | 5.4.2 | |
| `vite` | 5.1.6 | ビルドツール |
| `@tauri-apps/api` | 1.5.3 | Tauri JS SDK |
| `lucide-svelte` | 0.356.0 | アイコン |
| `lodash` | 4.17.21 | |

パッケージマネージャ: **pnpm**(`pnpm-lock.yaml` で固定)。

## 3. アーキテクチャ

### ディレクトリ(抜粋)

```
typstudio/
├── src-tauri/                   # Rust backend (≒1000 LOC)
│   └── src/
│       ├── main.rs              # Tauri setup, state, IPC 登録
│       ├── engine/
│       │   ├── engine.rs        # TypstEngine: Library + FontBook キャッシュ
│       │   └── font.rs          # フォント探索
│       ├── ipc/commands/
│       │   ├── typst.rs         # compile / render / autocomplete
│       │   ├── fs.rs            # ファイルシステム
│       │   └── clipboard.rs
│       └── project/
│           ├── manager.rs
│           ├── project.rs
│           └── world.rs         # typst::World トレイト実装
├── src/                         # SvelteKit frontend (≒1400 LOC)
│   ├── components/              # Editor / Preview / PreviewPage / ExplorerTree 等
│   └── lib/
│       ├── editor/
│       │   ├── monaco.ts        # Monaco 初期化 + Oniguruma ロード
│       │   ├── completion.ts    # typst_autocomplete invoke
│       │   ├── grammar.ts
│       │   └── lang/typst-tm.json
│       └── ipc/                 # typst / fs / clipboard invoke ラッパ
└── COPYING                      # GPLv3
```

### Typst crate との接続(要点)

- `project/world.rs` で `typst::World` を実装(VFS + font book + SlotMap キャッシュ)。
- `ipc/commands/typst.rs::typst_compile` は `typst::compile(&world, &mut tracer)` を直接呼び、結果 Document を state に保持。
- `typst_render` は `typst_render::render(&page.frame, scale, Color::WHITE)` で `Pixmap` → **PNG エンコード → Base64** で IPC 返却。フロントは `<img src="data:image/png;base64,...">` で表示する **raster 方式**。SVG/Canvas/PDF embed は採用せず。
- `typst_autocomplete` は `typst_ide::complete()` をラップするのみ。hover/definition/format 等は未実装。

### LSP

**LSP を持たない**。補完は `typst-ide` の直接呼出しのみで、tinymist 連携なし。

### UI 構成(高レベル)

`+page.svelte` に Editor / Preview / SidePanel / StatusBar を配置する単一ビュー。エディタ入力は throttle で `compile`、debounce で `save` を打つパイプライン。Preview は `typst_compile` イベントを購読してページメタを受け、ページ単位で on-demand に `render` IPC を投げる。

## 4. アクティビティ

### コミット状況(`git log` より抜粋)

```
9736613 2025-04-11  docs: update readme                      ← 最新(README のみ)
a72d25f 2024-03-17  chore(deps): upgrade dependencies (#40)  ← 最後の実装コミット
7457b47 2023-10-29  feat: add keybinds for menu actions
df1084e 2023-09-16  feat: improve diagnostics and add preview state indicator
5872fd0 2023-09-16  chore: upgrade dependencies
12a2331 2023-08-24  feat: add support for locally installed packages
...
```

- 活発期: 2023-04 〜 2023-09(週1-2回)
- 低速化: 2023-10 〜 2024-03(月1回程度)
- 停止: 2024-03 以降(README 更新1本のみ)
- **最終実装コミット**: 2024-03-17(1年以上前)

### 本人による声明(`README.md` 4-6 行目、verbatim)

> Typstudio is not actively maintained. I have not had time to actively maintain this project for the last year. The project will most likely be archived at some point in the foreseeable future.
>
> For those who are looking for an offline alternative, I recommend [Tinymist](https://github.com/Myriad-Dreamin/tinymist).

作者自身が「メンテ放棄」「近く archive 予定」「代替として Tinymist 推奨」と明言している。

### CHANGELOG / RELEASES

CHANGELOG は存在しない。GitHub Releases のみ(自動 draft)。

## 5. ビルド可否

### `cargo check` 結果

`cd ~/Projects/yuhitsu-refs/typstudio && cargo check` は **失敗**。エラー抜粋:

```
error[E0282]: type annotations needed for `Box<_>`
  --> .../time-0.3.34/src/format_description/parse/mod.rs:83:9
```

`time` v0.3.34 の既知リグレッションで、特定 Rust バージョンで compile error を起こす。typst v0.11.0 の transitive dep から流入。`Cargo.lock` の再生成 + Rust toolchain 固定で解消可能と見込まれるが、fork 路線を採らない以上この回避は着手不要。

### フロントエンド scripts

```
dev: vite dev
build: vite build
preview: vite preview
check: svelte-kit sync && svelte-check
lint: prettier + eslint
format: prettier --write
```

install/build は本調査では試していない。

## 6. fork コスト所感(参考情報)

仮に **ライセンスが許した** として見積もった場合のみの話。

| 項目 | 見積もり |
|---|---|
| typst v0.11 → v0.14 API 追従 | 1〜2週間(typst-* 全体、comemo も追従) |
| Tauri 1.6 → 2.x 移行 | 3〜5日(IPC API は大きく変わっていないが設定系は差分あり) |
| time 等 transitive dep 更新 | 2〜3日 |
| Monaco → CodeMirror 6 置換(方針通り) | 1〜2週間(補完・TextMate 再構築) |
| tinymist LSP 連携追加 | 2〜3週間(今は無いため新規実装) |
| 日本語ファースト UI(i18n、和文組版) | 2〜3週間 |
| フォーム型テンプレ / WYSIWYG-lite | スコープ外(+ Phase 2/3) |

**合計で 2〜3 ヶ月**。しかもゴールはゼロスタート v0.1 alpha と大差ない状態 — 既存コードで救えるのは `typst::World` 実装のパターン程度だが、これは公式 typst-cli や tinymist も持つため GPLv3 コードをわざわざ引き込む意味は薄い。

## 7. 所感

Typstudio は Tauri + Monaco + Typst の参考実装として読み物価値は高いが、Yuhitsu 実装の母体としては **ライセンス・メンテ・Typst 追従の三重の不適合**により不適。作者自身の「Tinymist を推奨」宣言が事態を決定づけている。fork は不可、コードの直接流用も GPLv3 汚染を避けるため不可。ただし `project/world.rs` の World 実装パターン、IPC 設計の境界、Monaco 統合時のハマり所(Oniguruma WASM のロード順など)は、Yuhitsu の独立実装時にも設計上の参考になる。**fork は断念し、読み物として 1 度通読した上で yuhitsu-refs 配下に置いたまま凍結する** ことを推奨。
