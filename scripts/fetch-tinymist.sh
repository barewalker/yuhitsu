#!/bin/sh
# tinymist バイナリを GitHub Releases から取得して
# `app/src-tauri/binaries/tinymist-<host-triple>` に配置する。
#
# 用途:
# - dev:`pnpm tauri dev` する前に手元で 1 度走らせる
# - CI:release.yml の各マトリクスで実行(target を引数 / 環境変数で渡す)
#
# 使い方:
#   ./scripts/fetch-tinymist.sh                  # ホストの triple を自動検出
#   TINYMIST_TARGET=aarch64-unknown-linux-gnu ./scripts/fetch-tinymist.sh
#   TINYMIST_VERSION=v0.14.16 ./scripts/fetch-tinymist.sh   # version 指定
#
# 既に配置済なら何もしない(冪等)。

set -eu

VERSION="${TINYMIST_VERSION:-v0.14.16}"

# 配置先(リポジトリルートからの相対パス)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN_DIR="$ROOT/app/src-tauri/binaries"
mkdir -p "$BIN_DIR"

# Target triple の決定
if [ -n "${TINYMIST_TARGET:-}" ]; then
  TRIPLE="$TINYMIST_TARGET"
else
  KERNEL="$(uname -s)"
  MACHINE="$(uname -m)"
  case "$KERNEL/$MACHINE" in
    Linux/x86_64)   TRIPLE="x86_64-unknown-linux-gnu" ;;
    Linux/aarch64)  TRIPLE="aarch64-unknown-linux-gnu" ;;
    Linux/arm64)    TRIPLE="aarch64-unknown-linux-gnu" ;;
    Darwin/arm64)   TRIPLE="aarch64-apple-darwin" ;;
    Darwin/x86_64)  TRIPLE="x86_64-apple-darwin" ;;
    MINGW*/x86_64|MSYS*/x86_64|CYGWIN*/x86_64) TRIPLE="x86_64-pc-windows-msvc" ;;
    MINGW*/aarch64|MSYS*/aarch64) TRIPLE="aarch64-pc-windows-msvc" ;;
    *)
      echo "[fetch-tinymist] Unsupported host: $KERNEL/$MACHINE" >&2
      echo "                 Set TINYMIST_TARGET explicitly." >&2
      exit 1
      ;;
  esac
fi

# Windows は .exe / .zip、それ以外は .tar.gz
case "$TRIPLE" in
  *windows*) ARCHIVE_EXT="zip"; BIN_EXT=".exe" ;;
  *)         ARCHIVE_EXT="tar.gz"; BIN_EXT="" ;;
esac

ARCHIVE_NAME="tinymist-$TRIPLE.$ARCHIVE_EXT"
URL="https://github.com/Myriad-Dreamin/tinymist/releases/download/$VERSION/$ARCHIVE_NAME"
TARGET_PATH="$BIN_DIR/tinymist-$TRIPLE$BIN_EXT"

# 既に配置済なら skip
if [ -x "$TARGET_PATH" ]; then
  echo "[fetch-tinymist] already at $TARGET_PATH (skip)"
  exit 0
fi

echo "[fetch-tinymist] downloading $VERSION for $TRIPLE"
echo "[fetch-tinymist]   from $URL"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# DL
if ! curl -fL --retry 3 --retry-delay 2 -o "$TMP/$ARCHIVE_NAME" "$URL"; then
  echo "[fetch-tinymist] download failed" >&2
  exit 1
fi

# 展開
case "$ARCHIVE_EXT" in
  tar.gz)
    tar -xzf "$TMP/$ARCHIVE_NAME" -C "$TMP"
    ;;
  zip)
    if command -v unzip >/dev/null 2>&1; then
      unzip -q "$TMP/$ARCHIVE_NAME" -d "$TMP"
    else
      # Windows runner だと unzip 無いことがある
      pwsh -Command "Expand-Archive -Path '$TMP/$ARCHIVE_NAME' -DestinationPath '$TMP'"
    fi
    ;;
esac

# アーカイブ内のバイナリを探す(tar.gz / zip でディレクトリ階層が違う場合への保険)
EXTRACTED="$(find "$TMP" -type f \( -name 'tinymist' -o -name 'tinymist.exe' \) | head -n 1)"
if [ -z "$EXTRACTED" ]; then
  echo "[fetch-tinymist] tinymist binary not found in archive" >&2
  ls -R "$TMP" >&2
  exit 1
fi

cp "$EXTRACTED" "$TARGET_PATH"
chmod +x "$TARGET_PATH"
echo "[fetch-tinymist] installed: $TARGET_PATH"
