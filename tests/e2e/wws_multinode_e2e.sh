#!/usr/bin/env bash
# WWS E2E: Multi-node peer discovery and network tests.
# Starts 3 connectors, verifies peer discovery and network state.
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BINARY="${BINARY:-$ROOT_DIR/target/release/wws-connector}"
SWARM_DIR="/tmp/wws-e2e-multi"
PASS=0; FAIL=0

log()  { echo "[wws-multinode] $*"; }
pass() { echo "  PASS  $1"; PASS=$((PASS+1)); }
fail() { echo "  FAIL  $1 — got: $2"; FAIL=$((FAIL+1)); }
jfield() { python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('$1',''))" 2>/dev/null; }

cleanup() { kill "${N1_PID:-}" "${N2_PID:-}" "${N3_PID:-}" 2>/dev/null || true; sleep 1; }
trap cleanup EXIT

[ -f "$BINARY" ] || { echo "FATAL: $BINARY not found"; exit 1; }
mkdir -p "$SWARM_DIR"
pkill -f "wws-connector" 2>/dev/null || true; sleep 1

wait_http() {
    local port=$1
    for i in $(seq 1 15); do
        sleep 1
        curl -sf "http://127.0.0.1:$port/api/health" >/dev/null 2>&1 && return 0
    done
    echo "FATAL: port $port never became healthy" >&2; exit 1
}

# ── Start node 1 (bootstrap) ──────────────────────────────────────────────────
log "Starting node 1 (bootstrap)..."
"$BINARY" \
    --listen      /ip4/127.0.0.1/tcp/19001 \
    --rpc         127.0.0.1:19370 \
    --files-addr  127.0.0.1:19371 \
    --agent-name  wws-multi-1 \
    --identity-path "$SWARM_DIR/n1.key" \
    >"$SWARM_DIR/n1.log" 2>&1 &
N1_PID=$!
wait_http 19371
log "Node 1 healthy"

