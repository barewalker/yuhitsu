# Phase 0 — 路線判断レポート

作成日: 2026-04-23
前提資料:
- [phase0-typstudio-analysis.md](./phase0-typstudio-analysis.md)
- [phase0-tinymist-analysis.md](./phase0-tinymist-analysis.md)

## 結論

**ゼロスタート路線を採用する。Tinymist を LSP サブプロセスとして統合する。Typstudio は fork せず、参考資料として凍結する。**

本決定は以下の事実によりほぼ一意に定まった:

1. **Typstudio は GPLv3**。Apache-2.0 を採用する Yuhitsu 本体に組み込めない。
2. **Typstudio 作者が公式に「メンテ放棄・近く archive 予定・代替は Tinymist」と宣言**(README 冒頭、2025-04-11 時点)。
3. **Tinymist は Apache-2.0 で極めて活発**(直近コミット 2026-04-06、v0.14.16 リリース)、Yuhitsu が必要とする LSP 機能を production-ready な形で全て具備。

判断を要する分岐点のため記録する(CLAUDE.md「作業進行の原則」に従い、最終承認は氏に仰ぐ)。

## 1. 路線比較

### Typstudio fork 路線

| 項目 | 評価 |
|---|---|
| ライセンス | ❌ GPLv3 → Apache-2.0 非互換、選択不能 |
| Typst 追従 | ❌ v0.11 固定、最新 v0.14 系まで 2 世代遅れ |
| LSP 機能 | ❌ 不在、自前で tinymist 連携を足す必要 |
| Preview | △ PNG+Base64 のラスタ方式(ズーム耐性・応答性で劣位) |
| 作者サポート | ❌ メンテ放棄宣言 |
| 既存 UI の日本語親和性 | ❌ i18n 基盤なし、Monaco は IME 動作に課題 |
| 初期立ち上がり速度 | △ シェル構造は流用価値あるが、ライセンスで不可 |

### ゼロスタート + Tinymist 統合路線

| 項目 | 評価 |
|---|---|
| ライセンス | ✅ 全て Apache-2.0 / MIT で統一 |
| Typst 追従 | ✅ tinymist が追従、Yuhitsu は追う必要最小 |
| LSP 機能 | ✅ completion/hover/definition/format/... 全て具備 |
| Preview | ✅ Incremental SVG over WebSocket |
| 作者サポート | ✅ 活発(週〜隔週リリース) |
| 日本語親和性 | ✅ CodeMirror 6 は IME 動作に定評、i18n は自前で最初から設計できる |
| 初期立ち上がり速度 | △ シェル構造を一から組む必要あり、ただし参考実装は十分にある |

## 2. メリット・デメリット

### fork 路線のメリット / デメリット

**メリット**(仮にライセンスが許した場合)
- Tauri シェル・IPC 境界・Monaco 統合の雛形が手に入る。
- Typst World 実装(`project/world.rs`)のパターンが即使える。

**デメリット**(現実)
- **GPLv3 により採用不能**。以下は仮定上の話。
- typst v0.11 → v0.14 追従だけで 1〜2 週間、しかも comemo 等の周辺 crate も巻き込む。
- Monaco → CodeMirror 6 置換を結局行う必要がある(和文 IME と電力消費の都合)。
- tinymist LSP 連携は新規実装(2〜3 週間)で、結局ゼロスタートと同じ工程。
- upstream がメンテ停止しているため、セキュリティと Typst 追従を丸ごと自責で負う。

### ゼロスタート路線のメリット / デメリット

**メリット**
- Apache-2.0 で全エコシステムが整合。(redacted internal repo)との法的分離も綺麗に成立する。
- 最新 Tauri 2 系・CodeMirror 6・最新 typst / tinymist を最初から採用できる。
- 日本語ファーストの前提で i18n 基盤・和文組版ヘルパー・テンプレ構造を最初から設計できる(後付けでは歪む)。
- WYSIWYG-lite モードのための AST-driven UI を、他方針に縛られず一貫して設計できる。

**デメリット**
- Tauri シェル + ファイル IO + ウィンドウ管理 + UI レイアウトをゼロから組む必要。
- LSP クライアント(CodeMirror 6 側)の実装が必要 — ただし既存実装(`vscode-languageclient` browser 派生、`@open-rpc/client-js` 等)を流用可能。

## 3. 工数見積もり

「v0.1.0 alpha リリース相当の MVP 完成まで」の所要を粗く見積もる。CLAUDE.md の Phase 1 機能リスト(Tauri シェル、LSP 統合、preview、PDF export、日本語テンプレ 5 本、フォント同梱、i18n)を対象にする。

