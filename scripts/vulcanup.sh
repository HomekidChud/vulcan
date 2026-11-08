#!/bin/sh
# vulcanup — the Vulcan installer (rustup-style: installs to ~/.vulcan, manages PATH)
#
#   curl -fsSL https://raw.githubusercontent.com/HomekidChud/vulcan/master/scripts/vulcanup.sh | sh
#
# or build from source:
#   sh vulcanup.sh --source
#
# What it does:
#   1. downloads the latest prebuilt binary for your OS/arch from GitHub
#      Releases (static musl builds on Linux — they run on Termux/Android too)
#   2. installs it to ~/.vulcan/bin
#   3. adds ~/.vulcan/bin to PATH (in ~/.bashrc, ~/.profile or $PREFIX/etc/profile.d)
#   4. runs `vulcan --version` to verify
#
# After that, `vulcan self-update` keeps it current.
set -eu

REPO="HomekidChud/vulcan"
VULCAN_HOME="${VULCAN_HOME:-$HOME/.vulcan}"
BIN="$VULCAN_HOME/bin"

say() { printf '%s\n' "$*"; }
die() { printf 'vulcanup: %s\n' "$*" >&2; exit 1; }

# --- detect platform ---------------------------------------------------------
os() {
  case "$(uname -s)" in
    Linux*) echo linux ;;
    Darwin*) echo macos ;;
    *) die "unsupported OS: $(uname -s) (use --source to build from source)" ;;
  esac
}
arch() {
  case "$(uname -m)" in
    x86_64|amd64) echo x86_64 ;;
    aarch64|arm64) echo aarch64 ;;
    armv7l|armv6l) die "ARMv6/7 not prebuilt — use --source (needs Rust on device)" ;;
    *) die "unsupported arch: $(uname -m)" ;;
  esac
}

# --- helpers -----------------------------------------------------------------
need() { command -v "$1" >/dev/null 2>&1 || die "missing dependency: $1 (install it first)"; }

fetch() { # fetch <url> <dest>
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL -o "$2" "$1" || return 1
  elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$2" "$1" || return 1
  else
    die "need curl or wget"
  fi
}

add_path() { # add a line to shell rc files (idempotent)
  line='export PATH="$HOME/.vulcan/bin:$PATH"'
  # Termux: prefer $PREFIX/etc/profile.d so every shell picks it up
  if [ -n "${PREFIX:-}" ] && [ -d "$PREFIX/etc/profile.d" ]; then
    if ! grep -qs '.vulcan/bin' "$PREFIX/etc/profile.d/vulcan.sh" 2>/dev/null; then
      printf '%s\n' "$line" > "$PREFIX/etc/profile.d/vulcan.sh"
    fi
    return
  fi
  for rc in "$HOME/.bashrc" "$HOME/.profile"; do
    [ -f "$rc" ] || continue
    grep -qs '.vulcan/bin' "$rc" || printf '\n%s\n' "$line" >> "$rc"
  done
}

# --- source build ------------------------------------------------------------
if [ "${1:-}" = "--source" ]; then
  need git
  need cargo || die "cargo not found — install Rust first: https://rustup.rs"
  tmp="$(mktemp -d)"
  say "cloning $REPO ..."
  git clone --depth 1 "https://github.com/$REPO" "$tmp/vulcan" || die "clone failed"
  say "building (cargo build --release) ..."
  (cd "$tmp/vulcan" && cargo build --release) || die "build failed"
  mkdir -p "$BIN"
  cp "$tmp/vulcan/target/release/vulcan" "$BIN/vulcan"
  chmod +x "$BIN/vulcan"
  rm -rf "$tmp"
  add_path
  say "installed: $BIN/vulcan"
  "$BIN/vulcan" --version || true
  say "restart your shell (or run: export PATH=\"$BIN:\$PATH\")"
  exit 0
fi

# --- prebuilt download -------------------------------------------------------
OS="$(os)"
ARCH="$(arch)"
ASSET="vulcan-latest-$OS-$ARCH.tar.gz"
URL="https://github.com/$REPO/releases/latest/download/$ASSET"

need tar
mkdir -p "$BIN" "$VULCAN_HOME/downloads"
say "downloading $ASSET ..."
fetch "$URL" "$VULCAN_HOME/downloads/$ASSET" || die "download failed — no release asset for $OS-$ARCH?
  releases: https://github.com/$REPO/releases
  (or build from source: sh vulcanup.sh --source)"

tmp="$(mktemp -d)"
tar -xzf "$VULCAN_HOME/downloads/$ASSET" -C "$tmp" || die "extract failed"
[ -f "$tmp/vulcan" ] || die "archive did not contain a 'vulcan' binary"
mv "$tmp/vulcan" "$BIN/vulcan"
chmod +x "$BIN/vulcan"
rm -rf "$tmp"

add_path
say "installed: $BIN/vulcan"
"$BIN/vulcan" --version || true
say "restart your shell (or run: export PATH=\"$BIN:\$PATH\")"
say "update later with: vulcan self-update"