# Extract full peer_id from DID (format: did:swarm:<full_peer_id>)
# The peer_id field in the API response is truncated; DID contains the full value.
N1_IDENTITY=$(curl -sf http://127.0.0.1:19371/api/identity)
N1_DID=$(echo "$N1_IDENTITY" | jfield did)
N1_FULL_PEER=$(echo "$N1_DID" | python3 -c "import sys; s=sys.stdin.read().strip(); print(s.replace('did:swarm:',''))")
[ -n "$N1_FULL_PEER" ] || { echo "FATAL: couldn't get node 1 peer_id from DID"; exit 1; }
BOOTSTRAP="/ip4/127.0.0.1/tcp/19001/p2p/$N1_FULL_PEER"
log "Bootstrap: $BOOTSTRAP"

# ── Start node 2 ──────────────────────────────────────────────────────────────
log "Starting node 2..."
"$BINARY" \
    --listen      /ip4/127.0.0.1/tcp/19002 \
    --rpc         127.0.0.1:19372 \
    --files-addr  127.0.0.1:19373 \
    --agent-name  wws-multi-2 \
    --identity-path "$SWARM_DIR/n2.key" \
    --bootstrap   "$BOOTSTRAP" \
    >"$SWARM_DIR/n2.log" 2>&1 &
N2_PID=$!
wait_http 19373
N2_DID=$(curl -sf http://127.0.0.1:19373/api/identity | jfield did)

# ── Start node 3 ──────────────────────────────────────────────────────────────
log "Starting node 3..."
"$BINARY" \
    --listen      /ip4/127.0.0.1/tcp/19003 \
    --rpc         127.0.0.1:19374 \
    --files-addr  127.0.0.1:19375 \
    --agent-name  wws-multi-3 \
    --identity-path "$SWARM_DIR/n3.key" \
    --bootstrap   "$BOOTSTRAP" \
    >"$SWARM_DIR/n3.log" 2>&1 &
N3_PID=$!
wait_http 19375
N3_DID=$(curl -sf http://127.0.0.1:19375/api/identity | jfield did)

log "All 3 nodes up. Waiting 30s for peer discovery..."
sleep 30

# ── Assert 1: DIDs are unique ─────────────────────────────────────────────────
log ""
log "=== 1. Unique identities ==="
[ -n "$N1_DID" ] && [ -n "$N2_DID" ] && [ -n "$N3_DID" ] \
    && pass "all 3 nodes have non-empty DIDs" || fail "all 3 DIDs non-empty" "$N1_DID|$N2_DID|$N3_DID"
[ "$N1_DID" != "$N2_DID" ] \
    && pass "N1 DID != N2 DID" || fail "N1 DID != N2 DID" "collision"
[ "$N1_DID" != "$N3_DID" ] \
    && pass "N1 DID != N3 DID" || fail "N1 DID != N3 DID" "collision"

# ── Assert 2: Node 1 sees all 3 peers (includes self) ────────────────────────
log ""
log "=== 2. Peer discovery (node 1) ==="

NET1=$(curl -sf http://127.0.0.1:19371/api/network)
PEER_COUNT1=$(echo "$NET1" | jfield peer_count)

# peer_count includes self; a 3-node cluster should show peer_count >= 3
if python3 -c "import sys; sys.exit(0 if int('${PEER_COUNT1:-0}') >= 3 else 1)" 2>/dev/null; then
    pass "node 1 peer_count >= 3 (got $PEER_COUNT1)"
else
    fail "node 1 peer_count >= 3" "$PEER_COUNT1"
fi

# bootstrap_connected should be true on nodes that connected to bootstrap
BOOTSTRAP_CONN1=$(echo "$NET1" | jfield bootstrap_connected)
[ "$BOOTSTRAP_CONN1" = "True" ] || [ "$BOOTSTRAP_CONN1" = "true" ] \
    && pass "node 1 bootstrap_connected=true" \
    || fail "node 1 bootstrap_connected=true" "$BOOTSTRAP_CONN1"

# /api/peers lists all known peers including self; expect >= 3 entries
PEERS_RESP=$(curl -sf http://127.0.0.1:19371/api/peers)
PEERS_COUNT=$(python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(len(d.get('peers',[])))" <<< "$PEERS_RESP" 2>/dev/null || echo 0)
if python3 -c "import sys; sys.exit(0 if int('${PEERS_COUNT:-0}') >= 3 else 1)" 2>/dev/null; then
    pass "node 1 /api/peers lists >= 3 entries (got $PEERS_COUNT)"
else
    fail "node 1 /api/peers lists >= 3 entries" "$PEERS_COUNT"
fi

# ── Assert 3: Non-bootstrap node 2 has peers ─────────────────────────────────
log ""
log "=== 3. Peer discovery (node 2 - non-bootstrap) ==="

NET2=$(curl -sf http://127.0.0.1:19373/api/network)
PEER_COUNT2=$(echo "$NET2" | jfield peer_count)
# Node 2 should see at least itself + node 1 (bootstrap)
if python3 -c "import sys; sys.exit(0 if int('${PEER_COUNT2:-0}') >= 2 else 1)" 2>/dev/null; then
    pass "node 2 peer_count >= 2 (got $PEER_COUNT2)"
else
    fail "node 2 peer_count >= 2" "$PEER_COUNT2"
fi

BOOTSTRAP_CONN2=$(echo "$NET2" | jfield bootstrap_connected)
[ "$BOOTSTRAP_CONN2" = "True" ] || [ "$BOOTSTRAP_CONN2" = "true" ] \
    && pass "node 2 bootstrap_connected=true" \
    || fail "node 2 bootstrap_connected=true" "$BOOTSTRAP_CONN2"

# ── Assert 4: /api/peers fields are populated ─────────────────────────────────
log ""
log "=== 4. Peers list fields ==="

# Use first non-self peer from node 1's peer list for field validation
FIRST_PEER=$(python3 -c "
import json,sys
d=json.loads(sys.stdin.read())
peers=d.get('peers',[])
print(json.dumps(peers[0])) if peers else print('{}')
" <<< "$PEERS_RESP" 2>/dev/null || echo "{}")

PEER_DID=$(echo "$FIRST_PEER" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('did',''))" 2>/dev/null || echo "")
PEER_TIER=$(echo "$FIRST_PEER" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('tier',''))" 2>/dev/null || echo "")
PEER_ONLINE=$(echo "$FIRST_PEER" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('online',''))" 2>/dev/null || echo "")
PEER_NAME=$(echo "$FIRST_PEER" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('name',''))" 2>/dev/null || echo "")

[ -n "$PEER_DID" ]    && pass "peer has non-empty did"    || fail "peer has non-empty did" "(empty)"
[ -n "$PEER_TIER" ]   && pass "peer has tier field"       || fail "peer has tier field" "(empty)"
[ -n "$PEER_NAME" ]   && pass "peer has name field"       || fail "peer has name field" "(empty)"
# online=True (Python bool → string) for recently seen peers
[ "$PEER_ONLINE" = "True" ] || [ "$PEER_ONLINE" = "true" ] \
    && pass "peer is online" || fail "peer is online" "$PEER_ONLINE"

# ── Assert 5: swarm_size_estimate reflects full cluster ───────────────────────
log ""
log "=== 5. Swarm size estimate ==="
SWARM_EST=$(echo "$NET1" | jfield swarm_size_estimate)
if python3 -c "import sys; sys.exit(0 if int('${SWARM_EST:-0}') >= 3 else 1)" 2>/dev/null; then
    pass "node 1 swarm_size_estimate >= 3 (got $SWARM_EST)"
else
    fail "node 1 swarm_size_estimate >= 3" "$SWARM_EST"
fi

log ""
log "=== Results: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ]
