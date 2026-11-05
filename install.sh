#!/usr/bin/env bash
set -e

REPO="futurekismo-oss/tapes"
BINARY_NAME="tapes"
LEGACY_BINARY_NAME="tapes-v2"
CONFIG_DIR="$HOME/.tapes"

install_file() {
  local src="$1"
  local dst="$2"
  if [ -w "$(dirname "$dst")" ]; then
    mv "$src" "$dst"
  else
    sudo mv "$src" "$dst"
  fi
}

remove_file() {
  local target="$1"
  if [ -w "$target" ] || [ -w "$(dirname "$target")" ]; then
    rm -f "$target"
  else
    sudo rm -f "$target"
  fi
}

echo "==> Checking system environment..."

if [ -f /etc/NIXOS ]; then
  echo ""
  echo "NixOS detected."
  echo "On NixOS, traditional binary installers are discouraged because binaries won't link against dynamic libraries properly."
  echo "Recommended options:"
  echo "    nix profile install github:$REPO"
  echo "    # or"
  echo "    nix run github:$REPO -- --help"
  echo ""
  printf "Do you still want to proceed with the standard manual binary installation? (y/N) "
  read -r REPLY </dev/tty
  case "$REPLY" in
  [yY][eE][sS] | [yY])
    echo "==> Proceeding with manual binary installation on NixOS..."
    ;;
  *)
    echo "Aborted."
    exit 0
    ;;
  esac
fi

ARCH=$(uname -m)
if [ "$ARCH" = "x86_64" ]; then
  TARGET_ASSET="$BINARY_NAME-x86_64-unknown-linux-gnu"
else
  echo "Error: Unsupported architecture ($ARCH). Currently, only x86_64 is supported for pre-built binaries."
  exit 1
fi

TARGET_DIR=""
NEED_PATH_WARN=false

if echo "$PATH" | tr ':' '\n' | grep -q "^$HOME/.local/bin$"; then
  TARGET_DIR="$HOME/.local/bin"
elif echo "$PATH" | tr ':' '\n' | grep -q "^/usr/local/bin$"; then
  TARGET_DIR="/usr/local/bin"
else
  TARGET_DIR="$HOME/.local/bin"
  NEED_PATH_WARN=true
fi

echo "==> Fetching latest release info..."
LATEST_TAG=$(curl -sL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_TAG" ]; then
  echo "Warning: Could not fetch latest release tag. Proceeding with default download..."
  LATEST_TAG="latest"
fi

INSTALLED_BIN="$TARGET_DIR/$BINARY_NAME"
if [ -x "$INSTALLED_BIN" ]; then
  CURRENT_VERSION=$("$INSTALLED_BIN" --version 2>/dev/null | head -n 1 || true)

  if [ -n "$CURRENT_VERSION" ] && [ -n "$LATEST_TAG" ]; then
    CLEAN_TAG="${LATEST_TAG#v}"
    if echo "$CURRENT_VERSION" | grep -q "$CLEAN_TAG"; then
      echo "==> $BINARY_NAME is already up to date ($CURRENT_VERSION)."
      exit 0
    else
      echo "==> Updating $BINARY_NAME: $CURRENT_VERSION -> $LATEST_TAG"
    fi
  else
    echo "==> Existing installation found at $INSTALLED_BIN. Overwriting..."
  fi
else
  echo "==> Installing $BINARY_NAME ($LATEST_TAG)..."
fi

RELEASE_URL="https://github.com/$REPO/releases/latest/download/$TARGET_ASSET"

echo "==> Preparing target and configuration directories..."
mkdir -p "$CONFIG_DIR" "$TARGET_DIR"

echo "==> Downloading $BINARY_NAME from GitHub Releases..."
TEMP_FILE=$(mktemp)
curl -fsSL "$RELEASE_URL" -o "$TEMP_FILE"
chmod +x "$TEMP_FILE"

echo "==> Installing to $TARGET_DIR/$BINARY_NAME..."
install_file "$TEMP_FILE" "$TARGET_DIR/$BINARY_NAME"

# Clean up legacy binary if it exists (only after successful installation)
LEGACY_LOCATIONS=(
  "$TARGET_DIR/$LEGACY_BINARY_NAME"
  "$(command -v "$LEGACY_BINARY_NAME" 2>/dev/null || true)"
)

for old_bin in "${LEGACY_LOCATIONS[@]}"; do
  if [ -n "$old_bin" ] && [ -e "$old_bin" ] && [ "$old_bin" != "$TARGET_DIR/$BINARY_NAME" ]; then
    echo "==> Notice: The binary has been renamed from '$LEGACY_BINARY_NAME' to '$BINARY_NAME'."
    echo "==> Removing legacy installation at $old_bin..."
    remove_file "$old_bin"
  fi
done

echo ""
echo "Successfully installed $BINARY_NAME to $TARGET_DIR/$BINARY_NAME"
echo "Created configuration directory at $CONFIG_DIR"

if [ -n "$TERM" ]; then
  CURRENT_SHELL=$(basename "$SHELL" 2>/dev/null || echo "")
  case "$CURRENT_SHELL" in
  bash)
    echo ""
    echo "To enable tab completion for Bash, add this to your ~/.bashrc:"
    echo "    source <(COMPLETIONS=bash $BINARY_NAME)"
    ;;
  zsh)
    echo ""
    echo "To enable tab completion for Zsh, add this to your ~/.zshrc:"
    echo "    source <(COMPLETIONS=zsh $BINARY_NAME)"
    ;;
  fish)
    echo ""
    echo "To enable tab completion for Fish, add this to your ~/.config/fish/config.fish:"
    echo "    COMPLETIONS=fish $BINARY_NAME | source"
    ;;
  nu* | nushell)
    echo ""
    echo "To enable tab completion for Nushell, add this to your Nushell config:"
    echo "    COMPLETIONS=nushell $BINARY_NAME | save --force ~/.cache/tapes/completions.nu"
    echo "    # and add 'use ~/.cache/tapes/completions.nu' to your config.nu"
    ;;
  elvish)
    echo ""
    echo "To enable tab completion for Elvish, add this to your Elvish rc:"
    echo "    COMPLETIONS=elvish $BINARY_NAME | eval"
    ;;
  *)
    echo ""
    echo "For tab completion, run with the appropriate shell environment variable (e.g., COMPLETIONS=bash/zsh/fish/nushell/elvish)."
    ;;
  esac
fi

if [ "$NEED_PATH_WARN" = true ]; then
  echo ""
  echo "Warning: Neither ~/.local/bin nor /usr/local/bin were found in your \$PATH."
  echo "Please add ~/.local/bin to your shell configuration (e.g., ~/.bashrc or ~/.zshrc):"
  echo ""
  echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
  echo ""
fi
