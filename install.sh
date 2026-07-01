#!/usr/bin/env bash
set -euo pipefail

INSTALL_DIR="$HOME/.code/dependencies/board-lang"
CONFIG_DIR="$HOME/.config/board-lang"
STATE_DIR="$HOME/.code/dependencies/.board-lang"

TMP_DIR="$(mktemp -d)"
cleanup() { rm -rf "$TMP_DIR"; }
trap cleanup EXIT

mkdir -p "$STATE_DIR"
mkdir -p "$CONFIG_DIR"

echo "Fetching latest release info..."
API_JSON="$(curl -s https://api.github.com/repos/TugaLaTurtuga/board-lang/releases/latest)"

ASSET_URL="$(echo "$API_JSON" | grep browser_download_url | grep tar.gz | cut -d '"' -f 4)"

if [[ -z "$ASSET_URL" ]]; then
  echo "No .tar.gz release asset found"
  exit 1
fi

echo "Downloading latest release..."
curl -L "$ASSET_URL" -o "$TMP_DIR/release.tar.gz"

echo "Unpacking..."
tar -xzf "$TMP_DIR/release.tar.gz" -C "$TMP_DIR"

SRC_DIR="$(find "$TMP_DIR" -maxdepth 1 -type d -name "board-lang*" | head -n 1)"

if [[ -z "$SRC_DIR" ]]; then
  echo "Could not locate source directory"
  exit 1
fi

cd "$SRC_DIR"

# -----------------------------
# Install dependencies
# -----------------------------
echo "Installing dependencies..."
DEPS_OUTPUT="$(./dependencies.sh)"

echo "$DEPS_OUTPUT" \
  | sed -n 's/^\t//p' \
  > "$DEPS_FILE"

# -----------------------------
# Build
# -----------------------------
echo "Building project..."
if [[ -f "Cargo.toml" ]]; then
  cargo build --release
else
  echo "No known build method found"
  exit 1
fi

# -----------------------------
# Put config files
# -----------------------------
mv default-config-files $CONFIG_DIR


# -----------------------------
# Install build output
# -----------------------------
echo "Installing to $INSTALL_DIR..."
rm -rf "$INSTALL_DIR"
mkdir -p "$INSTALL_DIR"

if [[ -d "./build" ]]; then
  cp -r ./build/* "$INSTALL_DIR/"
elif [[ -d "./target/release" ]]; then
  cp -r ./target/release/* "$INSTALL_DIR/"
else
  echo "Build output not found"
  exit 1
fi

echo "board-lang installed successfully"