| 工程 | fork 路線(仮定) | ゼロスタート路線 |
|---|---|---|
| ビルド環境の復活 / 立上げ | 3〜5 日(time 等の復旧) | 1〜2 日(Tauri init) |
| Tauri シェル・ファイル IO | 流用可(0.5 週) | 1〜1.5 週 |
| Monaco → CodeMirror 6 置換 | 1〜2 週 | — (最初から CM6) |
| typst crate v0.11→v0.14 追従 | 1〜2 週 | — (最初から v0.14) |
| Tinymist LSP 連携 | 2〜3 週 | 2〜3 週 |
| Preview(SVG over WebSocket) | 1〜1.5 週(PNG 廃止して切替) | 1〜1.5 週 |
| PDF エクスポート | 3〜5 日 | 3〜5 日 |
| 日本語 UI / i18n 基盤 | 1〜2 週(後付けで歪む) | 1 週(最初から組み込み) |
| 日本語テンプレ 5 本 + Harano Aji 同梱 | 2 週 | 2 週 |
| 総合整形・バグ修正・α リリース準備 | 1 週 | 1 週 |
| **合計(営業日ベース)** | **約 13〜17 週** | **約 11〜14 週** |

fork 路線は仮定上でもゼロスタートより遅い(Monaco 置換と Typst 追従の二重コストが効く)。ライセンスを度外視しても合理性が無く、ライセンスを考慮すれば論外。

## 4. 採用アーキテクチャ(暫定)

```
┌──────────────────────────────────────────────────────────┐
│  Yuhitsu (Tauri 2, Apache-2.0)                          │
│                                                          │
│  ┌─────────────── Frontend (WebView) ───────────────┐   │
│  │  - CodeMirror 6 (editor)                         │   │
│  │  - LSP client (stdio ブリッジ経由)                │   │
│  │  - Preview pane (SVG over WebSocket)             │   │
│  │  - i18n (ja first)                               │   │
│  │  - テンプレフォーム UI(Phase 2)                   │   │
│  └──────────────────┬───────────────────────────────┘   │
│                     │ Tauri IPC                          │
│  ┌──────────────────┴───────────────────────────────┐   │
│  │  Backend (Rust)                                  │   │
│  │  - ファイル IO / プロジェクト管理                  │   │
│  │  - typst crate (PDF export 用、公式 v0.14 系)    │   │
│  │  - tinymist 子プロセス管理                       │   │
│  │         ├── `tinymist lsp` (stdio)               │   │
│  │         └── `tinymist preview` (WebSocket)       │   │
│  │  - Harano Aji フォント bundle                     │   │
│  └──────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────┘
```

- エディタ知能(補完/診断/hover/定義/format)は **すべて tinymist LSP に委譲**。Yuhitsu は自前で書かない。
- Preview は tinymist preview の WebSocket を流用。Typstudio 方式の PNG+Base64 は採らない。
- typst crate は PDF export 等の用途で Yuhitsu 本体にも直接リンクする(tinymist を介さない独立経路)。公式 typst を使い、tinymist 同梱の fork は引き込まない。

## 5. 残るリスクと未確認項目

- [ ] Tauri 2.x + CodeMirror 6 + LSP over stdio のプロトタイプ未検証(Phase 1 頭で確認)。
- [ ] tinymist バイナリのクロスプラットフォーム bundle サイズ、署名配布手順(winget、GitHub Releases)。
- [ ] tinymist が前提とする typst fork と、Yuhitsu が直接使う公式 typst の version skew がどこまで許容されるか(PDF export 品質への影響)。
- [ ] Harano Aji の初回 DL 方式 vs 同梱方式のトレードオフ(インストーラサイズ、オフライン起動要件)。

上記は Phase 1 冒頭で PoC を組んで潰す(別途 PROGRESS.md のバックログに反映)。

## 6. 氏の判断を仰ぐ点

1. **本決定(ゼロスタート + tinymist LSP 統合)でよいか**。
2. **Tauri は 2.x を前提でよいか**(1.x 系は採らない)。
3. **typst crate は公式(typst/typst)を使い、Myriad-Dreamin fork は引かない方針でよいか**。
4. **`yuhitsu-refs/typstudio` はディスク上に残すが、GPLv3 コードを Yuhitsu 本体に持ち込まない運用でよいか**(ルールをこのリポに明文化する価値あり)。

承認が得られ次第、Phase 1 の環境準備(Tauri 2 init、tinymist bundle 方法の PoC)に進む。
