#!/usr/bin/env bash
# WWS All E2E: Orchestrates single-node API, multi-node, and Playwright UI tests.
# Usage: bash tests/e2e/wws_all_e2e.sh
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"
BINARY="$ROOT_DIR/target/release/wws-connector"
PASS_TOTAL=0; FAIL_TOTAL=0

log()  { echo ""; echo "[wws-all] $*"; }
sep()  { echo "══════════════════════════════════════════════════════"; }

cleanup_all() {
    pkill -f "wws-connector" 2>/dev/null || true
    sleep 1
}
trap cleanup_all EXIT

# ── Build (if binary is stale or missing) ─────────────────────────────────────
log "Checking binary..."
cd "$ROOT_DIR"
if [ ! -f "$BINARY" ]; then
    log "Building release binary..."
    cargo build --release -p wws-connector 2>&1 | tail -3
    log "Binary ready: $BINARY"
else
    log "Binary found: $BINARY"
fi

# ── Phase 1: Single-node API tests ───────────────────────────────────────────
sep
log "PHASE 1: Single-node API tests"
sep
pkill -f "wws-connector" 2>/dev/null || true; sleep 1
if bash "$ROOT_DIR/tests/e2e/wws_e2e.sh"; then
    log "PHASE 1: PASSED"
    PASS_TOTAL=$((PASS_TOTAL+1))
else
    log "PHASE 1: FAILED"
    FAIL_TOTAL=$((FAIL_TOTAL+1))
fi
pkill -f "wws-connector" 2>/dev/null || true; sleep 2

# ── Phase 2: Multi-node peer discovery tests ──────────────────────────────────
sep
log "PHASE 2: Multi-node peer discovery tests"
sep
if bash "$ROOT_DIR/tests/e2e/wws_multinode_e2e.sh"; then
    log "PHASE 2: PASSED"
    PASS_TOTAL=$((PASS_TOTAL+1))
else
    log "PHASE 2: FAILED"
    FAIL_TOTAL=$((FAIL_TOTAL+1))
fi
pkill -f "wws-connector" 2>/dev/null || true; sleep 2

# ── Phase 3: Playwright UI tests ──────────────────────────────────────────────
sep
log "PHASE 3: Playwright UI tests (starting single connector on 19370/19371)..."
sep

mkdir -p /tmp/wws-e2e-ui
"$BINARY" \
    --listen      /ip4/127.0.0.1/tcp/19001 \
    --rpc         127.0.0.1:19370 \
    --files-addr  127.0.0.1:19371 \
    --agent-name  wws-ui-test \
    --identity-path /tmp/wws-e2e-ui/ui.key \
    >/tmp/wws-e2e-ui/ui.log 2>&1 &
UI_PID=$!

# Wait for HTTP to be healthy
for i in $(seq 1 15); do
    sleep 1
    if curl -sf http://127.0.0.1:19371/api/health >/dev/null 2>&1; then
        log "Connector healthy on http://127.0.0.1:19371"
        break
    fi
done

cd "$ROOT_DIR/tests/playwright"
npm install --quiet 2>/dev/null
npx playwright install chromium >/dev/null 2>&1

set +e
WEB_BASE_URL=http://127.0.0.1:19371 \
npx playwright test wws-features-e2e.spec.js \
    --workers=1 \
    --reporter=list \
    --retries=0 \
    --timeout=120000
PW_EXIT=$?
set -e

kill "$UI_PID" 2>/dev/null || true

if [ $PW_EXIT -eq 0 ]; then
    log "PHASE 3: PASSED"
    PASS_TOTAL=$((PASS_TOTAL+1))
else
    log "PHASE 3: FAILED (exit=$PW_EXIT)"
    FAIL_TOTAL=$((FAIL_TOTAL+1))
fi

# ── Final summary ─────────────────────────────────────────────────────────────
sep
echo ""
echo "  WWS ALL E2E SUMMARY"
echo "  Phases passed: $PASS_TOTAL"
echo "  Phases failed: $FAIL_TOTAL"
sep

[ "$FAIL_TOTAL" -eq 0 ]
