#!/usr/bin/env bash
# Start N ASIP connector nodes without agents.
# Usage: bash tests/e2e/start_connectors.sh N
# Output: /tmp/asip-test/nodes.txt  (name|pid|rpc_port|files_port per line)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
BIN="$ROOT_DIR/target/release/openswarm-connector"
SWARM_DIR="/tmp/asip-test"
NODES_FILE="$SWARM_DIR/nodes.txt"

N="${1:-1}"

if [[ ! -f "$BIN" ]]; then
    echo "[start_connectors] Binary not found: $BIN"
    echo "[start_connectors] Run: ~/.cargo/bin/cargo build --release"
    exit 1
fi

mkdir -p "$SWARM_DIR"
> "$NODES_FILE"

# Kill any leftover connectors
pkill -f 'openswarm-connector' 2>/dev/null || true
sleep 1

BOOTSTRAP_ADDR=""

for i in $(seq 1 "$N"); do
    NAME="asip-node-$i"
    P2P_PORT=$((9000 + i))
    RPC_PORT=$((9368 + i * 2))    # 9370, 9372, 9374 ...
    FILES_PORT=$((9369 + i * 2))  # 9371, 9373, 9375 ...
    LOG="$SWARM_DIR/$NAME.log"

    CMD=("$BIN"
        "--listen"     "/ip4/127.0.0.1/tcp/$P2P_PORT"
        "--rpc"        "127.0.0.1:$RPC_PORT"
        "--files-addr" "127.0.0.1:$FILES_PORT"
        "--agent-name" "$NAME"
    )
    if [[ -n "$BOOTSTRAP_ADDR" ]]; then
        CMD+=("--bootstrap" "$BOOTSTRAP_ADDR")
    fi

    "${CMD[@]}" >"$LOG" 2>&1 &
    PID=$!
    echo "$NAME|$PID|$RPC_PORT|$FILES_PORT" >> "$NODES_FILE"
    echo "  Started $NAME  pid=$PID  rpc=$RPC_PORT  files=$FILES_PORT"

    sleep 2

    # Node 1: extract peer id for bootstrap address
    if [[ $i -eq 1 ]]; then
        for _try in 1 2 3 4 5; do
            PEER_ID=$(echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"s","signature":""}' \
                | nc -w 5 127.0.0.1 "$RPC_PORT" 2>/dev/null \
                | python3 -c "
import json,sys
raw=sys.stdin.read().strip()
if not raw: exit(1)
d=json.loads(raw)
aid=d.get('result',{}).get('agent_id','')
# agent_id is did:swarm:<peer_id>
print(aid.replace('did:swarm:',''))
" 2>/dev/null || echo "")
            if [[ -n "$PEER_ID" ]]; then
                BOOTSTRAP_ADDR="/ip4/127.0.0.1/tcp/$P2P_PORT/p2p/$PEER_ID"
                echo "  Bootstrap: $BOOTSTRAP_ADDR"
                break
            fi
            sleep 1
        done
    else
        # Explicitly connect to bootstrap node
        if [[ -n "$BOOTSTRAP_ADDR" ]]; then
            echo '{"jsonrpc":"2.0","method":"swarm.connect","params":{"addr":"'"$BOOTSTRAP_ADDR"'"},"id":"c","signature":""}' \
                | nc -w 5 127.0.0.1 "$RPC_PORT" >/dev/null 2>&1 || true
        fi
    fi
done

echo ""
echo "=== $N connector(s) started ==="
echo "Nodes file: $NODES_FILE"
cat "$NODES_FILE"
echo ""
echo "Node 1  RPC:   127.0.0.1:9370"
echo "Node 1  HTTP:  http://127.0.0.1:9371/"
