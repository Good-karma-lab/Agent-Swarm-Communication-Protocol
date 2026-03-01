#!/usr/bin/env bash
# WWS E2E: Single-node tests for identity, reputation, keys, network, names.
# Usage: bash tests/e2e/wws_e2e.sh
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BINARY="${BINARY:-$ROOT_DIR/target/release/wws-connector}"
SWARM_DIR="/tmp/wws-e2e-single"
PASS=0; FAIL=0

log()  { echo "[wws-e2e] $*"; }
pass() { echo "  PASS  $1"; PASS=$((PASS+1)); }
fail() { echo "  FAIL  $1 — got: $2"; FAIL=$((FAIL+1)); }

check() {
    local label="$1" expr="$2" want="$3"
    local got
    got=$(eval "$expr" 2>/dev/null || echo "")
    if [ "$got" = "$want" ]; then
        pass "$label"
    else
        fail "$label" "$got (expected: $want)"
    fi
}

# JSON helper: extract a field from a JSON response
jfield() { python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('$1',''))" 2>/dev/null; }

cleanup() {
    kill -9 "${PID1:-}" "${PID2:-}" 2>/dev/null || true
    sleep 1
}
trap cleanup EXIT

[ -f "$BINARY" ] || { echo "FATAL: binary not found: $BINARY"; exit 1; }

mkdir -p "$SWARM_DIR"

# Kill any stale connectors occupying the test ports before starting
pkill -9 -f "wws-connector.*19371" 2>/dev/null || true
pkill -9 -f "wws-connector.*19370" 2>/dev/null || true
pkill -9 -f "wws-connector.*19001" 2>/dev/null || true
sleep 2

# ── Test 1: Identity persistence ─────────────────────────────────────────────
log ""
log "=== 1. Identity Persistence ==="

IDENTITY_FILE="$SWARM_DIR/alice.key"
rm -f "$IDENTITY_FILE"

# Start connector, get DID, stop
"$BINARY" \
    --listen      /ip4/127.0.0.1/tcp/19001 \
    --rpc         127.0.0.1:19370 \
    --files-addr  127.0.0.1:19371 \
    --agent-name  wws-test-alice \
    --identity-path "$IDENTITY_FILE" \
    >"$SWARM_DIR/alice1.log" 2>&1 &
PID1=$!

for i in $(seq 1 10); do
    sleep 1
    if curl -sf http://127.0.0.1:19371/api/health >/dev/null 2>&1; then break; fi
done

