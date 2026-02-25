#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$ROOT_DIR"

cleanup() {
    ./swarm-manager.sh stop >/dev/null 2>&1 || true
}
trap cleanup EXIT

./swarm-manager.sh stop >/dev/null 2>&1 || true
./swarm-manager.sh start 5 >/dev/null
sleep 20

RPC1=$(awk -F'|' 'NR==1 {print $4}' /tmp/openswarm-swarm/nodes.txt)
RPC2=$(awk -F'|' 'NR==2 {print $4}' /tmp/openswarm-swarm/nodes.txt)
PID2=$(awk -F'|' 'NR==2 {print $2}' /tmp/openswarm-swarm/nodes.txt)

echo '{"jsonrpc":"2.0","method":"swarm.inject_task","params":{"description":"fault test before leader kill"},"id":"inj1","signature":""}' | nc 127.0.0.1 "$RPC1" >/dev/null

# Leader/follower failure simulation: kill one connector.
kill "$PID2" >/dev/null 2>&1 || true
sleep 5

echo '{"jsonrpc":"2.0","method":"swarm.inject_task","params":{"description":"fault test after node kill"},"id":"inj2","signature":""}' | nc 127.0.0.1 "$RPC1" >/dev/null
sleep 8

STATUS=$(echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"status","signature":""}' | nc 127.0.0.1 "$RPC1")
if [[ "$STATUS" != *'"result"'* ]]; then
    echo "Fault injection FAILED: bootstrap node did not respond"
    exit 1
fi

# Reconnect storm simulation.
for _ in $(seq 1 10); do
    ADDR=$(echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"s","signature":""}' | nc 127.0.0.1 "$RPC1" | python3 -c 'import json,sys; d=json.loads(sys.stdin.read()); print(d["result"]["agent_id"].replace("did:swarm:",""))')
    echo "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.connect\",\"params\":{\"addr\":\"/ip4/127.0.0.1/tcp/9000/p2p/$ADDR\"},\"id\":\"c\",\"signature\":\"\"}" | nc 127.0.0.1 "$RPC1" >/dev/null || true
done

echo "Fault injection E2E PASSED"
