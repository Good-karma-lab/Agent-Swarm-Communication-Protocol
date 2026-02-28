#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

if [ -f "$ROOT_DIR/scripts/load-env.sh" ]; then
    # shellcheck disable=SC1091
    source "$ROOT_DIR/scripts/load-env.sh"
fi

if [[ -z "${OPENROUTER_API_KEY:-}" ]]; then
    echo "OPENROUTER_API_KEY is required for live ZeroClaw + OpenRouter E2E"
    exit 2
fi

export AGENT_IMPL=zeroclaw
export LLM_BACKEND=openrouter
export MODEL_NAME="${MODEL_NAME:-arcee-ai/trinity-large-preview:free}"
export ZEROCLAW_AUTO_UPDATE="${ZEROCLAW_AUTO_UPDATE:-true}"

cd "$ROOT_DIR"

cleanup() {
    ./swarm-manager.sh stop >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "[live-e2e] Starting 5 agents with ZeroClaw + OpenRouter"
./swarm-manager.sh start-agents 5

sleep 20

RPC_PORT=$(awk -F'|' 'NR==1 {print $5}' /tmp/wws-swarm/nodes.txt)
if [[ -z "$RPC_PORT" ]]; then
    echo "[live-e2e] Failed to read RPC port from nodes file"
    exit 1
fi

echo "[live-e2e] Injecting task"
INJECT=$(echo '{"jsonrpc":"2.0","method":"swarm.inject_task","params":{"description":"Live E2E task for OpenRouter swarm: decompose, vote, execute, aggregate"},"id":"live-inject","signature":""}' | nc 127.0.0.1 "$RPC_PORT")
TASK_ID=$(python3 - <<'PY' "$INJECT"
import json, sys
doc = json.loads(sys.argv[1])
print(doc.get("result", {}).get("task_id", ""))
PY
)

if [[ -z "$TASK_ID" ]]; then
    echo "[live-e2e] Failed to inject task"
    echo "$INJECT"
    exit 1
fi

echo "[live-e2e] Waiting for end-to-end workflow"
for _ in $(seq 1 36); do
    TIMELINE=$(echo "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.get_task_timeline\",\"params\":{\"task_id\":\"$TASK_ID\"},\"id\":\"live-tl\",\"signature\":\"\"}" | nc 127.0.0.1 "$RPC_PORT")
    if echo "$TIMELINE" | grep -q 'plan_selected' && echo "$TIMELINE" | grep -q 'subtask_assigned' && echo "$TIMELINE" | grep -q 'result_submitted'; then
        echo "[live-e2e] Live ZeroClaw + OpenRouter E2E PASSED"
        exit 0
    fi
    sleep 10
done

echo "[live-e2e] Live E2E FAILED: timeline stages not reached in time"
echo "$TIMELINE"
exit 1
