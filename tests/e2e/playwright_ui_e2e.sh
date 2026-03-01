#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

cargo build --release -p wws-connector >/dev/null

cd tests/playwright
npm install >/dev/null
npx playwright install chromium >/dev/null
npx playwright test webapp.spec.js
