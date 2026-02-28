#!/usr/bin/env bash
#
# Phase 3 Holonic E2E via Playwright UI console
#
# Runs a full end-to-end test of the holonic swarm using the React web console:
#   • 20 connectors, each on its own port
#   • 20 phase3 agents, each attached to its own connector
#   • Task submitted via the web UI
#   • Playwright collects full trace: deliberation, plans, decompositions,
#     execution details, synthesis, final answer
#
# Usage:
#   bash tests/e2e/phase3_playwright_e2e.sh
#
# No fallbacks. No cheating.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"

SWARM_DIR="/tmp/asip-test"
AGENT_LOG_DIR="$SWARM_DIR/agents"
NODES_FILE="$SWARM_DIR/nodes.txt"
AGENT_PIDS=()

log() { echo "[phase3-playwright] $*"; }

# ── cleanup ──────────────────────────────────────────────────────────────────

cleanup() {
    log ""
    log "Stopping all agents and connectors..."
    if [[ ${#AGENT_PIDS[@]} -gt 0 ]]; then
        for pid in "${AGENT_PIDS[@]}"; do
            kill "$pid" 2>/dev/null || true
        done
    fi
    # phase3 agents launched with & — kill by script name too
    pkill -f "phase3_agent.py" 2>/dev/null || true
    pkill -f "openswarm-connector" 2>/dev/null || true
    sleep 1
    log "All processes stopped."
}
trap cleanup EXIT

# ── Step 1: Build ────────────────────────────────────────────────────────────

log "Building release binary..."
cd "$ROOT_DIR"
cargo build --release -p openswarm-connector 2>&1 | tail -3
log "Binary ready: target/release/openswarm-connector"

# ── Step 2: Start 20 connectors ──────────────────────────────────────────────

log "Starting 20 connectors (one per agent)..."
bash "$ROOT_DIR/tests/e2e/start_connectors.sh" 20

# ── Step 3: Wait for first connector to be healthy ───────────────────────────

log "Waiting for connector health on port 9370..."
for i in $(seq 1 30); do
    if echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"h","signature":""}' \
       | nc -w 3 127.0.0.1 9370 >/dev/null 2>&1; then
        log "Connector 1 healthy on port 9370"
        break
    fi
    sleep 1
done

# Verify web console is up too
log "Waiting for web console on http://127.0.0.1:9371/api/health..."
for i in $(seq 1 30); do
    if curl -sf "http://127.0.0.1:9371/api/health" >/dev/null 2>&1; then
        log "Web console healthy: http://127.0.0.1:9371/"
        break
    fi
    sleep 1
done

# ── Step 4: Start 20 agents (one per connector port) ─────────────────────────

mkdir -p "$AGENT_LOG_DIR"
log "Starting 20 phase3 agents..."

PORT=9370
for i in $(seq 1 20); do
    NAME="agent3-${PORT}"
    LOG_FILE="$AGENT_LOG_DIR/${NAME}.log"
    python3 "$ROOT_DIR/tests/e2e/phase3_agent.py" "$PORT" "$NAME" \
        > "$LOG_FILE" 2>&1 &
    LAST_PID=$!
    AGENT_PIDS+=("$LAST_PID")
    log "  Started $NAME (pid=$LAST_PID) -> $LOG_FILE"
    PORT=$((PORT + 2))
done

log "All 20 agents started."

# ── Step 5: Wait for tier stabilization ──────────────────────────────────────
#
# KeepAlive propagation takes ~20s. Phase3 agents require 5 polls (25s) of
# stable tier before acting. We wait a total of 40s to let the swarm form
# before Playwright injects the task.

log "Waiting 40s for tier stabilization across all 20 nodes..."
sleep 40

# Quick sanity: show tier distribution
log "Tier distribution snapshot:"
for rpc_port in 9370 9372 9374 9376; do
    resp=$(echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"t","signature":""}' \
           | nc -w 3 127.0.0.1 "$rpc_port" 2>/dev/null || echo '{}')
    tier=$(echo "$resp" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('result',{}).get('tier','?'))" 2>/dev/null || echo "?")
    known=$(echo "$resp" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('result',{}).get('known_agents','?'))" 2>/dev/null || echo "?")
    log "  port $rpc_port: tier=$tier known_agents=$known"
done

# ── Step 6: Run Playwright spec ───────────────────────────────────────────────

log "Installing Playwright deps..."
cd "$ROOT_DIR/tests/playwright"
npm install --quiet 2>/dev/null
npx playwright install chromium >/dev/null 2>&1

log "Running Playwright phase3 holonic E2E against http://127.0.0.1:9371/"
export WEB_BASE_URL="http://127.0.0.1:9371"
PLAYWRIGHT_HEADED=0

# Capture exit code without aborting the script (we want to show the report)
set +e
npx playwright test phase3-holonic-e2e.spec.js \
    --workers=1 \
    --reporter=list \
    --retries=0 \
    --timeout=720000
PW_EXIT=$?
set -e

# ── Step 7: Show report ───────────────────────────────────────────────────────

REPORT_FILE="/tmp/asip-test/phase3-e2e-report.txt"
if [[ -f "$REPORT_FILE" ]]; then
    echo ""
    echo "══════════════════════════════════════════════════════════════════════════"
    echo "  PHASE 3 E2E REPORT"
    echo "══════════════════════════════════════════════════════════════════════════"
    cat "$REPORT_FILE"
else
    log "WARNING: report file not found at $REPORT_FILE"
fi

# ── Step 8: Show agent logs on failure ────────────────────────────────────────

if [[ $PW_EXIT -ne 0 ]]; then
    log "Playwright test failed (exit=$PW_EXIT). Dumping coordinator logs..."
    for f in "$AGENT_LOG_DIR"/agent3-937*.log; do
        if [[ -f "$f" ]]; then
            echo "── $f ──"
            cat "$f"
        fi
    done
fi

# ── Done ─────────────────────────────────────────────────────────────────────

if [[ $PW_EXIT -eq 0 ]]; then
    log "Phase 3 Playwright holonic E2E PASSED ✓"
else
    log "Phase 3 Playwright holonic E2E FAILED (exit=$PW_EXIT)"
fi

exit $PW_EXIT
