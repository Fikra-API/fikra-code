#!/bin/sh
# Fikra Code installer for Linux and macOS.
# Usage: curl -fsSL https://fikraapi.co.ke/code/install.sh | sh
#
# Asset contract (the release workflow must produce exactly this):
#   GitHub release tag:  vX.Y.Z
#   Asset name:          fikra-package-<target>.tar.gz
#   Archive root:        bin/fikra (entrypoint) plus helper binaries and resources,
#                        in the same layout as upstream's codex package.
#   <target> is one of:  x86_64-unknown-linux-musl, aarch64-unknown-linux-musl,
#                        x86_64-apple-darwin, aarch64-apple-darwin
set -eu

REPO="${FIKRA_REPO:-Fikra-API/fikra-code}"
VERSION="${FIKRA_VERSION:-latest}"
FIKRA_HOME="${FIKRA_HOME:-$HOME/.fikra}"
BIN_DIR="${FIKRA_INSTALL_DIR:-$HOME/.local/bin}"

say() { printf '%s\n' "$*"; }
die() { printf 'fikra install: %s\n' "$*" >&2; exit 1; }

need() { command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"; }
need uname
need tar
need mkdir
if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fL --retry 3 --retry-delay 2 --connect-timeout 20 -C - -o "$2" "$1"; }
  fetch_text() { curl -fsSL --retry 3 --connect-timeout 20 "$1"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -c -t 3 -T 20 -O "$2" "$1"; }
  fetch_text() { wget -qO- -t 3 -T 20 "$1"; }
else
  die "need curl or wget"
fi

os="$(uname -s)"
arch="$(uname -m)"
case "$arch" in
  x86_64|amd64) arch=x86_64 ;;
  arm64|aarch64) arch=aarch64 ;;
  *) die "unsupported CPU architecture: $arch" ;;
esac
case "$os" in
  Linux) target="${arch}-unknown-linux-musl" ;;
  Darwin) target="${arch}-apple-darwin" ;;
  *) die "unsupported OS: $os (on Windows, use install.ps1)" ;;
esac

if [ "$VERSION" = "latest" ]; then
  VERSION="$(fetch_text "https://api.github.com/repos/$REPO/releases/latest" \
    | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n 1)"
  [ -n "$VERSION" ] || die "could not find the latest release of $REPO"
fi

asset="fikra-package-${target}.tar.gz"
url="https://github.com/$REPO/releases/download/$VERSION/$asset"
dest="$FIKRA_HOME/packages/$VERSION"
tmp="$FIKRA_HOME/downloads"
mkdir -p "$tmp" "$BIN_DIR"

say "Installing Fikra Code $VERSION ($target)"
# Downloads resume where they stopped, so a dropped connection doesn't restart from zero.
fetch "$url" "$tmp/$asset" || die "download failed: $url"

if fetch_text "$url.sha256" > "$tmp/$asset.sha256" 2>/dev/null; then
  expected="$(cut -d ' ' -f 1 < "$tmp/$asset.sha256")"
  if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$tmp/$asset" | cut -d ' ' -f 1)"
  else
    actual="$(shasum -a 256 "$tmp/$asset" | cut -d ' ' -f 1)"
  fi
  [ "$expected" = "$actual" ] || { rm -f "$tmp/$asset"; die "checksum mismatch, download removed; run the installer again"; }
fi

rm -rf "$dest.partial"
mkdir -p "$dest.partial"
tar -xzf "$tmp/$asset" -C "$dest.partial"
[ -x "$dest.partial/bin/fikra" ] || die "archive is missing bin/fikra"
rm -rf "$dest"
mv "$dest.partial" "$dest"
ln -sfn "$dest" "$FIKRA_HOME/packages/current"
ln -sf "$FIKRA_HOME/packages/current/bin/fikra" "$BIN_DIR/fikra"
rm -f "$tmp/$asset" "$tmp/$asset.sha256"

# Keep only the two newest versions, to save disk space on small machines.
ls -1dt "$FIKRA_HOME"/packages/v* 2>/dev/null | tail -n +3 | while read -r old; do rm -rf "$old"; done

say "Installed: $BIN_DIR/fikra"
case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) say "Add this to your shell profile, then open a new terminal:"
     say "  export PATH=\"$BIN_DIR:\$PATH\"" ;;
esac
say "Next: run 'fikra login' and paste your API key from https://fikraapi.co.ke/dashboard"
