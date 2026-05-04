# Yuhitsu (右筆)

**A local desktop GUI editor for [Typst](https://typst.app), with first-class Japanese typography support.**

> 🇯🇵 [日本語版はこちら](./README.md) / Japanese version

> 🚧 **Currently in development toward v0.1.0-alpha.** Early feedback is welcome, but not recommended for production use yet.

---

## What is this?

Yuhitsu is a desktop GUI editor for the [Typst](https://typst.app) typesetting system. It targets users who would benefit from Typst's quality but find IDE-style tools (VS Code + Tinymist) intimidating — office workers, business writers, technical authors who don't live in the terminal.

The name comes from **Yūhitsu** (右筆), the title of professional scribes who drafted and copied official documents for daimyō and shoguns from the Muromachi to Edo periods of Japan.

## Why?

Typst is gaining traction as a modern LaTeX alternative, but the existing tooling has gaps:

| Tool | OSS | Local | GUI | First-class Japanese |
|---|---|---|---|---|
| Typst (web app) | ❌ | ❌ | ✅ | ❌ |
| Typstify | ❌ | ✅ | ✅ | ❌ |
| Typstudio | ✅ | ✅ | ✅ | ❌ (long-stalled) |
| VS Code + Tinymist | ✅ | ✅ | ❌ (IDE) | ❌ |

Yuhitsu fills the **OSS × Local × GUI × first-class Japanese × practical** quadrant. It is a general-purpose Typst editor that happens to invest heavily in the parts most other editors neglect: Japanese typography, ergonomic templates for business documents, and a UI that doesn't assume you live in a terminal.

## Features (v0.1 alpha)

### Typst editor
- **Live preview** — even unsaved buffer changes reflect in real time (via `tinymist preview`)
- **LSP integration** — completion, diagnostics, hover docs (via `tinymist lsp`)
- **Syntax highlighting** for Typst
- **PDF export** — Ctrl+E
- **Find / Replace** — Ctrl+F / Ctrl+H, regex / case / whole-word
- **Insert helpers** — bold / italic / heading / list / math / code / link / footnote / quote / image / table / bibliography (toolbar buttons)

### Editor modes (switchable)
- **Default** (OS-standard keybindings)
- **vim** (experimental — known issues with IME / WebKit, advanced users only)
- **emacs**

### Files & projects
- **Tabs** — multiple files at once, drag-reorder, hot exit (tabs restored on next launch)
- **Project sidebar** — file tree, git status badges, right-click menu (new / rename / delete)
- **Image / PDF tabs** — view `.png` / `.svg` / `.pdf` etc. directly in tabs (Typst can `#image()` them)

### Built-in templates (ja / en)
- Business report / Technical report / Meeting minutes / Slides / Empty
- **Form-based template editing (lite)** — template function arguments are exposed as input fields; typing fills the document

### UI / customization
- **Command palette** (Ctrl+Shift+P / F1) — fuzzy search across all commands
- **Hamburger menu** (top-left) — categorized access to every feature
- **Toolbar editor** — drag to reorder, add / remove items, presets (standard / minimal / academic)
- **Keybindings editor** — every command rebindable
- **Dark / light themes** — auto (follow OS) or manual
- **Japanese / English UI** — auto (`navigator.language`) or manual
- **Custom title bar (CSD)** — file name, cursor position, character count consolidated at the top

### Bundled fonts
- **Harano Aji Mincho / Gothic** (Regular + Bold) — [trueroad/HaranoAjiFonts](https://github.com/trueroad/HaranoAjiFonts), SIL Open Font License 1.1

For the full list and roadmap, see [PROGRESS.md](./PROGRESS.md) (Japanese only for now).

---

## Download & install

> 🚧 v0.1.0-alpha is being prepared. After release, binaries will be available on the [Releases](https://github.com/barewalker/yuhitsu/releases) page.

### Linux

`.AppImage` (works on any distro) or `.deb` (Debian / Ubuntu) from Releases.

```bash
# AppImage
chmod +x yuhitsu_*.AppImage
./yuhitsu_*.AppImage

# .deb
sudo dpkg -i yuhitsu_*.deb
```

### Windows

Download the `.msi` installer from Releases and run it.

> ⚠️ **About the SmartScreen warning**
> Yuhitsu is an unfunded personal OSS project, so we don't carry a Microsoft code-signing certificate (which costs hundreds of USD per year). On first launch you'll see the blue SmartScreen warning — click **"More info" → "Run anyway"** to start. Helix, Alacritty, and many other OSS desktop apps work the same way.
> We're considering [SignPath.io's free OSS signing](https://signpath.io/) for the future.

### macOS

> 🚧 Not in distribution scope for alpha. The CI builds successfully but the maintainer doesn't have a Mac to test on. If you want to try it, see [Building from source](#building-from-source).

---

## Quick start

1. **Launch** → on first run, a template picker appears
2. Pick a template (business report / minutes / etc.) → content drops into the editor
3. **Top-left hamburger** → all commands by category
4. **F1** or **Ctrl+Shift+P** → command palette
5. **Ctrl+S** → save as `.typ`
6. **Ctrl+E** → export PDF

For more, open the **hamburger → Help → About Yuhitsu** dialog inside the app for documentation links.

---

## Building from source

### Prerequisites

- [Rust](https://www.rust-lang.org/) 1.77+
- [Node.js](https://nodejs.org/) 20+ + [pnpm](https://pnpm.io/) 10+
- [tinymist](https://github.com/Myriad-Dreamin/tinymist) on `PATH`
- Linux: `libwebkit2gtk-4.1-dev`, `libssl-dev`, `libgtk-3-dev`, etc. (Tauri's Linux build deps)
- macOS: Xcode CLT
- Windows: Microsoft C++ Build Tools + WebView2

### Setup

```bash
git clone --recursive https://github.com/barewalker/yuhitsu.git
cd yuhitsu/app
pnpm install
pnpm tauri dev
```

> ⚠️ **`--recursive` matters**: the bundled Harano Aji fonts are pulled in as a git submodule.

### Release build

```bash
cd app
pnpm tauri build
# Artifacts: app/src-tauri/target/release/bundle/
```

---

## Roadmap

- **Phase 0** (done): research & PoC, technology selection
- **Phase 1** (current): MVP — editor / templates / preview / distribution
- **Phase 2** (planned): UX polish, built-in AI features, friendly git UI
- **Phase 3** (planned): WYSIWYG-lite mode, `.typz` single-file bundle format, Yuhitsu as MCP server (external agent integration)

Details in [PROGRESS.md](./PROGRESS.md) (Japanese).

---

## Tech stack

- **Shell**: [Tauri 2](https://tauri.app/) (Rust + OS WebView)
- **Editor**: [CodeMirror 6](https://codemirror.net/)
- **Typst engine**: [Typst](https://github.com/typst/typst), used via [tinymist](https://github.com/Myriad-Dreamin/tinymist) (LSP / preview / compile)

---

## License

- Yuhitsu itself: **[Apache-2.0](./LICENSE)**
- Bundled fonts (Harano Aji Mincho / Gothic): **SIL Open Font License 1.1** (Copyright © trueroad)

---

## Author

[barewalker](https://github.com/barewalker)

A personal project that started from frustration with existing options for producing business documents. If you have similar pain points, hopefully Yuhitsu helps.

---

## Related

- [Typst](https://typst.app/)
- [Tinymist](https://github.com/Myriad-Dreamin/tinymist)
- [Harano Aji Fonts](https://github.com/trueroad/HaranoAjiFonts)

---

## Acknowledgements

Implementation of this project was assisted by [Claude Code](https://claude.com/claude-code) (Anthropic). Technical decisions, design, and quality responsibility lie with the author.
