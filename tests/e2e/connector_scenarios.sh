#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

TMP_DIR="$(mktemp -d /tmp/wws-e2e-scenarios.XXXXXX)"
PIDS=()

cleanup() {
    local pid
    for pid in "${PIDS[@]}"; do
        kill "$pid" 2>/dev/null || true
    done
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

start_node() {
    local name="$1"
    local p2p="$2"
    local rpc="$3"
    local files="$4"
    local bootstrap="${5:-}"
    local extra="${6:-}"
    local pid
    pid=$(start_connector "$name" "$p2p" "$rpc" "$files" "$TMP_DIR/$name.log" "$bootstrap" "$extra")
    PIDS+=("$pid")
    wait_for_rpc "$rpc"
}

register_agent() {
    local rpc="$1"
    local agent="$2"
    rpc_call "$rpc" "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.register_agent\",\"params\":{\"agent_id\":\"$agent\"},\"id\":\"reg-$agent\",\"signature\":\"\"}" >/dev/null
}

cd "$ROOT_DIR"
log "Building release binary"
cargo build --release -p wws-connector >/dev/null

# Scenario 1: connection + bootstrap discovery (internet-style)
log "Scenario 1: connection and internet-style autodiscovery"
start_node "s1-a" 20000 20370 20371

STATUS_A=$(rpc_call 20370 '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"status-a","signature":""}')
PEER_A=$(json_get "$STATUS_A" "result.agent_id" | sed 's/^did:swarm://')
BOOT_A="/ip4/127.0.0.1/tcp/20000/p2p/$PEER_A"

start_node "s1-b" 20001 20372 20373 "$BOOT_A"
CONNECT_RES=$(rpc_call 20372 "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.connect\",\"params\":{\"addr\":\"$BOOT_A\"},\"id\":\"connect\",\"signature\":\"\"}")
CONNECTED=$(json_get "$CONNECT_RES" "result.connected")
[[ "$CONNECTED" == "True" || "$CONNECTED" == "true" ]]

# Scenario 2: local autodiscovery smoke (mDNS default enabled)
log "Scenario 2: local autodiscovery smoke"
start_node "s2-a" 20100 20470 20471
start_node "s2-b" 20101 20472 20473
sleep 5

# Scenario 3: voting + decomposition + distribution + result propagation
log "Scenario 3: voting, decomposition, distribution, results propagation"
register_agent 20370 "agent-a"
register_agent 20372 "agent-b"
register_agent 20470 "agent-c"
register_agent 20472 "agent-d"

INJECT=$(rpc_call 20370 '{"jsonrpc":"2.0","method":"swarm.inject_task","params":{"description":"E2E validation task"},"id":"inject","signature":""}')
TASK_ID=$(json_get "$INJECT" "result.task_id")

PLAN_PAYLOAD=$(python3 - <<'PY' "$TASK_ID"
import json, sys
task_id = sys.argv[1]
print(json.dumps({
  "jsonrpc": "2.0",
  "method": "swarm.propose_plan",
  "id": "plan",
  "signature": "",
  "params": {
    "plan_id": "plan-e2e-1",
    "task_id": task_id,
    "proposer": "agent-a",
    "epoch": 1,
    "subtasks": [
      {"index": 0, "description": "subtask alpha", "required_capabilities": ["analysis"], "estimated_complexity": 0.4},
      {"index": 1, "description": "subtask beta", "required_capabilities": ["analysis"], "estimated_complexity": 0.6}
    ],
    "rationale": "parallel decomposition",
    "estimated_parallelism": 2,
    "created_at": "2026-01-01T00:00:00Z"
  }
}))
PY
)

rpc_call 20370 "$PLAN_PAYLOAD" >/dev/null
sleep 2

TIMELINE=$(rpc_call 20370 "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.get_task_timeline\",\"params\":{\"task_id\":\"$TASK_ID\"},\"id\":\"timeline\",\"signature\":\"\"}")
EVENTS=$(json_get "$TIMELINE" "result.events")

if [[ "$EVENTS" != *"proposed"* || "$EVENTS" != *"subtask_created"* || "$EVENTS" != *"published"* ]]; then
    echo "Connector scenarios FAILED: expected timeline stages missing"
    echo "$TIMELINE"
    exit 1
fi

# Scenario 4: peer-to-peer messaging (existing network integration test)
log "Scenario 4: peer-to-peer messaging"
cargo test -p wws-connector --test multi_agent_tests test_same_tier_task_topic_exchange_between_peers -- --ignored >/dev/null

# Scenario 5: scaling smoke
log "Scenario 5: scaling smoke"
cargo test -p wws-connector --test multi_agent_tests test_three_node_network -- --ignored >/dev/null

log "Connector scenario suite PASSED"
