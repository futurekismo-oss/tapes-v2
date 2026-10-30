#!/usr/bin/env bash
set -e

REPO="futurekismo-oss/tapes-v2"
BINARY_NAME="tapes-v2"
CONFIG_DIR="$HOME/.tapes"

echo "==> Checking system environment..."

if [ -f /etc/NIXOS ]; then
  echo ""
  echo "NixOS detected."
  echo "On NixOS, traditional binary installers are discouraged because binaries won't link against dynamic libraries properly."
  echo "Please add 'tapes-v2' to your configuration.nix or home.nix, or run it directly via:"
  echo ""
  echo "    nix profile install github:$REPO"
  echo "    # or"
  echo "    nix run github:$REPO -- --help"
  echo ""
  exit 0
fi

ARCH=$(uname -m)
if [ "$ARCH" = "x86_64" ]; then
  TARGET_ASSET="tapes-v2-x86_64-unknown-linux-gnu"
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
LATEST_TAG=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_TAG" ]; then
  echo "Warning: Could not fetch latest release tag. Proceeding with default download..."
  LATEST_TAG="latest"
fi

INSTALLED_BIN="$TARGET_DIR/$BINARY_NAME"
if [ -x "$INSTALLED_BIN" ]; then
  CURRENT_VERSION=$("$INSTALLED_BIN" --version 2>/dev/null | head -n 1 || true)

  if [ -n "$CURRENT_VERSION" ] && [ -n "$LATEST_TAG" ]; then
    if echo "$CURRENT_VERSION" | grep -q "$LATEST_TAG"; then
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

echo "==> Preparing configuration directory ($CONFIG_DIR)..."
mkdir -p "$CONFIG_DIR"

if [ "$TARGET_DIR" = "$HOME/.local/bin" ]; then
  mkdir -p "$HOME/.local/bin"
fi

echo "==> Downloading $BINARY_NAME from GitHub Releases..."
TEMP_FILE=$(mktemp)
curl -L "$RELEASE_URL" -o "$TEMP_FILE"
chmod +x "$TEMP_FILE"

echo "==> Installing to $TARGET_DIR/$BINARY_NAME..."
if [ "$TARGET_DIR" = "/usr/local/bin" ]; then
  sudo mv "$TEMP_FILE" "$TARGET_DIR/$BINARY_NAME"
else
  mv "$TEMP_FILE" "$TARGET_DIR/$BINARY_NAME"
fi

echo ""
echo "Successfully installed $BINARY_NAME to $TARGET_DIR/$BINARY_NAME"
echo "Created configuration directory at $CONFIG_DIR"

if [ "$NEED_PATH_WARN" = true ]; then
  echo ""
  echo "Warning: Neither ~/.local/bin nor /usr/local/bin were found in your \$PATH."
  echo "Please add ~/.local/bin to your shell configuration (e.g., ~/.bashrc or ~/.zshrc):"
  echo ""
  echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
  echo ""
fi
