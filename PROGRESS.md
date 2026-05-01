# Yuhitsu — 進捗管理

最終更新: 2026-05-01
現在のフェーズ: **Phase 1 — Sprint 3 進行中(UI 文字列 i18n 化完了、フォーム型テンプレ簡素版が次の差別化ポイント)**

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
- [x] Sprint 1 commit(`91970ec feat: scaffold Tauri 2 + Svelte 5 app under app/`)

### Sprint 2: 基本編集ループ(ファイル開閉 → 編集 → プレビュー → 出力)

実装順序: **(1) ファイル開閉 → (2) エディタ → (3) ライブプレビュー → (4) PDF → (5) 操作モード3種 → (6) LSP 統合**(2026-04-26 順序見直し)。
最低限の編集 1 ループを通すことを優先し、各機能は MVP 水準でつなぐ。typst.app 風の 2 ペイン UI に早く近づけるためライブプレビューを LSP より先に着手する方針。

#### (1) Tauri シェル、ファイル/フォルダ開閉(c7be3c4 で完了)
- [x] `tauri-plugin-dialog` 追加(`tauri-plugin-fs` は使わず Rust 側 command で完結 = ハイブリッド方式 C)
- [x] Open File / Save の Tauri command(`.typ` 想定、UTF-8、`open_file` / `save_file`)
- [x] Svelte 側ツールバー(開く / 保存 / 名前を付けて保存)+ Ctrl/Cmd+O / S / Shift+S
- [x] dirty 状態管理(`*` マーク、`onCloseRequested` で終了時の確認)
- [ ] Open Folder(将来のファイルツリー用、最低限のディレクトリ選択)— **Sprint 2 (2) 以降に繰越**(現時点で不要)

#### (2) CodeMirror 6 + Typst syntax highlight
- [x] CodeMirror 6 を Svelte に組み込み(`@codemirror/state`, `@codemirror/view`, `@codemirror/commands`)— commit `977c59b`
- [x] dirty 状態で終了確認 → 「はい」で閉じない問題を修正(`core:window:allow-destroy` 追加)— commit `38b3017`
- [x] Typst 用 syntax highlighting(`codemirror-lang-typst` v0.4.0、Apache-2.0、Typst 公式 typst-syntax を WASM 化)— commit `2d576d6`
  - [x] `vite-plugin-wasm` + `vite-plugin-top-level-await` 導入(WASM ESM 対応)
  - [x] SvelteKit hydration の TDZ 回避のため Editor.svelte を動的 import
  - [x] One Dark 風の自前 HighlightStyle を `Prec.highest` で同梱版に優先させる
  - [x] `t.list`(`-` `+`)・`t.definitionOperator`(`/`)など Typst 固有タグをカバー

