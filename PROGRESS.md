# Yuhitsu — 進捗管理

最終更新: 2026-04-23
現在のフェーズ: **Phase 0 — 調査 & PoC**

---

## Phase 0: 調査 & PoC(~2週間想定)

### 目的
Typstudio fork 路線 vs ゼロスタート路線の判断材料を集め、技術的実現性を確認する。

### タスク

#### 環境準備
- [ ] 作業ディレクトリ作成 `~/projects/yuhitsu/`
- [ ] Typstudio clone(参照用、別ディレクトリ)
- [ ] Tinymist clone(参照用、別ディレクトリ)
- [ ] Typst 本体 clone(参照用、別ディレクトリ)

#### Typstudio 分析
- [ ] LICENSE 確認(fork 可否、派生物の条件)
- [ ] Cargo.toml / package.json の依存確認
- [ ] `src/` のアーキテクチャ把握
- [ ] 使用エディタライブラリの特定(Monaco / CodeMirror / 独自)
- [ ] 使用 typst crate バージョン
- [ ] リポジトリアクティビティ(最終コミット、Issue/PR 状況)
- [ ] `cargo build` で動作確認
- [ ] → `docs/phase0-typstudio-analysis.md` に記録

#### Tinymist 調査
- [ ] crate 構造確認(`tinymist-query` などが library として使えるか)
- [ ] LSP プロセスとして使う場合の起動方法・プロトコル
- [ ] preview 機構の再利用可能性
- [ ] Yuhitsu 用途での推奨統合方法の所見
- [ ] → `docs/phase0-tinymist-analysis.md` に記録

#### 路線判断
- [ ] fork 路線のメリット/デメリット整理
- [ ] ゼロスタートのメリット/デメリット整理
- [ ] 所要時間見積もり(両路線について)
- [ ] 推奨路線の提案
- [ ] → `docs/phase0-decision.md` に記録
- [ ] 氏の判断待ち → 承認後 Phase 1 へ

### 成果物
- `docs/phase0-typstudio-analysis.md`
- `docs/phase0-tinymist-analysis.md`
- `docs/phase0-decision.md`
- (判断が出たら)初期リポジトリ構成

---

## Phase 1: MVP(Phase 0 完了後、3-4ヶ月)

### 機能
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
