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

## 技術スタック(Phase 0 完了時点で確定)

- **デスクトップシェル**: **Tauri 2.x**(Rust + WebView)
- **エディタ**: **CodeMirror 6**(Monaco は電力消費・和文 IME・重量で不採用)
- **Typst 統合**: 公式 **typst crate**(typst/typst)を Tauri バックエンドに直接リンク。tinymist 同梱の Myriad-Dreamin fork は引き込まない
- **LSP**: **tinymist を subprocess として spawn**(stdio JSON-RPC)。エディタ知能(補完/診断/hover/定義/format)はすべて tinymist に委譲し、自前実装しない
- **プレビュー**: **tinymist preview の WebSocket + incremental SVG** を流用。PNG ラスタ方式は採らない

詳細根拠: `docs/phase0-typstudio-analysis.md` / `docs/phase0-tinymist-analysis.md` / `docs/phase0-decision.md`

## リポジトリ方針

- 本体: `barewalker/yuhitsu`(public, Apache-2.0 予定)
- 日本語汎用テンプレート: `barewalker/yuhitsu-ja`(public)

### 機密情報の分離(絶対厳守)

本体リポジトリは public 公開を前提とする。**業務 / 所属 / 取引に紐づく一切の情報を持ち込まない**。
README / PROGRESS / CLAUDE / コミットメッセージ / コード / コメント / テストデータ など、文脈を問わず適用する。

具体的には以下を含めない(網羅ではなく類型):

- 所属組織や関係する企業・人物・チームの固有名
- 業務 / 案件 / 顧客 / 製品 / プロジェクトに紐づく固有名詞
- 実データを含むサンプル文書(メール本文、社内議事録、スクリーンショット等)
- 業務固有の識別体系(文書番号、ID 採番ルール、参照体系、社内用語)
- 業務固有のドキュメント(下書き、特許ドラフト、契約書、内部仕様、レビューコメント)
- 業務インフラ(内部 API、認証基盤、シークレット管理、社内 SaaS)との密結合コード
- ローカル環境固有のパス / 認証情報 / 鍵 / トークン

迷ったら**入れない**を選ぶ。社内で必要なものは別 private リポに分けて管理する。

## ライセンス

- 本体: **Apache-2.0**(Typst エコシステムと整合、特許保護を確保)
- 同梱フォントは SIL OFL 1.1
- ソースファイル冒頭のライセンスヘッダは付与しない方針(LICENSE ファイルで一元管理)

### GPLv3 コード隔離(絶対厳守)

Phase 0 で Typstudio の fork を断念した経緯(`docs/phase0-decision.md`)から、本体リポには以下を**一切持ち込まない**:

- Typstudio(GPLv3)由来のコード断片・ファイル・アセット
- その他 GPL 系ライセンス(GPLv2/v3, AGPL, LGPL など強コピーレフト)のコード

参照実装としての読解は可(`~/Projects/yuhitsu-refs/typstudio/` に保持)。ただし関数単位・ファイル単位のコピーや、見ながらの「同型コーディング」は GPL 汚染リスクがあるため避ける。アルゴリズムの理解 → 自分の言葉で再設計、を徹底する。

依存追加時は Cargo / npm パッケージのライセンスを必ず確認し、GPL 系が混入しないようにする(MIT / Apache-2.0 / BSD / MPL-2.0 / ISC / Unlicense は OK、GPL 系・SSPL・Commons Clause は不可、判断つかない時は氏に確認)。

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
