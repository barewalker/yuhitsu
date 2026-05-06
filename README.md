# 右筆 (Yuhitsu)

**業務文書作成の味方。[Typst](https://typst.app) を GUI で、ローカルで、日本語ファーストで。**

> 🇬🇧 [English version](./README.en.md)

> 🚧 **現在 v0.1.0-alpha に向けた開発段階です。** 早期フィードバック大歓迎ですが、本番運用には推奨しません。

---

## これは何?

Yuhitsu(右筆)は、[Typst](https://typst.app) 組版システムのためのデスクトップ GUI エディタです。

名前は、室町〜江戸期に大名や将軍の側近として公文書を起草・清書した専門職「右筆」に由来します。

## なぜ作るか

Typst は LaTeX 代替として注目されている組版システムですが、既存のツールには以下のギャップがあります:

| ツール | OSS | ローカル | GUI | 日本語ファースト |
|---|---|---|---|---|
| Typst 公式 Web アプリ | ❌ | ❌ | ✅ | ❌ |
| Typstify | ❌ | ✅ | ✅ | ❌ |
| Typstudio | ✅ | ✅ | ✅ | ❌(長期停滞) |
| VS Code + Tinymist | ✅ | ✅ | ❌(IDE) | ❌ |

Yuhitsu は「**OSS × ローカル × GUI × 日本語ファースト × 実用**」の空白を埋めます。事務 / 営業 / コマンドラインに不慣れな技術者など、**Typst を直接書くにはハードルが高い層** が業務文書を作れることを目指しています。

## 主な機能(v0.1 alpha 段階)

### Typst エディタ機能
- **ライブプレビュー** — 編集中の未保存内容もリアルタイムに反映(tinymist preview 経由)
- **LSP 統合** — 補完 / 診断 / hover ドキュメント(tinymist LSP 経由)
- **構文ハイライト** — Typst 構文を色分け
- **PDF 出力** — Ctrl+E で 1 ボタン
- **検索 / 置換** — Ctrl+F / Ctrl+H、正規表現 / 大文字小文字 / 単語単位
- **GUI 挿入ボタン** — 太字 / 斜体 / 見出し / リスト / 数式 / コード / リンク / 脚注 / 引用 / 画像 / 表 / 参考文献

### 編集モード(切替可能)
- **OS 標準**(default)
- **vim**(experimental — IME / WebKit 環境で挙動不安定の既知問題あり、上級者向け)
- **emacs**

### ファイル / プロジェクト管理
- **タブ機能** — 複数同時編集、ドラッグ並び替え、ホットエグジット(終了時のタブ復元)
- **プロジェクトビュー(サイドバー)** — ファイルツリー、git status バッジ、右クリックメニュー(新規 / 名前変更 / 削除)
- **画像 / PDF タブ表示** — `.png` / `.svg` / `.pdf` 等を直接タブで開いて閲覧(Typst で取り込む資源として)

### 業務文書テンプレート(同梱、ja / en 両対応)
- 業務報告書 / 技術報告書 / 議事録 / スライド / 空ドキュメント
- **フォーム型テンプレート(簡素版)** — テンプレ関数の引数をフォームに展開、入力で文書本文に反映

### UI / カスタマイズ
- **コマンドパレット**(Ctrl+Shift+P / F1) — 全機能を fuzzy 検索、日本語 locale ではかな・ローマ字・英語で引ける
- **三点リーダーメニュー** — 全機能のカテゴリ別アクセス
- **ツールバー編集 UI** — D&D で並び替え、項目追加 / 削除、プリセット(標準 / ミニマル / 論文寄り)
- **キーバインド設定 UI** — 全コマンドのキーバインドを変更可
- **ダーク / ライトテーマ** — OS 設定に自動追従 or 手動固定
- **日本語 / 英語 UI** — `navigator.language` 自動推測 or 設定で固定
- **自前タイトルバー(CSD)** — ファイル名 / カーソル位置 / 文字数を集約

### 同梱フォント
- **Harano Aji Mincho / Gothic**(Regular + Bold)— [trueroad/HaranoAjiFonts](https://github.com/trueroad/HaranoAjiFonts)、SIL Open Font License 1.1

詳細・予定機能は [PROGRESS.md](./PROGRESS.md) を参照。

---

## ダウンロードとインストール

> 🚧 v0.1.0-alpha のリリース準備中です。リリース後 [Releases](https://github.com/barewalker/yuhitsu/releases) ページから OS 別バイナリを取得できます。

### Linux

`.AppImage`(どのディストロでも動く)or `.deb`(Debian / Ubuntu)を Releases から DL。

```bash
# AppImage
chmod +x yuhitsu_*.AppImage
./yuhitsu_*.AppImage

# .deb
sudo dpkg -i yuhitsu_*.deb
```

> 💡 **日本語入力(IME)が効かない場合**
>
> 日本語ロケールでない Linux に後付けで fcitx5 / ibus 等を入れた環境では、IME 連携用の環境変数がセッションに伝わっていないことがあり、Yuhitsu のエディタで漢字変換ができないことがあります(WebKit2GTK + fcitx5 / ibus 全般の挙動で、Yuhitsu 固有ではありません)。
>
> Ubuntu の場合は `im-config -n fcitx5`(または `ibus`)でセッション設定を行うのが標準です。設定しない場合は、起動時に明示する方法でも回避できます:
>
> ```bash
> # fcitx5 を使う場合
> GTK_IM_MODULE=fcitx XMODIFIERS=@im=fcitx yuhitsu
> # ibus を使う場合
> GTK_IM_MODULE=ibus XMODIFIERS=@im=ibus yuhitsu
> ```
>
> 永続化するなら `~/.profile` や `~/.config/environment.d/im.conf` 等に書きます。

### Windows

`.msi` インストーラを Releases から DL → 実行。

> ⚠️ **SmartScreen 警告について**
> Yuhitsu は個人 OSS のため、Microsoft のコード署名証明書(年額数万円)を取得していません。初回実行時に SmartScreen の青い警告画面が出ますが、**「詳細情報」→「実行」**で起動できます。これは Helix エディタや Alacritty 等の他 OSS でも同様の運用です。
> 将来的に [SignPath.io の OSS 無料署名](https://signpath.io/) などの対応を検討します。

### macOS

> 🚧 alpha 段階では macOS は配布対象外です。CI でビルド成功は確認していますが、開発者が macOS マシンを持たないため動作未検証です。試したい方は [ソースからのビルド](#開発者向け-ソースからのビルド) をお試しください。

---

## 使い方の基本

1. **起動** → 初回起動時にテンプレート選択ダイアログが出ます
2. **テンプレート選択**(業務報告書 / 議事録 等)→ エディタに本文が入る
3. **左上の三点リーダー** から全機能にアクセス可能
4. **F1** または **Ctrl+Shift+P** でコマンドパレット
5. **Ctrl+S** で保存(`.typ` ファイルとして)
6. **Ctrl+E** で PDF 書き出し

詳しい操作は本体の **三点リーダー → ヘルプ → Yuhitsu について** で各種ドキュメントへのリンクが見られます。

---

## 開発者向け(ソースからのビルド)

### 前提

- [Rust](https://www.rust-lang.org/) 1.77+
- [Node.js](https://nodejs.org/) 20+ + [pnpm](https://pnpm.io/) 10+
- Linux:`libwebkit2gtk-4.1-dev`、`libssl-dev`、`libgtk-3-dev` 等(Tauri の Linux ビルド前提)
- macOS:Xcode CLT
- Windows:Microsoft C++ Build Tools + WebView2

`tinymist` は Yuhitsu に sidecar として同梱されるため、開発者が個別にインストールする必要はありません(`scripts/fetch-tinymist.sh` が `pnpm install` 直後に target triple 用のバイナリを取得します)。

### セットアップ

```bash
git clone --recursive https://github.com/barewalker/yuhitsu.git
cd yuhitsu
./scripts/fetch-tinymist.sh   # tinymist sidecar を取得
cd app
pnpm install
pnpm tauri dev
```

> ⚠️ **`--recursive`** が重要:Harano Aji フォントを git submodule として取り込んでいるため。
>
> 💡 **`fetch-tinymist.sh`** は冪等(同じ binary がある場合は再 DL しない)。版数を変えたい時は `TINYMIST_VERSION=v0.14.16 ./scripts/fetch-tinymist.sh` のように環境変数で指定。

### リリースビルド

```bash
cd app
pnpm tauri build
# 成果物は app/src-tauri/target/release/bundle/ 以下
```

---

## ロードマップ

- **Phase 0**(完了):調査 & PoC、技術選定
- **Phase 1**(現在):MVP — エディタ / テンプレ / プレビュー / 配布
- **Phase 2**(予定):UX 強化、内蔵 AI 機能、git お世話機能
- **Phase 3**(予定):WYSIWYG-lite モード、`.typz` 単一ファイルバンドル、MCP サーバ化(外部エージェント連携)

詳細は [PROGRESS.md](./PROGRESS.md)。

---

## 技術スタック

- **シェル**:[Tauri 2](https://tauri.app/) (Rust + OS WebView)
- **エディタ**:[CodeMirror 6](https://codemirror.net/)
- **Typst エンジン**:[Typst](https://github.com/typst/typst) を [tinymist](https://github.com/Myriad-Dreamin/tinymist) 経由で使用(LSP / preview / compile)

---

## ライセンス

- 本体:**[Apache-2.0](./LICENSE)**
- 同梱フォント(Harano Aji Mincho / Gothic):**SIL Open Font License 1.1** (Copyright © trueroad)

---

## 作者

[barewalker](https://github.com/barewalker)

業務文書作成の現場ニーズから出発した個人プロジェクトです。同じ課題を抱える方に広く使っていただければ幸いです。

---

## 関連リンク

- [Typst 公式](https://typst.app/)
- [Typst 公式日本語ドキュメント](https://typst-jp.github.io/docs/)
- [Tinymist](https://github.com/Myriad-Dreamin/tinymist)
- [Harano Aji Fonts](https://github.com/trueroad/HaranoAjiFonts)

---

## 謝辞

本プロジェクトの実装は [Claude Code](https://claude.com/claude-code)(Anthropic)の支援のもとで進められました。技術選定・設計判断・品質責任は作者が担っています。
