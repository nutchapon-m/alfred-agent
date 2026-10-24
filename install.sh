#!/usr/bin/env bash
# Install alfred-agent: build from source and copy the binary to a bin directory.
#
# Usage:
#   ./install.sh                 # install to ~/.local/bin
#   PREFIX=/usr/local ./install.sh   # install to /usr/local/bin (may need sudo)
#   ./install.sh --uninstall     # remove the binary
set -euo pipefail

BIN_NAME="alfred-agent"      # installed name
CARGO_BIN="alfred_agent"     # name produced by cargo
PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
CONFIG_DIR="$HOME/.alfred"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

info() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarn:\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

if [[ "${1:-}" == "--uninstall" ]]; then
  rm -f "$BIN_DIR/$BIN_NAME"
  info "Removed $BIN_DIR/$BIN_NAME (config in $CONFIG_DIR left untouched)"
  exit 0
fi

command -v cargo >/dev/null 2>&1 \
  || die "cargo not found. Install Rust first: https://rustup.rs"
command -v claude >/dev/null 2>&1 \
  || warn "'claude' CLI not found in PATH. alfred-agent needs it at runtime."

info "Building release binary"
(cd "$ROOT" && cargo build --release --locked)

info "Installing to $BIN_DIR/$BIN_NAME"
mkdir -p "$BIN_DIR"
install -m 755 "$ROOT/target/release/$CARGO_BIN" "$BIN_DIR/$BIN_NAME"

info "Creating config directory $CONFIG_DIR"
mkdir -p "$CONFIG_DIR"
[[ -e "$CONFIG_DIR/system_prompt.txt" ]] || : > "$CONFIG_DIR/system_prompt.txt"
if [[ ! -e "$CONFIG_DIR/prompt.json" ]]; then
  printf '%s\n' '{"type":"user","message":{"role":"user","content":"Hello"}}' > "$CONFIG_DIR/prompt.json"
fi

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) warn "$BIN_DIR is not in your PATH. Add: export PATH=\"$BIN_DIR:\$PATH\"" ;;
esac

info "Done. Try: $BIN_NAME --help"