#### (3) Live preview pane(順序を上げて Sprint 2 内で先行実装)
- [x] tinymist preview を起動(subprocess、`--no-open --data-plane-host 127.0.0.1:23625 --control-plane-host 127.0.0.1:23626`)— commit `e1881c5`
- [x] +page.svelte を 2 ペイン化(左: エディタ、右: プレビュー)
- [x] **iframe 方式で MVP 実装**(http://127.0.0.1:23625/ をそのまま表示、tinymist 公式フロントエンド HTML を流用)
- [x] WindowEvent::Destroyed で subprocess を kill(プロセスリーク防止)
- [x] ファイル切替時の preview 再起動・既存パス保存時の自動再コンパイル動作確認
- [ ] HTTP プローブによる起動完了判定(現状は 1.5 秒固定 wait)
- [ ] 自前 WebSocket クライアントへの置き換え(エディタ ↔ preview 双方向同期、クリックジャンプ等)
- [ ] エディタ ←→ preview の同期スクロール
- [ ] 編集中(未保存)もリアルタイムにプレビュー反映(現状は保存時に tinymist が watch 検知)

#### (4) PDF エクスポート
- [x] **MVP: tinymist compile 経由で PDF 出力**(Tauri command + ツールバー + Ctrl+E)— commit `af1cf8f`
  - 採用根拠:既に subprocess で tinymist が動いており、最小コードで完結。フォント・パッケージ解決も tinymist 任せにできる
- [x] 編集中の自動保存と緑色メッセージで自動保存の事実を通知
- [ ] 公式 typst crate を Tauri バックエンドに直接リンクする方式への移行検討(フォント同梱・カスタムレンダリングが必要になった段階で)
- [ ] File → Export PDF メニュー(現状はツールバーボタンのみ)
- [ ] フォントロード経路の暫定実装(同梱フォントは Sprint 3 以降)
- [ ] 日本語テンプレート5本
  - [ ] 業務報告書
  - [ ] 稟議書
  - [ ] 議事録
  - [ ] 技術論文(材料学会スタイル等)
  - [ ] スライド
- [ ] Harano Aji フォント同梱
- [ ] 日本語 UI(i18n 基盤)

#### (5) エディタ操作モード3種ビルトイン(差別化ポイント、設定で切替)
- [x] OS 標準(CodeMirror 6 デフォルトキーマップ)
- [x] vim(`@replit/codemirror-vim` v6.3.0、MIT)
- [x] emacs(`@replit/codemirror-emacs` v6.1.0、MIT)
- [x] 設定永続化(`tauri-plugin-store` v2.4.2、MIT)— commit `d241c6a`
- [x] Compartment による動的切替(再マウント不要)
- [x] ツールバーのセレクトボックス UI(日本語ラベル: 標準 / vim / emacs)

#### (6) Tinymist LSP 統合(実装重く最後に)
- [x] Tauri バックエンドから tinymist lsp を subprocess spawn(stdio JSON-RPC、Content-Length ベースのフレーミングを Rust 側で処理)— commit `72d4f4d`
- [x] LSP クライアントは `@codemirror/lsp-client` v6.2.3(MIT、CodeMirror 公式)を採用
- [x] 補完(`#l` で `let` `link` 等のポップアップ): 動作
- [x] 診断(typo・引数エラーで赤波線): 動作
- [x] hover(関数名にカーソルを当ててツールチップ): 動作
- [ ] 定義ジャンプ / format / signature help / rename(Phase 2 以降で改善)
- [ ] hover の Markdown 整形改善(現状は素のままで素っ気ない)
- [ ] 未閉鎖の数式など、tinymist 側の挙動依存の診断改善

#### Phase 1 で並行して仕込む下準備(AI エージェント連携の素地)
- [x] エディタ操作 API レイヤを Svelte UI / Tauri command / 将来の MCP ハンドラから共通で呼べる形に整理 — Sprint 3 (3) で `$lib/commands.ts` のコマンドカタログ化が完了。`{ id, label, run, ... }` の形で 19 種を集約し、ツールバー / キーバインド / 設定永続化が同じ ID を参照する。MCP ハンドラから呼ぶ際もこのカタログを再利用できる構造
- [ ] 文書状態(全文・カーソル・選択範囲・dirty)の JSON シリアライズ可能性確保
- [x] Tauri Store の設定領域に `ai.*` サブカテゴリを切る — 操作モード実装の副産物として完了(`$lib/settings.ts` の Settings 型に領域確保済み)
- [ ] `@codemirror/merge`(MIT)を評価(後で AI 提案 diff を見せる UI の素地)

#### Sprint 3: 一般ユーザに「コードを書かせない」ための橋渡し(2026-04-28 氏合意で Phase 2 前倒し着手)
**狙い**: 現状の「Typst を書ける人にとって便利な GUI」から「コード非経験層が文書を作れる GUI」へ転換する。Yuhitsu のターゲット(事務・営業・CUI 苦手な技術者)が初めてアプリを開いた時の体験を変える。
- [x] **起動時テンプレート選択ダイアログ + 多言語テンプレート + 参考文献挿入**(2026-05-01 実装)
  - [x] 初期 6 種:空ドキュメント / 業務報告書 / 技術報告書 / 議事録 / 簡易レター / スライド
  - [x] テンプレファイル構造:`app/src/lib/templates/<id>/{meta.json, ja.typ, en.typ}` で同梱、Vite import.meta.glob で静的解決
  - [x] meta.json に `title.{ja,en}` / `description.{ja,en}` を持たせ、ローダー側で locale 解決
  - [x] 用紙サイズは本文中の `{{paper}}` プレースホルダで保持、`document.paperSize` から流し込む
  - [x] 表示タイミング:**初回起動で自動表示(D 案、`flags.firstRunDone` を立てて以降は出さない)+ 専用コマンド `new-from-template` (Ctrl+Shift+T、FilePlus2)**
  - [x] カード選択時:active タブが空タブなら本文差替、そうでなければ新規タブを作って差替
  - [x] 設定スキーマ拡張:`appearance.locale` (auto/ja/en)、`document.paperSize` (auto/a4/letter/b5)、`flags.firstRunDone`
  - [x] `$lib/i18n/locale.ts` 新設(`navigator.language` → ja/en、未対応は en フォールバック)
  - [x] 技術報告書テンプレに `#bibliography(...)` を bibtex / Hayagriva 両対応のコメント付きで含める
  - [x] **参考文献挿入コマンド**(`bibliography`、BookOpen):`.bib` / `.yml` / `.yaml` ファイル選択 → 相対パス化 → 末尾に `#bibliography("...")` 挿入
- [x] **GUI 挿入ボタン最小セット**(太字・斜体・見出し H1〜H3・箇条書きリスト・番号付きリスト)
  - [x] `$lib/editor-commands.ts`(EditorView を引数にとる純粋関数群、UI 非依存)
  - [x] Editor.svelte に `onReady` / `onTeardown` コールバックを追加し view を親に公開
  - [x] ツールバーに B / I / H1 / H2 / H3 / • / 1. ボタン(`title` でホバー説明)
  - [x] Ctrl+B / Ctrl+I のショートカット(view 不在時はパススルーで vim Normal モードと共存)
  - [x] 行頭マーカー切替時のインデント保持、トグル動作(同種なら除去・別種なら置換・なしなら付与)
- [x] **GUI 挿入ボタン拡張セット**(表・画像・数式・脚注・引用・コード(インライン/ブロック)・リンク)
  - [x] `$lib/commands.ts` 新設:全コマンド(ファイル系 + 挿入系 19 種)を `{ id, label, icon, buttonClass, defaultKey, needsEditor, run }` でカタログ化
  - [x] `$lib/editor-commands.ts` に `toggleMath` / `toggleInlineCode` / `insertCodeBlock` / `insertLink` / `insertFootnote` / `insertQuote` / `insertTable` / `insertImage` を追加
  - [x] 表は最小 2x2 を挿入、`insertTable(view, { columns, rows })` 引数化(将来「列数指定モーダル」を増設しやすい形に)
  - [x] 画像は `#figure(image("..."), caption: [|])` で挿入し caption にカーソル(テンプレート側 `#show figure: ...` でスタイル統一できる Typst 流儀)
  - [x] ツールバーを設定駆動に書き換え(`toolbarItems: ToolbarItem[]` を `{#each}` で描画、divider もデータ表現)
  - [x] ボタンを Lucide ピクトグラム化(`@lucide/svelte` v1.14、ISC、tree-shakable な個別 import)
  - [x] キーバインド一括登録(`onKeydown` がカタログを順次照合、`effectiveKey` で override → defaultKey の順)
  - [x] プリセット 3 種(標準 / ミニマル / 論文寄り)を `TOOLBAR_PRESETS` で内蔵
  - [x] 画像挿入はファイル選択ダイアログ +(`getCurrentWebview().onDragDropEvent` 経由の)ウィンドウ D&D、ファイル相対パスへ自動正規化(`../` を含む完全相対化)
  - [x] tinymist の `--root` を filesystem ルート(Linux/Mac は `/`、Windows は入力ドライブ)に統一(preview / PDF)。LSP の rootUri は workspace 機能のためファイルの親ディレクトリのまま
  - [x] hover 等に出る外部 URL クリックは `tauri-plugin-opener` 経由で OS デフォルトに流す
  - [x] dev_log bridge(JS → Rust stderr)を追加し、開発時にフロントログを Rust 出力で確認可能
- [x] **ワークスペース表示制御**(Sprint 3 中に氏の要望で追加)
  - [x] プレビュー on/off トグル(ツールバーボタン + Ctrl+Shift+P、`toggle-preview` コマンドとしてカタログ化)
  - [x] エディタ / プレビュー境界の可変リサイズ(pointer events ベースのスプリッタ、最小/最大 10〜90%)
  - [x] 設定永続化(`workspace.previewVisible` / `workspace.editorPaneRatio` を Tauri Store に保存)
  - [x] 既存設定への migration(`toggle-preview` を末尾に自動追加)
- [x] **プロジェクトビュー(サイドバー、Sprint 2 (1) からの繰越を Sprint 3 内で回収)**
  - [x] Rust `list_directory` コマンド(全ファイル表示、隠しファイル + 定番ノイズフォルダ(`node_modules` / `target` / `.git` / `.svelte-kit` 等)を除外、深さ制限 8)
  - [x] フロント `$lib/project.ts` + `ProjectTree.svelte`(再帰描画は Svelte 5 流の self-import)
  - [x] ファイルアイコンを拡張子で分岐(`.typ` / `.pdf` / 画像 / その他汎用)
  - [x] フォルダ選択(`@tauri-apps/plugin-dialog` の `directory: true`)、永続化(`workspace.currentFolder`)、起動時自動復元
  - [x] ツリーからのファイル切替:`.typ` はエディタで開く(dirty 時の確認込み、現在ファイルのハイライト)、それ以外(PDF / 画像 / 任意)は `openUrl` で OS デフォルトに流す
  - [x] サイドバー on/off + 幅リサイズ(`workspace.projectViewVisible` / `workspace.projectPaneRatio`)、`toggle-project-view` コマンド + Ctrl+Shift+E
  - [x] 既存設定への migration(`open-folder` / `toggle-project-view` を自動追加)
- [ ] プロジェクトビューの拡張(右クリックメニュー、新規・リネーム・削除、git status 連携)
- [x] **タブ機能 + テキストファイル対応**(Sprint 3 中に氏の要望で追加、競合 Typstudio との比較で必要性が浮上)
  - [x] Tab 型(`{ id, path, content, dirty, cursorAnchor, cursorHead, scrollTop }`)、複数同時編集
  - [x] 既存タブ再利用(同じパスならそれを active に)/ 空タブ再利用(無題かつ未編集のタブは差し替え)/ 新規タブ追加
  - [x] タブ切替時にカーソル / スクロール位置を per-tab で保持(`Editor.svelte` の `onValueApplied` フックで親が復元)
  - [x] タブを閉じる(dirty 時の確認 / 全閉じで空タブ自動生成)
  - [x] ツールバー:`new-tab`(Ctrl+T)/ `close-tab`(Ctrl+W)を追加、各タブに ✕ ボタンと「+」新規ボタン
  - [x] テキストファイル全般を開けるよう `openFile` のフィルタ撤廃、ツリーからは `.typ` / `.md` / `.csv` 等のテキスト系はタブで、バイナリは `openUrl` で OS デフォルト
  - [x] Typst 以外のファイルでは構文ハイライトを plain に、LSP / preview / PDF 機能を無効化(`Editor.svelte` に `languageMode` Compartment 追加)
- [ ] タブの永続化(次回起動時に開いていたタブを復元)
- [x] タブの D&D 並び替え(HTML5 Drag and Drop API、依存追加なし、ドラッグ中は半透明 + ドロップ先に左ボーダー)
- [ ] Ctrl+Tab 切替 / 中ボタン閉じ(後回し)
- [x] **テーマ機能(自動 / ライト / ダーク)**(Sprint 3 中に氏の要望で追加、Phase 2 予定の前倒し)
  - [x] 全 UI 色を CSS 変数化(`app.html` の `:root` に 27 種、背景・ボーダー・文字・アクセント・ステータス・Syntax)
  - [x] `:root[data-theme="light"]` にライトテーマ(One Light 風)を定義、`color-scheme` も追従
  - [x] `settings.appearance.theme` で永続化(`"auto" | "light" | "dark"`、デフォルト `"auto"`)
  - [x] "自動" は `prefers-color-scheme` に追従、OS 側変更も `matchMedia` で即反映
  - [x] CodeMirror の `EditorView.theme` / `HighlightStyle` も `var(...)` 化してテーマ切替に連動
- [x] **UI 文字最小化方針 + 設定ファイル直編集ルート**(Sprint 3 中に氏の方針確立)
  - [x] ツールバーから操作モード / テーマ セレクトを撤去(「UI に文字を使わない」が Yuhitsu 美学。設定 UI は Phase 2)
  - [x] ツールバーから filename / status 表示も撤去(タブと重複していた)
  - [x] 設定変更は `settings.json` 直編集 + ウィンドウへのフォーカス復帰時に自動再読み込み
        (再起動不要、`window.focus` イベントで `loadSettings` を再実行して各 state を更新)
- [x] **ステータスバー(画面下部、設定で on/off)を仕込み**
  - [x] `workspace.statusbarVisible`(デフォルト `false`)+ migration、`settings.json` 編集 + focus で切替
  - [x] HTML 構造を配置:左にメッセージ、右に行数 / 文字数 / ワードカウント用の空スロット(`<span class="counter">` × 3)
  - [x] 行数 / 文字数 / ワードカウントは **Phase 2 で実装する仕込みのみ**(コメントで明示)。
        ワードカウントは Typst コンパイル後の本文字数(仕上り時)を想定
- [x] **設定ファイル(settings.json)を Yuhitsu 自身のタブで開ける**(2026-05-01 実装、設定 UI ができるまでの一次手段)
  - [x] Rust `get_settings_path` コマンド(`app_data_dir` を返す、未作成なら空 JSON を作成)
  - [x] フロントの `open-settings` コマンド(Ctrl+, / Settings アイコン、ツールバー右端)
  - [x] save() 内で settings.json への保存を検知したら自動 reloadSettings(focus 任せより確実)
  - [x] loadSettings の冒頭で `store.reload()`(Yuhitsu の `fs::write` は Tauri Store のキャッシュを無効化しないため必須)
- [x] **UI 文字列の i18n 化**(2026-05-01 氏指摘で追加 → 即日実装)
  - [x] 自前 `$lib/i18n/index.svelte.ts`($state ベースの `i18nState.locale` + `setLocale` + `t(key, params?)`、ライブラリ追加なし)
  - [x] 辞書 `ja.json` / `en.json`(command label / dialog / placeholder / splitter / project / preset / filter / status / preview / templateDialog / tab.untitled / app.name)
  - [x] `CommandDef.label` / `ToolbarPreset.label` を `labelKey` 化
  - [x] `+page.svelte` の全 UI 文字列(確認ダイアログ / ステータス / プレースホルダ / aria-label / タブ (無題) / プロジェクトビュー / iframe title / ファイル選択フィルタ)を `t()` 経由に統一
  - [x] `TemplateDialog.svelte` の `aria-label` も `t()` 化
  - [x] `onMount` + `reloadSettings` で `setLocale(resolveLocale(localeMode))` を呼び、`settings.json` の locale 変更が UI 全体にリアクティブ反映
- [ ] **ツールバー D&D 編集 UI**(設定 JSON は既に書き換え可能、GUI で並び替えるのは別タスク。`svelte-dnd-action` 採用予定)
- [ ] **キーバインド設定 UI**(`settings.keybindings` に override を保存する仕組みは整備済み、編集 UI のみ未実装)
- [ ] **フォーム型テンプレート簡素版**(`#show: template.with(...)` の引数を右ペインのフォームに展開)
- [ ] 設定読み込みエラーの可視化(JSON パースエラー等が UI に出ない問題、Phase 2 で改善)

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

注: 「フォーム型テンプレート」「GUI 挿入ボタン」は Sprint 3 で前倒し着手済みの場合、ここでは拡張・成熟化のみを担う。

- [ ] フォーム型テンプレート入力の成熟化(差別化ポイント #2、Sprint 3 簡素版を強化)
- [ ] GUI 挿入ボタンの成熟化(差別化ポイント #4、数式は MathLive (MIT) 等の手書き UI 検討)
- [ ] パッケージ管理 UI(Typst Universe 連携、ワンクリックでテンプレート取り込み)
- [ ] テンプレートギャラリー
- [ ] **内蔵 AI 機能の最小実装**(段落整え・補完・テンプレート穴埋め支援)
  - [ ] LLM プロバイダ抽象(Anthropic / OpenAI / ローカル LLM / VPN 内エンドポイント を等しく扱う)
  - [ ] API キーの安全な保存(`tauri-plugin-stronghold` または OS キーチェーン)
  - [ ] 差分プレビュー → ユーザ確認 → 適用の二段フロー
  - [ ] 非同期ジョブキュー(UI ブロックなし、ストリーミング応答対応)
  - [ ] **情報漏洩対策はユーザ環境構築に委ねる方針**(Yuhitsu 側で誤った安心感を与えない、抽象化に徹する。2026-04-28 氏合意)
- [ ] v0.2.0 リリース

---

## Phase 3: WYSIWYG-lite モード(差別化の本丸) + 外部エージェント連携

- [ ] AST ベース dual-rendering 設計書作成
- [ ] 見出し・強調・引用の記法非表示化(PoC)
- [ ] カーソル位置での記法表示切り替え
- [ ] 表・リスト等の WYSIWYG 化
- [ ] **Yuhitsu を MCP サーバとして公開**(Claude Cowork 等の外部エージェントから制御可能化)
  - [ ] `@modelcontextprotocol/sdk`(MIT)で Yuhitsu の操作を MCP ツールとして公開
  - [ ] read_document / apply_edit / get_diagnostics / compile などのツール定義
  - [ ] 外部からの編集にもユーザ確認フローを通す権限モデル
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

### 2026-04-25: Sprint 2 スコープ確定
- 実装順序:**(1) ファイル開閉 → (2) エディタ+LSP → (3) プレビュー → (4) PDF**(氏承認)
- **エディタ操作モード3種ビルトイン**(氏指示):OS 標準 / vim / emacs を標準装備、設定で切替
  - 採用ライブラリ:CodeMirror 6 デフォルト + `@replit/codemirror-vim` + `@replit/codemirror-emacs`(全て MIT、Apache-2.0 互換)
  - vim/emacs ユーザーを取り込みつつ、非エンジニアには OS 標準を提供する両取り戦略

### 2026-04-26: Sprint 2 順序見直し + AI エージェント連携の長期方針
- **Sprint 2 順序を組み替え**(氏承認):**(1) ファイル開閉 → (2) エディタ + Typst syntax → (3) ライブプレビュー → (4) PDF → (5) 操作モード3種 → (6) LSP 統合**
  - 理由:typst.app 風の 2 ペイン UI に早く近づけて視覚的な完成度を先に上げるため、ライブプレビューを LSP より先行させる。LSP は実装が一番重く、優先度最後で問題なし
- **AI エージェント連携を長期方針として確立**(氏承認):
  - プロトコルは **MCP(Model Context Protocol)** に乗る前提
  - **Phase 1**:実装はしないが、構造として下準備(エディタ操作 API レイヤ、状態シリアライズ、設定領域分離、`@codemirror/merge` 評価)
  - **Phase 2**:内蔵 AI 機能の最小実装(LLM プロバイダ抽象、API キー保護、diff プレビュー、非同期ジョブ)
  - **Phase 3**:Yuhitsu を MCP サーバ化、外部エージェント(Claude Cowork 等)から制御可能に
  - 避ける:特定ベンダー SDK の直 import / 同期前提 UI / 暗黙の編集適用 / AI 必須化 / 平文 API キー保存 / GPL 系ライブラリ混入
- **Sprint 2 (1)〜(2) の実装完了状況**:
  - ファイル開閉(`c7be3c4`)
  - 終了確認バグ修正(`38b3017`、`core:window:allow-destroy` 追加)
  - CodeMirror 6 素導入(`977c59b`)
  - Typst 構文ハイライト統合(`2d576d6`、`codemirror-lang-typst` v0.4.0 + `vite-plugin-wasm`)

### 2026-04-29: タブ機能 + プロジェクトビュー + テキストファイル対応
- 競合 Typstudio に「ExplorerTree / SidePanel」があるのを契機に、Yuhitsu にも以下を一括追加:プロジェクトビュー(サイドバー)、タブ機能、テキストファイル全般対応
- **プロジェクトビュー**:Rust `list_directory`(全ファイル表示・ノイズフォルダ除外)、`ProjectTree.svelte`(Svelte 5 self-import で再帰描画)、フォルダ選択 + 永続化 + 起動時自動復元、サイドバー on/off + 幅リサイズ、ツリーから `.typ` はタブで開く・PDF/画像はバイナリは OS デフォルトに流す
- **タブ機能**:`{ id, path, content, dirty, cursorAnchor, cursorHead, scrollTop }` を per-tab で管理、`+` ボタン + Ctrl+T で新規、各タブに ✕ + Ctrl+W、ファイル open は同じパスがあれば既存 active に・空タブを再利用・なければ新規追加。タブ切替時に位置復元
- **テキストファイル対応**:`.md` / `.txt` / `.csv` / `.bib` / `.yaml` / `.json` / `.toml` / `.html` / `.rs` 等をタブで編集可能。`Editor.svelte` に `languageMode` Compartment を追加し、Typst 以外は plain。LSP / preview / PDF は Typst のみ動作
- **トラブル / 解決した workaround**:
  - 起動時 LSP rootUri を `file:///` にすると tinymist が `entry is not in any set root directory` でフォールバック → 親ディレクトリに戻し、preview / PDF 側だけ `--root /`(filesystem root)に拡張(絶対パス画像が読める)
  - `codemirror-lang-typst` v0.4.0 の WASM パーサが「Typst 言語拡張が有効な状態で大規模 replace edit(タブ切替の全置換)」を処理できず `Unreachable code should not be executed` で落ちる → Editor.svelte の `$effect` を 3 段階 dispatch(lang Compartment 一旦外す → doc 全置換 → 再有効化)に変更
  - フロント側ログを直接見るために `dev_log` Tauri command + `$lib/dev-log.ts` を新設(JS → Rust stderr)。`pnpm tauri dev` のログから `[js] ...` 行を読めるので、DevTools を開かずデバッグ可能に

### 2026-04-28: Sprint 3 (3) — GUI 挿入ボタン拡張セット + ツールバー駆動化
- **氏方針**:ツールバーはユーザがメニュー構造を柔軟に変更できる設計に。デフォルトとプリセットを用意した上で自分でいじれること。ショートカットも設定可能。D&D 編集 UI は後回しで OK
- 実装:
  - `$lib/commands.ts` を新設し、全コマンド(ファイル系 + 挿入系 19 種)を ID 中心のカタログに集約。`{ id, label, buttonText, buttonClass, defaultKey, needsEditor, run }` を共通形式とし、ツールバー / キーバインド / 設定永続化 / 将来の MCP から同じ ID で参照
  - 挿入コマンドを `editor-commands.ts` に追加実装(数式 / インラインコード = 既存 `toggleInlineWrap` を `$` `` ` `` で再利用、コードブロック / リンク / 脚注 / 引用 / 表 / 画像)
  - 表は `insertTable(view, { columns?, rows? })` で引数化(MVP は 2x2、後で「列数指定モーダル」を増設しやすい形)
  - ツールバーを `toolbarItems: ToolbarItem[]` の配列駆動に書き換え、divider も `"divider"` という ID として扱う
  - キーバインドは `onKeydown` がカタログを順次照合する一括方式に(`effectiveKey = override ?? defaultKey`、`matchKey` で `Mod-Shift-b` 形式を判定)。`needsEditor` のコマンドは view 不在時にパススルーするので vim Normal モードと共存
  - プリセット 3 種(標準 / ミニマル / 論文寄り)を `TOOLBAR_PRESETS` で内蔵
  - `settings.ts` に `toolbar.items` と `keybindings` 領域を追加。`saveToolbarItems` / `saveKeybindings` を export
  - 画像挿入はファイル選択ダイアログ(`@tauri-apps/plugin-dialog` の open + 拡張子フィルタ)と、`getCurrentWebview().onDragDropEvent` 経由のウィンドウ D&D の両入り口
  - 画像パスは現在編集中の `.typ` のディレクトリを基準に相対パス化(同階層・サブ階層のみ。それ以外は絶対パスを `/` 区切りで)
- 残:D&D 編集 UI(`svelte-dnd-action` 採用予定)/ キーバインド設定 UI / 起動時テンプレート選択ダイアログ / フォーム型テンプレート簡素版

### 2026-04-28: Sprint 3 着手 — GUI 挿入ボタン最小セット完了
- **Sprint 3 (2)** GUI 挿入ボタン最小セットを実装(太字・斜体・見出し H1〜H3・箇条書き・番号付き)
  - `$lib/editor-commands.ts` を新設し、`EditorView` を引数にとる純粋関数として `toggleBold` / `toggleItalic` / `applyHeading` / `toggleBulletList` / `toggleNumberedList` を定義(UI 非依存)
  - これは Phase 1 の下準備「エディタ操作 API レイヤを Svelte UI / Tauri command / 将来の MCP ハンドラから共通で呼べる形に整理」の第一歩を兼ねる
  - Editor.svelte は `onReady(view)` / `onTeardown()` で view を親に公開、+page.svelte 側がコマンドを呼ぶ
  - Ctrl+B / Ctrl+I は editorView が無いと preventDefault しない(vim Normal モードでの ページ送り等と共存)
  - 行頭マーカーは「同種ならトグル除去 / 別種なら置換 / なしなら付与」、インデントを保持
- 残り Sprint 3 項目:起動時テンプレート選択ダイアログ / GUI 挿入ボタン拡張セット / フォーム型テンプレート簡素版

### 2026-04-28: Sprint 2 完了 + Yuhitsu の長期ポジション確定
- **Sprint 2 (3)〜(6) すべて MVP として動作**:
  - ライブプレビュー(`e1881c5`、tinymist preview の iframe 方式)
  - PDF エクスポート(`af1cf8f`、tinymist compile 経由)
  - 操作モード3種(`d241c6a`、`@replit/codemirror-{vim,emacs}` + tauri-plugin-store)
  - tinymist LSP 統合(`72d4f4d`、`@codemirror/lsp-client` v6.2.3、補完 / 診断 / hover 動作確認済み)
- **Yuhitsu の長期ポジションを確定**(氏合意):
  - 最終ゴールは **「AI エージェンティック前提のローカル GUI ドキュメントエディタ」**。氏自身が Claude Code でこの開発をしている動き方が、そのまま Yuhitsu の理想的なユーザ体験。
  - 空白座標は「OSS × ローカル × GUI × 実用 × エージェント連携可」で、現状の市場に同等品なし。
  - **情報漏洩対策はユーザ環境構築側に委ねる**。Yuhitsu は LLM プロバイダを抽象化するに留め、特定の保護機構を凝らない。コンプライアンス厳格な現場でも、ユーザがローカル LLM や VPN 内エンドポイントを選べば採用できる、という設計。
  - **実装順序合意**:Phase 1 末で Sprint 3(起動時テンプレート選択 + GUI 挿入ボタン最小セット + フォーム型テンプレート簡素版)を前倒し → Phase 2 で内蔵 AI 機能 → Phase 3 で WYSIWYG-lite + MCP サーバ化 + Universe 連携。

### 2026-05-01: テーマ機能(自動 / ライト / ダーク)を追加
- 氏要望:**Ubuntu のダーク設定で Yuhitsu もダーク見えしているが、それは Yuhitsu 自体のテーマか OS 追従か?** を契機にテーマ対応を実装(Phase 2 予定の前倒し)
- **判断**:公開時に複数テーマを選べる前提なので、まず CSS 変数化で素地を作り、確認用にライト固定セットを足す → 氏が「DevTools 経由は嫌、UI 欲しい」で auto/light/dark 切替セレクトまで一気に実装
- 実装:
  - 全 UI 色を `app.html` の `:root` に CSS 変数として集約(背景 6 / ボーダー 2 / 文字 7 / アクセント 3 / ステータス 3 / Syntax 9、計 27 種)
  - `:root[data-theme="light"]` でライトテーマを定義(One Light 風配色、`color-scheme: light`)
  - `+page.svelte` / `Editor.svelte`(`<style>` + CodeMirror `EditorView.theme` + `HighlightStyle`)/ `ProjectTree.svelte` の全色を `var(...)` 化
  - `settings.ts` に `AppearanceSettings` 領域を新設、`saveTheme` を export、Tauri Store で永続化
  - ツールバーに「テーマ: 自動 / ライト / ダーク」セレクト(操作モードの隣)。"自動" は `prefers-color-scheme` に追従、`matchMedia` で OS 側変更も即反映
- 副次効果:Phase 2 で正式テーマ UI を作る時はプリセット(solarized / nord / gruvbox 等)を `[data-theme="..."]` セットとして増やすだけで対応できる構造に

### 2026-05-01: UI 文字列の i18n 化(自前 i18n.ts + ja/en 辞書)
- **氏指摘で発覚**:locale を ja に切り替えても UI 全体が日本語のまま → UI 文字列がハードコードされたままで i18n 機構が無いことが判明
- **方針(再確認)**:UI 文字最小化方針(hover / 確認 / ステータス / プレースホルダのみ)に従い、辞書ルックアップ + `{key}` 差し込みで足りる範囲に絞る → 自前 `i18n.ts` で十分、ライブラリ追加なし
- 実装:
  - `$lib/i18n/index.svelte.ts`:$state(locale) + setLocale + t(key, params?)、`.svelte.ts` 拡張子で `$state` を共有モジュールから export
  - 辞書 `ja.json` / `en.json`:約 70 キー
  - `CommandDef.label` → `labelKey`、表示時に `t(def.labelKey)` を呼ぶ。`ToolbarPreset.label` も同様
  - 確認ダイアログ(`ask`)・`setStatus`・プレースホルダ HTML・splitter aria-label・タブ「(無題)」・iframe title・ファイル選択フィルタなどを全部 `t()` 化
  - `setLocale(resolveLocale(localeMode))` を `onMount` + `reloadSettings` で呼び、settings.json での locale 変更が UI 全体にリアクティブ反映
- 確認:タブ「(無題) ↔ (Untitled)」/ テンプレダイアログのカード / プレビュープレースホルダの切替を確認済み
- 残:**テンプレ本体の用語**(例:「業務報告書」テンプレ内の `[ここに実施内容を箇条書きで記載]`)はテンプレファイル自体の i18n(`ja.typ` / `en.typ`)で対応済み

### 2026-05-01: 設定ファイルを Yuhitsu で開く + UI 文字列 i18n の認識合わせ
- **氏要望**:設定確認のたびに別エディタで開いて再起動するのが面倒。Yuhitsu 自身で開けるように
- 実装:
  - ツールバー右端に Settings アイコン + `open-settings` コマンド (Ctrl+,)
  - Rust `get_settings_path` で `app_data_dir()` を返す(`tauri-plugin-store` v2 が `app_data_dir` に保存するため、間違えて `app_config_dir` を使うと別ディレクトリに空ファイルを作る落とし穴あり)
  - save() 内で「保存先が settings.json と一致したら自動 reloadSettings」(focus イベント任せだとタブ切替で発火せず取りこぼす)
  - loadSettings の冒頭で `store.reload()`:Yuhitsu は `fs::write` で素朴に書き込むので、Tauri Store のメモリキャッシュが古いまま。reload しないと外部・内部どちらの編集も反映されない
- **副次:i18n の現状確認**:氏が「locale 設定したのに UI が日本語のまま」と指摘 → UI 文字列はテンプレカード以外すべて日本語ハードコードであることを認識合わせ。次タスクとして i18n.ts 実装を予定

### 2026-05-01: 起動時テンプレ選択ダイアログ + 多言語テンプレート + 参考文献挿入
- **氏方針**(議論で確立):
  - 表示タイミングは **D 案**(初回起動で自動表示 + 専用コマンド)。毎回起動時に出すのはうざい
  - テンプレ本体は **多言語ファイル管理**(meta.json + ja.typ + en.typ)。UI 文字列のフル i18n は別タスク
  - **用紙サイズは locale 非依存**(`document.paperSize` で別管理、auto は locale から推測)
  - 共通スタイルは初期は **完全独立**(各 locale ファイルに直書き、テンプレが増えたら共通化)
  - 参考文献は **Hayagriva (.yml) 推奨、BibTeX (.bib) も対応**(Typst が両方ネイティブサポート)
- 実装:
  - 初期セット 6 種(空 / 業務報告書 / 技術報告書 / 議事録 / 簡易レター / スライド)、各 ja/en 2 言語
  - 技術報告書ひな形は「概要 / はじめに / 方法 / 結果 / 考察 / 結論 / 参考文献」構成
  - スライドは素のページ分割版(`presentation-16-9`、touying / polylux 統合は Phase 2)
  - テンプレダイアログはカードグリッド(アイコン大 + タイトル小 + hover で説明)、ESC / 背景クリックでキャンセル
  - 参考文献挿入は画像挿入と同じく「ファイル選択 → 相対パス化 → 末尾挿入」フロー
- 残:設定 UI(Phase 2)、フォーム型テンプレ簡素版(Sprint 3 続き)、ツールバー D&D 編集 UI / キーバインド設定 UI

### 2026-05-01: 無題タブを閉じる時の確認ダイアログを「保存しますか?」形式に
- **氏指摘**:「破棄して閉じますか? はい/いいえ」は否定形が混ざって意図が取りづらく、「名前をつけて保存しますか?」の方が自然
- 実装:
  - `closeTab` の確認メッセージを変更:無題タブは「名前をつけて保存しますか?」、既存ファイルは「保存しますか?」
  - okLabel="保存" / cancelLabel="保存しない"
  - 「保存」→ 対象タブを active にしてから save/saveAs → 成功で閉じる(保存ダイアログでキャンセルしたらタブ残す)
  - 「保存しない」→ 従来通り破棄して閉じる
  - `save` / `saveAs` を `Promise<boolean>` 化(成功 / キャンセル / エラー判定可能に)。既存呼び出し箇所は戻り値を使わないので無害
- 関連項目:ウィンドウ全体を閉じる時の「終了してよろしいですか?」も同種の改善余地あり(複数 dirty タブの一括処理が必要なため、Phase 2 で対応)

### 2026-05-01: UI 文字最小化方針 + ステータスバー仕込み + WYSIWYG-lite の方向性確認
- **氏方針(Yuhitsu 美学として確立)**:UI に文字を使うべきじゃない、hover ヒント程度にしたい。設定系(操作モード・テーマ)は編集中に頻繁に変えるものではないのでツールバーから外す
- 実装:
  - ツールバーから操作モード / テーマ セレクトと filename / status 表示を撤去 → ツールバーは編集アクション専用に
  - 設定変更ルートは「`settings.json` 直編集 → Yuhitsu にフォーカス復帰で自動再読み込み」(`window.focus` イベントで `loadSettings` 再実行、再起動不要)
  - 設定 UI 画面は **Phase 2 で本格実装**(それまでは直編集が一次手段)
- **i18n 範囲が縮んだ**:残るのは hover ヒント / ステータス / ダイアログ / テンプレカード / プレースホルダ のみ → 自前 `i18n.ts` で十分、ライブラリ追加なし
- **ステータスバー(画面下部、設定で on/off、デフォルト off)を仕込み**:
  - 「設定 UI に出すほどでもないが、行数 / 文字数 / ワードカウントは見たくなる」という氏の予感に対応
  - 表示は HTML 構造とスタイルだけ用意、行数 / 文字数 / ワードカウントは **Phase 2 で実装する空スロット**
  - ワードカウントは Typst コンパイル後の本文字数(仕上り時)を出す前提
- **WYSIWYG-lite モードの方向性確認**(氏の質問:Obsidian 的 UI をプレビュー側に実装できるか?):
  - 結論:**プレビュー側(SVG 出力)では実装不可、エディタ側に AST ベース Decoration で実装する**(これが Phase 3 の本丸)
  - Obsidian / Typora は CodeMirror 6 の Decoration API でエディタ側に被せる方式
  - tinymist preview の SVG 出力は contentEditable 化できない(SVG → Typst の逆変換不能)
  - Typst は Markdown より文法が複雑で、装飾で隠せるノード(見出し / 強調 / リスト / リンク)、プレースホルダ代用ノード(数式 / 表)、生コードのまま見せるノード(`#let` / `#import` / 関数定義)の 3 カテゴリに分かれる → Phase 3 の「設計書作成」が一番重い作業
