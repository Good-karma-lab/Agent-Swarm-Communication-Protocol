#!/usr/bin/env bash

set -euo pipefail

echo "[zeroclaw] Updating to latest from GitHub..."

if ! command -v cargo >/dev/null 2>&1; then
    echo "[zeroclaw] Error: cargo is required to build latest ZeroClaw"
    exit 1
fi

TMP_DIR="$(mktemp -d /tmp/zeroclaw-src.XXXXXX)"
trap 'rm -rf "$TMP_DIR"' EXIT

git clone "https://github.com/zeroclaw-labs/zeroclaw.git" "$TMP_DIR"
cargo install --path "$TMP_DIR" --force --locked

if command -v zeroclaw >/dev/null 2>&1; then
    echo "[zeroclaw] Installed: $(zeroclaw --version 2>/dev/null || echo 'version unknown')"
else
    echo "[zeroclaw] Warning: zeroclaw is not on PATH after update"
fi
