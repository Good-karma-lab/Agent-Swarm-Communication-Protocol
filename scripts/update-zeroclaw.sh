#!/usr/bin/env bash

set -euo pipefail

echo "[zeroclaw] Updating to latest from GitHub..."

if ! command -v python3 >/dev/null 2>&1; then
    echo "[zeroclaw] Error: python3 is required"
    exit 1
fi

python3 -m pip install --upgrade "git+https://github.com/zeroclaw-labs/zeroclaw.git"

if command -v zeroclaw >/dev/null 2>&1; then
    echo "[zeroclaw] Installed: $(zeroclaw --version 2>/dev/null || echo 'version unknown')"
else
    echo "[zeroclaw] Warning: zeroclaw is not on PATH after update"
fi
