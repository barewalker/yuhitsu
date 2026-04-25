# Yuhitsu — 進捗管理

最終更新: 2026-04-25
現在のフェーズ: **Phase 1 — Sprint 1 進行中**

---

## Phase 0: 調査 & PoC(~2週間想定)

### 目的
Typstudio fork 路線 vs ゼロスタート路線の判断材料を集め、技術的実現性を確認する。

### タスク

#### 環境準備
- [x] 作業ディレクトリ作成 `~/Projects/yuhitsu/`
- [x] Typstudio clone(`~/Projects/yuhitsu-refs/typstudio/`, shallow depth=50)
- [x] Tinymist clone(`~/Projects/yuhitsu-refs/tinymist/`, shallow depth=50)
- [x] Typst 本体 clone(`~/Projects/yuhitsu-refs/typst/`, shallow depth=50)
- [x] git ブランチ master → main にリネーム

#### Typstudio 分析
- [x] LICENSE 確認 → **GPLv3 / Apache-2.0 と非互換、fork 不可**
- [x] Cargo.toml / package.json の依存確認(typst v0.11.0, tauri 1.6, monaco 0.47, svelte 4)
- [x] `src/` のアーキテクチャ把握(src-tauri ≒1000 LOC, src ≒1400 LOC)
- [x] 使用エディタライブラリの特定 → **Monaco**
- [x] 使用 typst crate バージョン → **0.11.0**(現行 v0.14 系から 2 世代遅れ)
- [x] リポジトリアクティビティ → **実質停止**(最終実装 2024-03、作者が archive 予告)
- [x] `cargo check` 実行 → time crate で失敗(深追いせず)
- [x] → [docs/phase0-typstudio-analysis.md](./docs/phase0-typstudio-analysis.md) 完成

#### Tinymist 調査
- [x] crate 構造確認(workspace 26 crate、`tinymist-query` 等は path dep のみ)
- [x] LSP プロセスとして使う場合の起動方法・プロトコル(`tinymist lsp`、stdio JSON-RPC)
- [x] preview 機構の再利用可能性 → **WebSocket + incremental SVG、流用可**
- [x] Yuhitsu 用途での推奨統合方法 → **(A) LSP subprocess 方式**
- [x] → [docs/phase0-tinymist-analysis.md](./docs/phase0-tinymist-analysis.md) 完成

#### 路線判断
- [x] fork 路線のメリット/デメリット整理
- [x] ゼロスタートのメリット/デメリット整理
- [x] 所要時間見積もり(fork 13〜17 週、ゼロスタート 11〜14 週)
- [x] 推奨路線の提案 → **ゼロスタート + Tinymist LSP 統合**
- [x] → [docs/phase0-decision.md](./docs/phase0-decision.md) 完成
- [x] **氏承認(2026-04-24)**:fork しない、ゼロスタート路線で確定

### 成果物
- `docs/phase0-typstudio-analysis.md`
- `docs/phase0-tinymist-analysis.md`
- `docs/phase0-decision.md`
- (判断が出たら)初期リポジトリ構成

---

## Phase 1: MVP(Phase 0 完了後、3-4ヶ月)

### Sprint 1: 環境セットアップ + 骨組み(2026-04-25 進行中)
- [x] pnpm 10.33.2 導入(公式 standalone installer 経由、`~/.local/share/pnpm`)
- [x] tauri-cli 2.10.1 導入(`cargo install`)
- [x] tinymist v0.14.16 導入(GitHub Releases prebuilt、`~/.local/bin`)
- [x] `pnpm create tauri-app` で svelte-ts + Tauri 2 骨組み生成
- [x] `app/` サブディレクトリ配置(Cargo workspace ルート構成)
- [x] Bundle identifier `com.barewalker.yuhitsu` 設定
- [x] License 修正(Tauri テンプレ既定 MIT → Apache-2.0)
- [x] LICENSE ファイル(Apache-2.0)を repo 直下に配置
- [x] root `Cargo.toml`(workspace 宣言)/ `.gitignore` 整備
- [x] `cargo check` 通過(47.82s、Tauri 2 依存解決済み)
- [x] `pnpm build` 通過(1.75s、frontend production ビルド)
- [x] **`pnpm tauri dev` で GUI ウィンドウ起動確認**(2026-04-25 氏が確認、デフォルトの Tauri+Svelte welcome 画面表示)
- [ ] Sprint 1 commit

