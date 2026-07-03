#!/usr/bin/env bash
set -euo pipefail

# tui-notes installer.
# Builds the release binary, installs it to ~/.local/bin, ensures the notes
# directory exists, and prints the Hyprland keybind to paste in.
#
# Overridable env vars:
#   BIN_DIR        install target       (default: ~/.local/bin)
#   TUI_NOTES_DIR  notes root to create (default: ~/.local/tui-notes)

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
NOTES_DIR="${TUI_NOTES_DIR:-$HOME/.local/tui-notes}"
BIN_NAME="tui-notes"

echo "==> building release binary"
cargo build --release --manifest-path "$REPO_DIR/Cargo.toml"

echo "==> installing to $BIN_DIR/$BIN_NAME"
mkdir -p "$BIN_DIR"
install -m 0755 "$REPO_DIR/target/release/$BIN_NAME" "$BIN_DIR/$BIN_NAME"

echo "==> ensuring notes dir $NOTES_DIR"
mkdir -p "$NOTES_DIR"

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) echo "WARN: $BIN_DIR is not in \$PATH — add it to your shell profile" ;;
esac

cat <<EOF

Done. Binary installed: $BIN_DIR/$BIN_NAME

Hyprland keybind — add to ~/.config/hypr/hyprland.conf:

  bind = SUPER ALT, N, exec, kitty --class tui-notes -e $BIN_DIR/$BIN_NAME

Then reload Hyprland:

  hyprctl reload

Press SUPER+ALT+N to open the notes tree.
EOF
