#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Ensure cargo is on PATH in non-interactive shells.
export PATH="$HOME/.cargo/bin:$PATH"
if [ -f "$ROOT_DIR/scripts/load-env.sh" ]; then
    # shellcheck disable=SC1091
    source "$ROOT_DIR/scripts/load-env.sh"
fi

cd "$ROOT_DIR"

CONSOLE_PID=""

cleanup() {
    if [ -n "$CONSOLE_PID" ]; then
        kill "$CONSOLE_PID" >/dev/null 2>&1 || true
        wait "$CONSOLE_PID" 2>/dev/null || true
    fi
    ./swarm-manager.sh stop >/dev/null 2>&1 || true
}
trap cleanup EXIT

./swarm-manager.sh stop >/dev/null 2>&1 || true

echo "[playwright-real] Building connector binary"
if ! command -v cargo >/dev/null 2>&1; then
    echo "[playwright-real] cargo not found in PATH. Install Rust or add cargo to PATH."
    exit 1
fi
cargo build --release -p openswarm-connector >/dev/null

echo "[playwright-real] Starting swarm of 30 agents"
echo "[playwright-real] LLM backend: ${LLM_BACKEND:-unset}, model: ${MODEL_NAME:-unset}"
./swarm-manager.sh start-agents 30

RPC_PORT=$(awk -F'|' 'NR==1 {print $5}' /tmp/openswarm-swarm/nodes.txt)
P2P_PORT=$(awk -F'|' 'NR==1 {print $4}' /tmp/openswarm-swarm/nodes.txt)

if [ -z "$RPC_PORT" ] || [ -z "$P2P_PORT" ]; then
    echo "[playwright-real] Failed to parse swarm node info"
    exit 1
fi

echo "[playwright-real] Waiting for bootstrap node health"
for _ in $(seq 1 60); do
    if echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"s","signature":""}' | nc 127.0.0.1 "$RPC_PORT" >/dev/null 2>&1; then
        break
    fi
    sleep 1
done

PEER_ID=$(echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"s","signature":""}' | nc 127.0.0.1 "$RPC_PORT" | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d["result"]["agent_id"].replace("did:swarm:",""))')
BOOTSTRAP_ADDR="/ip4/127.0.0.1/tcp/$P2P_PORT/p2p/$PEER_ID"

echo "[playwright-real] Starting dedicated web console connector"
./target/release/openswarm-connector \
  --listen /ip4/127.0.0.1/tcp/22900 \
  --rpc 127.0.0.1:22970 \
  --files-addr 127.0.0.1:22971 \
  --bootstrap "$BOOTSTRAP_ADDR" \
  --agent-name operator-web-30 \
  > /tmp/openswarm-playwright-console.log 2>&1 &
CONSOLE_PID=$!

echo "[playwright-real] Waiting for web console readiness"
for _ in $(seq 1 80); do
    if curl -sf "http://127.0.0.1:22971/api/health" >/dev/null; then
        break
    fi
    sleep 1
done

echo "[playwright-real] Installing Playwright deps"
cd "$ROOT_DIR/tests/playwright"
npm install >/dev/null
npx playwright install chromium >/dev/null

echo "[playwright-real] Running browser E2E in headed mode"
export WEB_BASE_URL="http://127.0.0.1:22971"
export PLAYWRIGHT_HEADED=1

if [ -z "${DISPLAY:-}" ]; then
    if command -v xvfb-run >/dev/null 2>&1; then
        echo "[playwright-real] No DISPLAY found, using xvfb-run"
        xvfb-run -a npx playwright test e2e-30-agent-web.spec.js --workers=1
    else
        echo "[playwright-real] WARNING: DISPLAY is not set and xvfb-run is missing."
        echo "[playwright-real] Falling back to headless mode with trace/video artifacts."
        PLAYWRIGHT_HEADED=0 npx playwright test e2e-30-agent-web.spec.js --workers=1
    fi
else
    npx playwright test e2e-30-agent-web.spec.js --workers=1
fi

echo "[playwright-real] If failures happen, inspect: tests/playwright/playwright-report/index.html"

echo "[playwright-real] Real 30-agent web E2E PASSED"
