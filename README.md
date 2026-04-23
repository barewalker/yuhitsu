# 右筆 (Yuhitsu)

**業務文書作成の味方。Typst を GUI で、ローカルで、日本語ファーストで。**

> ⚠️ **現在 Phase 0(調査 & PoC)段階です。動くものはまだありません。**

## これは何?

Yuhitsu(右筆)は、[Typst](https://typst.app) 組版システムのためのデスクトップ GUI エディタです。

名前は、室町〜江戸期に大名や将軍の側近として公文書を起草・清書した専門職「右筆」に由来します。

## なぜ作るか

Typst は LaTeX 代替として注目されている組版システムですが、既存のツールには以下のギャップがあります:

- **[Typst 公式 Web アプリ](https://typst.app)** はクラウド必須で、機密文書を扱う企業・組織には導入しづらい
- **[Typstify](https://typstify.com)** は完成度が高いが商用・クローズドソース
- **[Typstudio](https://github.com/Cubxity/typstudio)** は OSS だが長期停滞
- **VS Code + Tinymist** は強力だが、IDE に慣れない層には届かない

Yuhitsu は「**OSS × ローカル × GUI × 日本語ファースト**」の空白を埋めます。

## 特徴(予定)

- 🇯🇵 **日本語ファースト**: Harano Aji フォント同梱、和文組版の罠を自動回避
- 📝 **業務文書テンプレート同梱**: 報告書、稟議、議事録、論文、スライド
- 🖱️ **GUI 親和的**: IDE に不慣れな方でも使える live preview エディタ
- ✨ **WYSIWYG-lite モード**(将来): 記法を隠してプレーンに、編集時は見える
- 📋 **フォーム型テンプレート**(将来): テンプレート引数を自動で入力フォーム化
- 🔒 **完全ローカル**: データは端末から出ない
- 🆓 **オープンソース**: Apache-2.0

## 開発状況

- [x] 構想・企画
- [ ] **Phase 0: 調査 & PoC**(現在ここ)
- [ ] Phase 1: MVP
- [ ] Phase 2: UX 強化
- [ ] Phase 3: WYSIWYG-lite モード

詳細は [PROGRESS.md](./PROGRESS.md) を参照。

## 技術スタック(暫定)

- [Tauri](https://tauri.app/) (Rust + WebView)
- [CodeMirror 6](https://codemirror.net/)
- [typst](https://github.com/typst/typst) crate
- [tinymist](https://github.com/Myriad-Dreamin/tinymist) LSP

## ライセンス

[Apache-2.0](./LICENSE)

## 作者

[barewalker](https://github.com/barewalker)

業務文書作成の現場ニーズから出発した個人プロジェクトです。同じ課題を抱える方に広く使っていただければ幸いです。