DID1=$(curl -sf http://127.0.0.1:19371/api/identity | jfield did)
PEER_ID1=$(curl -sf http://127.0.0.1:19371/api/identity | jfield peer_id)
kill "$PID1" 2>/dev/null
sleep 1
kill -9 "$PID1" 2>/dev/null || true
# Wait for port 19371 to be released (kill any stragglers)
for i in $(seq 1 8); do
    sleep 1
    if ! lsof -i :19371 >/dev/null 2>&1; then break; fi
    # Forcibly kill any process still holding the port
    lsof -ti :19371 2>/dev/null | xargs kill -9 2>/dev/null || true
done

[ -f "$IDENTITY_FILE" ] && pass "identity key file created" || fail "identity key file created" "file not found"
[ -n "$DID1" ]          && pass "DID is non-empty"          || fail "DID is non-empty" "(empty)"

# Capture key file bytes before restart
KEY_BYTES_BEFORE=$(xxd "$IDENTITY_FILE" 2>/dev/null | md5sum 2>/dev/null | cut -d' ' -f1 || md5 "$IDENTITY_FILE" 2>/dev/null | awk '{print $NF}' || echo "")

# Restart with same identity-path, check key file is preserved
"$BINARY" \
    --listen      /ip4/127.0.0.1/tcp/19001 \
    --rpc         127.0.0.1:19370 \
    --files-addr  127.0.0.1:19371 \
    --agent-name  wws-test-alice \
    --identity-path "$IDENTITY_FILE" \
    >"$SWARM_DIR/alice2.log" 2>&1 &
PID1=$!

for i in $(seq 1 10); do
    sleep 1
    if curl -sf http://127.0.0.1:19371/api/health >/dev/null 2>&1; then break; fi
done

DID2=$(curl -sf http://127.0.0.1:19371/api/identity | jfield did)
KEY_BYTES_AFTER=$(xxd "$IDENTITY_FILE" 2>/dev/null | md5sum 2>/dev/null | cut -d' ' -f1 || md5 "$IDENTITY_FILE" 2>/dev/null | awk '{print $NF}' || echo "")

# The key file bytes must be unchanged across restarts (key reuse, not regenerated)
if [ -n "$KEY_BYTES_BEFORE" ] && [ "$KEY_BYTES_BEFORE" = "$KEY_BYTES_AFTER" ]; then
    pass "identity key file bytes preserved across restart"
else
    fail "identity key file bytes preserved across restart" "before=$KEY_BYTES_BEFORE after=$KEY_BYTES_AFTER"
fi

# DID should be non-empty on second start
[ -n "$DID2" ] && pass "DID non-empty on restart" || fail "DID non-empty on restart" "(empty)"

# ── Test 2: Reputation ────────────────────────────────────────────────────────
log ""
log "=== 2. Reputation (fresh node) ==="

REP=$(curl -sf http://127.0.0.1:19371/api/reputation)
REP_SCORE=$(echo "$REP" | jfield score)
REP_TIER=$(echo "$REP" | jfield tier)
REP_NEXT=$(echo "$REP" | jfield next_tier_at)

[ "$REP_SCORE" = "10" ]       && pass "reputation score = 10"         || fail "reputation score = 10" "$REP_SCORE"
[ "$REP_TIER" = "newcomer" ]  && pass "reputation tier = newcomer"    || fail "reputation tier = newcomer" "$REP_TIER"
[ "$REP_NEXT" = "15" ]        && pass "reputation next_tier_at = 15"  || fail "reputation next_tier_at = 15" "$REP_NEXT"

# ── Test 3: Keys ──────────────────────────────────────────────────────────────
log ""
log "=== 3. Keys endpoint ==="

KEYS=$(curl -sf http://127.0.0.1:19371/api/keys)
KEYS_DID=$(echo "$KEYS" | jfield did)
KEYS_TYPE=$(echo "$KEYS" | jfield key_type)

[ "$KEYS_TYPE" = "Ed25519" ]   && pass "key_type = Ed25519"           || fail "key_type = Ed25519" "$KEYS_TYPE"
[ "$KEYS_DID" = "$DID2" ]      && pass "keys DID matches identity DID" || fail "keys DID matches identity DID" "$KEYS_DID vs $DID2"

# ── Test 4: Network (isolated, no peers) ─────────────────────────────────────
log ""
log "=== 4. Network (isolated node) ==="

NET=$(curl -sf http://127.0.0.1:19371/api/network)
NET_PEERS=$(echo "$NET" | jfield peer_count)
# peer_count may be 0 or 1 (node may count its own bootstrap entry); check endpoint responds
[ -n "$NET_PEERS" ]  && pass "network endpoint returns peer_count ($NET_PEERS)"  || fail "network endpoint returns peer_count" "(empty)"

# ── Test 5: Name registration and lifecycle ───────────────────────────────────
log ""
log "=== 5. Name Registry ==="

# 5a. Register a name via HTTP
REG=$(curl -sf -X POST -H 'Content-Type: application/json' \
    -d '{"name":"wws-test-alice"}' http://127.0.0.1:19371/api/names)
REG_OK=$(echo "$REG" | jfield ok)
[ "$REG_OK" = "True" ] || [ "$REG_OK" = "true" ] \
    && pass "POST /api/names succeeds" || fail "POST /api/names succeeds" "$REG"

# 5b. List names
NAMES_RESP=$(curl -sf http://127.0.0.1:19371/api/names)
NAME_COUNT=$(python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(len(d.get('names',[])))" <<< "$NAMES_RESP" 2>/dev/null || echo 0)
[ "$NAME_COUNT" = "1" ]  && pass "GET /api/names returns 1 name" || fail "GET /api/names returns 1 name" "$NAME_COUNT"

# 5c. Resolve via RPC
RESOLVE=$(echo '{"jsonrpc":"2.0","method":"swarm.resolve_name","params":{"name":"wws-test-alice"},"id":"1","signature":""}' \
    | nc -w 5 127.0.0.1 19370)
RESOLVE_DID=$(echo "$RESOLVE" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('result',{}).get('did',''))" 2>/dev/null || echo "")
[ "$RESOLVE_DID" = "$DID2" ]  && pass "RPC resolve_name returns correct DID" || fail "RPC resolve_name returns correct DID" "$RESOLVE_DID"

# 5d. Duplicate registration → name_taken (returns HTTP 409; use curl -s not -sf to capture body)
DUP=$(curl -s -X POST -H 'Content-Type: application/json' \
    -d '{"name":"wws-test-alice"}' http://127.0.0.1:19371/api/names)
DUP_ERR=$(echo "$DUP" | jfield error)
[ "$DUP_ERR" = "name_taken" ]  && pass "duplicate name → name_taken" || fail "duplicate name → name_taken" "$DUP_ERR"

# 5e. Delete name
DEL=$(curl -sf -X DELETE http://127.0.0.1:19371/api/names/wws-test-alice)
DEL_OK=$(echo "$DEL" | jfield ok)
[ "$DEL_OK" = "True" ] || [ "$DEL_OK" = "true" ] \
    && pass "DELETE /api/names succeeds" || fail "DELETE /api/names succeeds" "$DEL"

# 5f. After delete: names list is empty
NAMES_AFTER=$(curl -sf http://127.0.0.1:19371/api/names)
COUNT_AFTER=$(python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(len(d.get('names',[])))" <<< "$NAMES_AFTER" 2>/dev/null || echo -1)
[ "$COUNT_AFTER" = "0" ]  && pass "names list empty after delete" || fail "names list empty after delete" "$COUNT_AFTER"

# 5g. RPC resolve after delete → error
RESOLVE_GONE=$(echo '{"jsonrpc":"2.0","method":"swarm.resolve_name","params":{"name":"wws-test-alice"},"id":"2","signature":""}' \
    | nc -w 5 127.0.0.1 19370)
RESOLVE_GONE_ERR=$(echo "$RESOLVE_GONE" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print('error' if d.get('error') else 'ok')" 2>/dev/null || echo "?")
[ "$RESOLVE_GONE_ERR" = "error" ]  && pass "resolve after delete returns error" || fail "resolve after delete returns error" "$RESOLVE_GONE_ERR"

kill "${PID1:-}" 2>/dev/null || true

# ── Summary ───────────────────────────────────────────────────────────────────
log ""
log "=== Results: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ]
