#!/usr/bin/env bash
# vibe-pilot installer — cargo install, with a release-binary fast path.
#   curl -fsSL https://vibe-pilot.dev/install.sh | bash
set -euo pipefail

REPO="vibe-pilot/vibe-pilot"
VERSION="${VP_VERSION:-latest}"

say() { printf '\033[32m[vp]\033[0m %s\n' "$1"; }
warn() { printf '\033[33m[vp]\033[0m %s\n' "$1" >&2; }

ARCH="$(uname -m)"
OS="$(uname -s)"
case "$OS" in
  Linux) OSDIR="linux" ;;
  Darwin) OSDIR="macos" ;;
  *) OSDIR="" ;;
esac

# ---- 1. release binary, when we can match one ----
if [ -n "$OSDIR" ] && [ "$VERSION" = "latest" ] && command -v curl >/dev/null 2>&1; then
  case "$ARCH" in
    x86_64) TARBALL="vp-x86_64-${OSDIR}.tar.gz" ;;
    aarch64|arm64) TARBALL="vp-aarch64-${OSDIR}.tar.gz" ;;
    *) TARBALL="" ;;
  esac
  if [ -n "$TARBALL" ]; then
    URL="https://github.com/${REPO}/releases/latest/download/${TARBALL}"
    TMP="$(mktemp -d)"
    if curl -fsSL "$URL" -o "$TMP/$TARBALL" 2>/dev/null; then
      tar -xzf "$TMP/$TARBALL" -C "$TMP"
      BIN_DIR="${CARGO_HOME:-$HOME/.cargo}/bin"
      mkdir -p "$BIN_DIR"
      mv "$TMP/vp" "$BIN_DIR/vp"
      chmod +x "$BIN_DIR/vp"
      say "installed vp -> $BIN_DIR/vp"
      say "next: cd your-repo && vp report"
      exit 0
    fi
    warn "no prebuilt binary for $OSDIR/$ARCH — falling back to cargo"
  fi
fi

# ---- 2. cargo install (zero deps: compiles in seconds) ----
if ! command -v cargo >/dev/null 2>&1; then
  warn "cargo not found. install Rust first:"
  warn "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
  exit 1
fi

if [ "$VERSION" = "latest" ]; then
  cargo install --locked --git "https://github.com/${REPO}"
else
  cargo install --locked --git "https://github.com/${REPO}" --tag "$VERSION"
fi

say "installed $(command -v vp)"
say "next: cd your-repo && vp report"