### Sprint 2 以降の機能
- [ ] Tauri シェル、ファイル/フォルダ開閉
- [ ] CodeMirror 6 + Tinymist LSP 統合
- [ ] Live preview pane
- [ ] PDF エクスポート
- [ ] 日本語テンプレート5本
  - [ ] 業務報告書
  - [ ] 稟議書
  - [ ] 議事録
  - [ ] 技術論文(材料学会スタイル等)
  - [ ] スライド
- [ ] Harano Aji フォント同梱
- [ ] 日本語 UI(i18n 基盤)

### 配布
- [ ] winget で配布可能にする
- [ ] GitHub Releases で Windows/macOS/Linux バイナリ提供
- [ ] コード署名(氏の GlobalSign USB トークン活用)

### リリース
- [ ] v0.1.0 alpha リリース
- [ ] 日本語 README 最終版
- [ ] 英語 README 整備

---

## Phase 2: UX 強化(Phase 1 リリース後、+3ヶ月)

- [ ] フォーム型テンプレート入力(差別化ポイント #2)
- [ ] 表・数式・画像挿入 GUI ボタン(差別化ポイント #4)
- [ ] パッケージ管理 UI(Typst Universe 連携)
- [ ] テンプレートギャラリー
- [ ] v0.2.0 リリース

---

## Phase 3: WYSIWYG-lite モード(差別化の本丸)

- [ ] AST ベース dual-rendering 設計書作成
- [ ] 見出し・強調・引用の記法非表示化(PoC)
- [ ] カーソル位置での記法表示切り替え
- [ ] 表・リスト等の WYSIWYG 化
- [ ] v0.3.0 リリース

---

## バックログ / 検討事項

- Typst の breaking change 追従コスト(月1ペース)
- Harano Aji 同梱のインストーラサイズ影響(数十MB × 複数ウェイト)
  - → 初回起動時オンデマンド DL 方式も検討
- Detypify(手書き数式)統合可否
- (redacted internal plugin)
- 縦組みテンプレート
- 共同編集機能(Typst On-Premises と被るため優先度低)

---

## 判断ログ

意思決定の経緯を記録する。

### 2026-04-23: プロジェクト開始
- 名前を「右筆 (Yuhitsu)」に決定
- GitHub 公開方針
- ライセンスは **Apache-2.0** 確定
- 本体 / 汎用テンプレ / private テンプレの三層分離
- 自社専用 + 他社利用可の両立を目指す

### 2026-04-23: Phase 0 調査完了
- Typstudio 分析:**fork 不可**(GPLv3 が Apache-2.0 と非互換、作者がメンテ放棄宣言)
- Tinymist 調査:**Apache-2.0 で活発、Yuhitsu の必要 LSP 機能を全て具備**
- 推奨路線:**ゼロスタート + Tinymist を LSP subprocess として統合**
- 詳細は `docs/phase0-decision.md`
- 補足:git ブランチを master → main にリネーム済み(参照リポは `~/Projects/yuhitsu-refs/` に配置)

### 2026-04-24: 路線確定 — fork しない
- 氏承認:**ゼロスタート路線で確定**
- Typstudio は本体に流用しない(GPLv3 汚染回避)
- 参照リポ `~/Projects/yuhitsu-refs/typstudio/` は読み物として保持、本体には一切持ち込まない

### 2026-04-24: 技術スタック確定
- ✅ **Tauri 2.x** 採用(1.x は不採用)
- ✅ **typst crate は公式(typst/typst)のみ** 使用(Myriad-Dreamin fork は引き込まない)
- ✅ **GPLv3 隔離ルールを CLAUDE.md に明文化**(Typstudio 由来コード持込み禁止、依存追加時のライセンス確認義務)
- → Phase 1 着手準備完了
