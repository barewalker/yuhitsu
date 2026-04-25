# Phase 0 — Tinymist 調査レポート

調査日: 2026-04-23
対象: [Myriad-Dreamin/tinymist](https://github.com/Myriad-Dreamin/tinymist)
調査用 clone: `~/Projects/yuhitsu-refs/tinymist/` (shallow, depth=50)

## 結論サマリ

| 観点 | 判定 |
|---|---|
| ライセンス互換性(Apache-2.0 Yuhitsu) | ✅ **完全互換**(Apache-2.0) |
| メンテナンス状況 | ✅ **極めて活発**(直近コミット 2026-04-06) |
| Typst 追従性 | ✅ 最新 patch 追従(v0.14.6 派生を同梱) |
| LSP 完成度 | ✅ production-ready、VSCode / neovim / emacs / helix / zed 実績 |
| preview 機構 | ✅ WebSocket + incremental SVG(流用価値高) |

→ **Yuhitsu のエディタ知能はすべて tinymist に寄せる。自前実装しない。**

## 1. ライセンス

- `LICENSE` は **Apache License 2.0**(Copyright 2023-2025 Myriad Dreamin, Nathan Varner)。
- 同梱の tinymist-preview は元 typst-preview(Enter-tainer, **MIT**)— Apache-2.0 と互換。

### 互換性判定

| 統合方式 | Yuhitsu (Apache-2.0) への取込可否 |
|---|---|
| (A) `tinymist lsp` バイナリを subprocess として spawn | ✅ 可(バイナリ単位で LICENSE 表記 + NOTICE 添付) |
| (B) `tinymist-query` 等を crate として static link | ✅ 可(Apache-2.0 同士、NOTICE 表記のみで良) |

両方式とも Apache-2.0 の要件(NOTICE 配布、変更告知)を満たせば問題なし。

## 2. リポジトリ構成

```
tinymist/
├── crates/                    # Rust workspace, 26 crates
│   ├── tinymist/              # LSP サーバ本体(cdylib + rlib)
│   ├── tinymist-cli/          # バイナリ `tinymist`(lsp/preview/compile subcmd)
│   ├── tinymist-query/        # 補完/ホバー/定義等の分析 API
│   ├── tinymist-analysis/     # 型推論・診断・シンボル索引
│   ├── tinymist-render/       # SVG/PDF 出力
│   ├── tinymist-preview/      # Preview サーバ(WebSocket、旧 typst-preview)
│   ├── sync-ls/               # LSP プロトコル実装
│   ├── tinymist-project/      # プロジェクト/ワールド管理
│   ├── tinymist-vfs/          # 仮想 FS
│   ├── tinymist-world/        # Typst コンパイルコンテキスト
│   ├── tinymist-lint/
│   ├── tinymist-debug/
│   ├── tinymist-dap/          # Debug Adapter Protocol
│   ├── tinymist-derive/
│   ├── tinymist-l10n/
│   └── tinymist-task/
├── editors/                   # エディタ側クライアント
│   ├── vscode/                # TypeScript, vscode-languageclient 使用
│   ├── neovim/
│   ├── emacs/
│   ├── helix/
│   ├── sublime-text/
│   └── zed/
├── docs/
├── tests/
└── LICENSE                    # Apache-2.0
```

Rust toolchain は 1.91 想定。Typst は `Myriad-Dreamin/typst` fork(`tinymist/v0.14.6-rc2` タグ)を git patch で差し込んでおり、公式 typst の pub 範囲を拡張して内部 API を expose している。

## 3. library として組み込み可能か

### crates.io 公開状況

主要 crate(`tinymist-query`, `-analysis`, `-render`, `-preview`, `sync-ls`)は **crates.io には publish されていない**。workspace 内 path dependency として扱われる。

### 公開 API

- `tinymist-query`: README に "an analyzing library for Typst" と明記。
  - `CompletionRequest::request()`, `HoverRequest::request()`, `GotoDefinitionRequest::request()` 等
  - `LocalContext` を介して同期呼び出し可能。
- `tinymist-analysis`: `SharedContext` ベースの型推論・診断。
- `tinymist-render`: SVG/PDF レンダリング関数群。

LSP を経由せず関数呼び出しで補完・診断・hover を取り出すことは **技術的には可能**。ただし `tinymist-world`, `tinymist-analysis` への推移的依存を丸ごと取り込むことになる。

### Typst バージョン整合

- 同梱 typst fork のタグ: `tinymist/v0.14.6-rc2`
- Tinymist 本体: **v0.14.16**(2026-04-03 リリース)
- 偶数 patch が stable、奇数 patch が nightly

## 4. LSP プロセスとして使う場合

### 起動

```
tinymist lsp [--mirror <logfile>]
```

VSCode 側の実装(`editors/vscode/src/lsp.system.ts`)は `{ command: "tinymist", args: ["lsp"] }` で spawn。標準的な stdio JSON-RPC。

### サポートする LSP メソッド(抜粋)

- **textDocument**:
  - `completion`(トリガ: `#`, `(`, `<`, `,`, `.`, `:`, `/`, `"`, `@`)
  - `hover` / `definition` / `references`
  - `documentSymbol` / `workspaceSymbol`
  - `formatting` / `rangeFormatting`(`typstyle` または `typstfmt`)
  - `foldingRange` / `documentHighlight`
  - `documentLink`(画像・参考文献パス)
  - `documentColor`(カラーピッカー)
  - `rename` / `prepareRename`
  - `inlayHint`
  - `codeAction`(見出しレベル変更、数式整形など)
  - `codeLens`(export ボタン)
  - `semanticTokens`(incremental)
  - `signatureHelp`
  - `selectionRange`
- **workspace**:
  - `executeCommand`(テンプレートギャラリー、プロファイリング等のカスタム)
  - `willRenameFiles`(参照更新)

Yuhitsu が必要とする「補完・診断・hover・定義ジャンプ・フォーマット」は**全て標準装備**。

### 設定

- サーバ capability は `initialize` レスポンスで返却。
- ワークスペース単位の設定は `config.toml` / `.typst.toml`。
- クライアント側設定は `tinymist.*` namespace で `workspace/configuration` 経由で取得。

### Tauri / CodeMirror 統合事例

README には直接記載なし。ただし:
- VSCode 拡張は `vscode-languageclient`(MIT)を使用 — Tauri の WebView 上でも同等の JS ベースクライアントを流用可能。
- Tauri では Rust 側で `tinymist lsp` を spawn し、stdio を Tauri event bridge 経由でフロントに渡す構成が自然。

## 5. Preview 機構

- サブコマンド: `tinymist preview`(feature flag `[preview]`)。
- crate: `tinymist-preview`(元 typst-preview、Enter-tainer 作、MIT)。
- プロトコル: **WebSocket**、デフォルトポート **23625**。
  - 制御平面(control-plane): 編集指示・設定変更
  - データ平面(data-plane): レンダリング結果 push
- 出力: **Incremental SVG**(reflexo-typst 使用)。partial rendering フラグ `enable_partial_rendering` で変更部分のみ送信。
- 2 モード: Document / Slide。
- フロント HTML は tinymist-assets に bundle。Tauri の WebView で `localhost:23625` に接続するか、bundle 済み HTML を抜き出してカスタム配置が可能。

Yuhitsu の Tauri WebView 内で **そのまま再利用可能**。ラスタ方式(Typstudio の PNG+Base64)に比べてズーム耐性とレスポンスが段違いに優位。

## 6. アクティビティ

### コミット(最新 10 件)

```
fe65980 2026-04-06  feat(ci): ensure lockfiles are updated (#2476)
9c6ff7a            fix: align output path pattern resolution with documented behavior (#2473)
c77e716            build: bump version to 0.14.16 (#2474)
79b6200            build: bump version to 0.14.16-rc1 (#2471)
5051cd8            fix: set default formatter mode to `typstyle` for non-vscode clients (#2468)
729e91d            dev: propose compiler settings docs (#2465)
83818f8            fix: use explicit label syntax for incompatible cite keys (#2464)
929d61e            dev: archive fix-math-module-field-completion (#2463)
23cc149            dev: propose tinymist rust tool skill (#2455)
8552a0d            dev: propose configuration-wide client flag (#2390) (#2459)
```

### リリース

```
v0.14.16        (stable, 2026-04-03)
v0.14.16-rc1
v0.14.14
v0.14.14-rc1
v0.14.12
```

週〜隔週ペースで patch リリース。奇数 patch = nightly、偶数 patch = stable。

## 7. Yuhitsu にとっての推奨統合方法

### (A) `tinymist lsp` を subprocess として起動 ★ 推奨 ★

- **構成**: Tauri(Rust)が `tinymist lsp` を spawn → stdio JSON-RPC → CodeMirror 6 に JS LSP クライアント(独自 or `vscode-languageclient` の brower 派生)で接続。
- **メリット**:
  - tinymist の更新追従が単純(バイナリ差し替えのみ)。
  - VSCode 拡張と同じプロトコルで、VSCode で検証済みの挙動をそのまま得られる。
  - 責任分離: エディタ知能と UI を切り離せるため Yuhitsu 側の複雑度が下がる。
  - preview も同じバイナリの `tinymist preview` で起動可能。
- **デメリット**:
  - tinymist バイナリを bundle する必要(20〜30 MB)。
  - プロセス境界の latency(実測数 ms、UX 上はほぼ無視可)。

### (B) `tinymist-query` を library として static link

- **構成**: Yuhitsu の Rust 側で `use tinymist_query::*` し、Tauri IPC で補完要求を受けて直接呼ぶ。
- **メリット**:
  - 最小 latency、バイナリ 1 本に収まる。
  - Rust 側で密結合カスタマイズが効く。
- **デメリット**:
  - **crates.io 未公開 → git dependency 固定(tag)必須**、API が安定公開されていないため tinymist 更新のたびに型追従コストが発生。
  - typst の Myriad-Dreamin fork を引き込むため、Yuhitsu の Cargo.lock が公式 typst と非整合になる。
  - fork の sync 労力が継続的に発生。

### (C) preview のみ tinymist を使い、補完・診断は自前実装

- 補完品質が実用水準に届かない(Typst は型・スコープが非自明で、AST 走査だけでは限界)。Yuhitsu の差別化は UX と日本語にあるため、エディタ知能は自作せず tinymist に寄せるべき。**却下**。

## 8. 所感

Tinymist は Apache-2.0 かつ極めて活発で、Yuhitsu の必要機能を **既にすべて実装している**。Yuhitsu 側は「LSP クライアント + preview クライアント + 日本語 UI / テンプレ / WYSIWYG-lite」に開発資源を集中でき、エディタ知能の車輪の再発明を避けられる。統合は **(A) subprocess 方式** が第一選択。(B) は当面採らず、将来「tinymist-query が crates.io 公開され API 安定化」した時点で再評価の余地あり。Yuhitsu 本体の typst crate はレンダリング目的などで別途必要になるが、tinymist と独立に公式 typst を使えばよく、fork 同梱を自前で運用する必要はない。
