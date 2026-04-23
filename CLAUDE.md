# 右筆 (Yuhitsu) — Claude Code 向けプロジェクト指針

このファイルは、このリポジトリで作業する Claude Code への指示書です。
新しいセッションを開始した Claude Code は、まずこのファイルを読んでから作業を開始してください。

## プロジェクト概要

Yuhitsu は Typst をベースにした、GUI 親和的なローカルデスクトップエディタ。
事務・営業・CUI 苦手な技術者まで含めた「業務文書作成の味方」を目指す。

名前の由来: 室町〜江戸期、大名や将軍の側近として公文書を起草・清書した専門職「右筆」から。

## ポジショニング(既存との差別化)

既存ツールには以下のギャップがある:

| ツール | OSS | ローカル | GUI | 実用レベル |
|---|---|---|---|---|
| Typst 公式 Web アプリ | ❌ | ❌ | ✅ | ✅ |
| Typstify | ❌ | ✅ | ✅ | ✅ |
| Typstudio | ✅ | ✅ | ✅ | ❌ (長期停滞) |
| VS Code + Tinymist | ✅ | ✅ | ❌ (IDE) | ✅ |

→ Yuhitsu は「**OSS × ローカル × GUI × 実用**」の空白座標を埋める。

## 差別化ポイント

1. **WYSIWYG-lite モード**(v0.3 以降、ただし設計は最初から考慮)
   - Typora / Obsidian Live Preview と同じ発想
   - AST ベースで、表示時は記法を非表示、カーソル位置で記法を表示
2. **フォーム型テンプレート入力**
   - `#show: template.with(顧客名: "...", 試料名: "...")` の引数を自動でフォーム UI に展開
   - 非技術層でも「穴埋めするだけ」で文書完成
3. **日本語ファースト**
   - Harano Aji Mincho / Gothic 同梱(SIL OFL 1.1、再配布可)
   - `set text(lang: "ja")` デフォルト
   - 和文行長=全角整数倍マージン自動調整
   - 業務文書テンプレート同梱(報告書、稟議、議事録、論文、スライド)
   - 日本語 UI
4. **表・数式・図・参考文献の GUI 挿入ボタン**
   - 将来的に Detypify(手書き数式認識)統合も検討

## 技術スタック(暫定)

- **UI フレームワーク**: Tauri (Rust + WebView)
- **エディタ**: CodeMirror 6(Monaco は電力消費と重さで非推奨)
- **Typst 統合**: typst crate を Tauri バックエンドに埋め込み
- **LSP**: tinymist を library または LSP プロセスとして統合
- **プレビュー**: SVG または Canvas レンダリング(tinymist の preview 機構利用)

ただし Phase 0 の調査結果次第で変更の可能性あり。

## リポジトリ方針

- 本体: `barewalker/yuhitsu`(public, Apache-2.0 予定)
- 日本語汎用テンプレート: `barewalker/yuhitsu-ja`(public)
- (redacted internal template repo)(private、**本体と厳密に分離**)

### 社内情報の分離(絶対厳守)

本体リポジトリには以下を絶対に含めない:
- 顧客名、製品名、案件名
- 実データを含むサンプル文書
- 社内文書番号体系、QMS ID
- 特許ドラフト
- (redacted internal infra)との密結合コード

## ライセンス

- 本体: **Apache-2.0**(Typst エコシステムと整合、特許保護を確保)
- Typstudio を fork する場合は元ライセンスに従う(要確認、ただし fork 先は Apache-2.0 を選択可能)
- 同梱フォントは SIL OFL 1.1
- ソースファイル冒頭のライセンスヘッダは付与しない方針(LICENSE ファイルで一元管理)

## コミット規約

- Conventional Commits (feat:, fix:, docs:, refactor:, chore: 等)
- 日本語コミットメッセージ可、ただし summary 行は簡潔に
- co-author として Claude を自動付与しない(barewalker 単独名義)

## セキュリティ

- `.gitignore` で社内情報、鍵、実データを徹底除外
- pre-commit hook で社内固有語のリーク検知(実装予定)
- 署名鍵、API キーは絶対にコミットしない

## 作業進行の原則

- **PROGRESS.md を常に最新に保つ**。タスクの完了/追加/変更があれば即反映
- **判断を要する分岐点では氏に確認**。特に以下:
  - 路線変更(fork ↔ scratch)
  - 依存ライブラリ追加
  - ライセンス変更
  - 公開/非公開の切り替え
- **調査フェーズではコードを書かない**。レポート作成に専念
- **ドキュメントは日本語ファースト**、必要に応じて英語版を追加

## 参考リンク

- Typst: https://github.com/typst/typst
- Tinymist: https://github.com/Myriad-Dreamin/tinymist
- Typstudio (fork 候補): https://github.com/Cubxity/typstudio
- Typstify (比較対象): https://typstify.com
- Typst 公式日本語ドキュメント: https://typst-jp.github.io/docs/
