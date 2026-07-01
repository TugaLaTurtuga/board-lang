#!/usr/bin/env bash
set -e

INSTALLED=()

install_cargo() {
  if command -v brew >/dev/null 2>&1; then
    echo "Installing Rust via Homebrew..."
    brew install rust
    INSTALLED+=("rust (brew)")
  else
    echo "Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    INSTALLED+=("rust (rustup)")
  fi
}

# Check if cargo exists
if ! command -v cargo >/dev/null 2>&1; then
  install_cargo
fi

if [ "${#INSTALLED[@]}" -eq 0 ]; then
  echo "All dependencies already installed."
else
  echo "Installed these dependencies:"
  for dep in "${INSTALLED[@]}"; do
    echo "  - $dep"
  done
fi
