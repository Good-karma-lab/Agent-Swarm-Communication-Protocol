# WWS Features End-to-End Test Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Write and run a 3-layer E2E test suite that exercises all WWS-specific features — persistent identity, name registry, reputation, keys, peer discovery, and UI panels — against real running connectors.

**Architecture:** Three test layers run in sequence: (1) single-node API tests via bash+curl that verify identity persistence, name CRUD, reputation, and keys; (2) multi-node API tests with 3 connectors verifying peer discovery; (3) a Playwright spec exercising all UI panels. A master shell script orchestrates all three and prints a pass/fail summary.

**Tech Stack:** bash + curl (API layer), JSON via `python3 -c "import json,sys; ..."` for parsing, Playwright/JS (UI layer), release wws-connector binary at `target/release/wws-connector`.

---

## Key Architecture Facts (read before implementing)

### CLI flags (correct names)
```bash
wws-connector \
  --listen      /ip4/127.0.0.1/tcp/<P2P_PORT> \
  --rpc         127.0.0.1:<RPC_PORT> \
  --files-addr  127.0.0.1:<HTTP_PORT> \
  --agent-name  <NAME> \
  --identity-path <PATH>   # optional; defaults to ~/.wws/<agent-name>.key
```

**Common mistake:** the old `wws_e2e.sh` used `--rpc-bind-addr` and `--file-server-addr` — these are WRONG. Use `--rpc` and `--files-addr`.

### Port layout (3-connector test)
```
Node 1: P2P=19001  RPC=19370  HTTP=19371
Node 2: P2P=19002  RPC=19372  HTTP=19373
Node 3: P2P=19003  RPC=19374  HTTP=19375
```
(Use 19xxx to avoid colliding with the phase3 E2E on 9xxx.)

### Bootstrap sequence
1. Node 1 starts (no bootstrap flag). Wait for health.
2. Extract its PeerID: `curl -sf http://127.0.0.1:19371/api/identity | python3 -c "import json,sys; print(json.load(sys.stdin)['peer_id'])"`
3. Nodes 2 and 3 start with `--bootstrap /ip4/127.0.0.1/tcp/19001/p2p/<PEER_ID>`.

### RPC wire format
```bash
echo '{"jsonrpc":"2.0","method":"<method>","params":<params>,"id":"1","signature":""}' | nc -w 5 127.0.0.1 <RPC_PORT>
```
Where `<params>` is a JSON object. Example: `{"name":"alice"}`.

### API response shapes (what to assert)
- `GET /api/identity` → `{ did, peer_id, wws_name, tier, key_healthy, uptime_secs }`
- `GET /api/reputation` → `{ score, tier, next_tier_at, positive_total, negative_total, decay }`
  - Fresh node: score=10, tier="newcomer", next_tier_at=15
- `GET /api/keys` → `{ did, pubkey_hex, key_type, guardian_count, threshold, last_rotation }`
  - key_type is always "Ed25519"
- `GET /api/network` → `{ bootstrap_connected, peer_count, swarm_size_estimate, nat_type, current_epoch }`
  - Fresh node with no peers: peer_count=0, bootstrap_connected=false
  - After 2 more nodes join: peer_count>=2 on node 1
- `POST /api/names` body `{ "name": "alice" }` → `{ ok: true }` or `{ ok: false, error: "name_taken" }`
- `GET /api/names` → `{ names: [{ name, did, registered_at, expires_at, ttl_secs }] }`
- `DELETE /api/names/alice` → `{ ok: true }`
- RPC `swarm.resolve_name` params `{"name":"alice"}` → `{ result: { name, did, peer_id, expires_at } }` or error

### Identity persistence mechanism
- `--agent-name alice` makes the connector use `~/.wws/alice.key` as identity file.
- `--identity-path /tmp/test/alice.key` overrides the path explicitly.
- On restart with the same path, `load_or_create_keypair` loads the existing key → same DID.

### UI component structure relevant to tests
- **Header** (`.app-header`): `.brand` text "WWS", `.header-identity` button (shows name + `.tier-badge`), `.header-stats` shows agent/peer counts
- **LeftColumn** (`.col-left`): sections with `.section-header` labels: "My Agent", "Names", "Key Health", "Network"
  - "My Agent": `.identity-name`, `.tier-badge`, `.rep-score` (click → opens Reputation panel), `.id-field` rows for DID and PeerID
  - "Names": lists `.name-row` items, `button.add-name-btn` (text "+ Register name" → opens NameRegistryPanel)
  - "Key Health": rows with `.status-dot`, click → opens KeyManagementPanel
- **NameRegistryPanel**: `input[type="text"]` for name, `button[type="submit"]` to register, shows success/error message
- **KeyManagementPanel**: shows DID, pubkey, key_type. Triggered by clicking "Key Health" header or ⚙ button.
- **ReputationPanel**: shows score, tier, progress bar. Triggered by clicking rep score in LeftColumn.
- **AuditPanel**: triggered by "Audit" button in Header.

---

## Task 1: Rewrite `tests/e2e/wws_e2e.sh` — Single-Node API Tests

**Files:**
- Modify: `tests/e2e/wws_e2e.sh` (full rewrite)

**What this test covers:**
1. Identity persistence (start → get DID → restart same key → same DID)
2. Initial reputation (score=10, tier=newcomer)
3. Keys endpoint (key_type=Ed25519, DID matches identity)
4. Network endpoint (fresh node: peer_count=0)
5. Name registration via HTTP (POST /api/names)
6. Name listing (GET /api/names)
7. Name resolution via RPC (swarm.resolve_name)
8. Name conflict (register same name twice → name_taken)
9. Name deletion (DELETE /api/names/:name)
10. Name not found after deletion (swarm.resolve_name → error)

**Step 1: Write the new wws_e2e.sh**

Write exactly this file at `tests/e2e/wws_e2e.sh`:

```bash
#!/usr/bin/env bash
# WWS E2E: Single-node tests for identity, reputation, keys, network, names.
# Usage: bash tests/e2e/wws_e2e.sh
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BINARY="${BINARY:-$ROOT_DIR/target/release/wws-connector}"
SWARM_DIR="/tmp/wws-e2e-single"
PASS=0; FAIL=0

log()  { echo "[wws-e2e] $*"; }
pass() { echo "  PASS  $1"; ((PASS++)); }
fail() { echo "  FAIL  $1 — got: $2"; ((FAIL++)); }

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
    kill "${PID1:-}" "${PID2:-}" 2>/dev/null || true
    sleep 1
}
trap cleanup EXIT

[ -f "$BINARY" ] || { echo "FATAL: binary not found: $BINARY"; exit 1; }

mkdir -p "$SWARM_DIR"

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
kill "$PID1" 2>/dev/null; sleep 2

[ -f "$IDENTITY_FILE" ] && pass "identity key file created" || fail "identity key file created" "file not found"
[ -n "$DID1" ]          && pass "DID is non-empty"          || fail "DID is non-empty" "(empty)"

# Restart with same identity-path, check DID is the same
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

if [ "$DID1" = "$DID2" ] && [ -n "$DID1" ]; then
    pass "DID stable across restarts ($DID1)"
else
    fail "DID stable across restarts" "$DID1 != $DID2"
fi

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
[ "$NET_PEERS" = "0" ]  && pass "peer_count = 0 (isolated)"  || fail "peer_count = 0" "$NET_PEERS"

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

# 5d. Duplicate registration → name_taken
DUP=$(curl -sf -X POST -H 'Content-Type: application/json' \
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
```

**Step 2: Make executable and run**

```bash
chmod +x tests/e2e/wws_e2e.sh
bash tests/e2e/wws_e2e.sh
```

Expected: `=== Results: 14 passed, 0 failed ===`

**Step 3: Fix any failures**

If `jfield ok` returns "True" (Python bool repr) but not "true" (JSON string), adjust the check: the `jfield` helper uses `print(d.get(...))` which stringifies Python `True` as `"True"`. The check uses `[ "$X" = "True" ] || [ "$X" = "true" ]` which handles both. If the JSON key is actually a boolean `true`, Python parses it as `True`.

**Step 4: Commit**

```bash
git add tests/e2e/wws_e2e.sh
git commit -m "test(e2e): rewrite wws_e2e.sh with comprehensive single-node WWS API tests"
```

---

## Task 2: Write `tests/e2e/wws_multinode_e2e.sh` — Multi-Node Peer Discovery

**Files:**
- Create: `tests/e2e/wws_multinode_e2e.sh`

**What this test covers:**
1. 3 connectors start and discover each other
2. Node 1 peer_count >= 2 after all nodes connect
3. GET /api/peers lists peers with name/tier/online fields
4. Each node has a unique DID
5. bootstrap_connected=true on non-bootstrap nodes

**Step 1: Write the script**

```bash
#!/usr/bin/env bash
# WWS E2E: Multi-node peer discovery and network tests.
# Starts 3 connectors, verifies peer discovery and network state.
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BINARY="${BINARY:-$ROOT_DIR/target/release/wws-connector}"
SWARM_DIR="/tmp/wws-e2e-multi"
PASS=0; FAIL=0

log()  { echo "[wws-multinode] $*"; }
pass() { echo "  PASS  $1"; ((PASS++)); }
fail() { echo "  FAIL  $1 — got: $2"; ((FAIL++)); }
jfield() { python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('$1',''))" 2>/dev/null; }

cleanup() { kill "${N1_PID:-}" "${N2_PID:-}" "${N3_PID:-}" 2>/dev/null || true; sleep 1; }
trap cleanup EXIT

[ -f "$BINARY" ] || { echo "FATAL: $BINARY not found"; exit 1; }
mkdir -p "$SWARM_DIR"

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

# Extract peer_id for bootstrap address
N1_PEER=$(curl -sf http://127.0.0.1:19371/api/identity | jfield peer_id)
N1_DID=$(curl -sf http://127.0.0.1:19371/api/identity | jfield did)
[ -n "$N1_PEER" ] || { echo "FATAL: couldn't get node 1 peer_id"; exit 1; }
BOOTSTRAP="/ip4/127.0.0.1/tcp/19001/p2p/$N1_PEER"
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

log "All 3 nodes up. Waiting 20s for peer discovery..."
sleep 20

# ── Assert 1: DIDs are unique ─────────────────────────────────────────────────
log ""
log "=== 1. Unique identities ==="
[ -n "$N1_DID" ] && [ -n "$N2_DID" ] && [ -n "$N3_DID" ] \
    && pass "all 3 nodes have non-empty DIDs" || fail "all 3 DIDs non-empty" "$N1_DID|$N2_DID|$N3_DID"
[ "$N1_DID" != "$N2_DID" ] \
    && pass "N1 DID != N2 DID" || fail "N1 DID != N2 DID" "collision"
[ "$N1_DID" != "$N3_DID" ] \
    && pass "N1 DID != N3 DID" || fail "N1 DID != N3 DID" "collision"

# ── Assert 2: Node 1 sees 2+ peers ───────────────────────────────────────────
log ""
log "=== 2. Peer discovery (node 1) ==="

NET1=$(curl -sf http://127.0.0.1:19371/api/network)
PEER_COUNT1=$(echo "$NET1" | jfield peer_count)
BOOTSTRAP_CONN1=$(echo "$NET1" | jfield bootstrap_connected)

[ "$PEER_COUNT1" -ge 2 ] 2>/dev/null \
    && pass "node 1 peer_count >= 2 (got $PEER_COUNT1)" \
    || fail "node 1 peer_count >= 2" "$PEER_COUNT1"

PEERS_RESP=$(curl -sf http://127.0.0.1:19371/api/peers)
PEERS_COUNT=$(python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(len(d.get('peers',[])))" <<< "$PEERS_RESP" 2>/dev/null || echo 0)
[ "$PEERS_COUNT" -ge 2 ] 2>/dev/null \
    && pass "node 1 /api/peers lists >= 2 entries (got $PEERS_COUNT)" \
    || fail "node 1 /api/peers lists >= 2 entries" "$PEERS_COUNT"

# ── Assert 3: Non-bootstrap node 2 has peers ─────────────────────────────────
log ""
log "=== 3. Peer discovery (node 2 - non-bootstrap) ==="

NET2=$(curl -sf http://127.0.0.1:19373/api/network)
PEER_COUNT2=$(echo "$NET2" | jfield peer_count)
[ "$PEER_COUNT2" -ge 1 ] 2>/dev/null \
    && pass "node 2 peer_count >= 1 (got $PEER_COUNT2)" \
    || fail "node 2 peer_count >= 1" "$PEER_COUNT2"

# ── Assert 4: /api/peers fields are populated ─────────────────────────────────
log ""
log "=== 4. Peers list fields ==="

FIRST_PEER=$(python3 -c "
import json,sys
d=json.loads(sys.stdin.read())
peers=d.get('peers',[])
print(json.dumps(peers[0])) if peers else print('{}')
" <<< "$PEERS_RESP" 2>/dev/null || echo "{}")

PEER_DID=$(echo "$FIRST_PEER" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('did',''))" 2>/dev/null || echo "")
PEER_TIER=$(echo "$FIRST_PEER" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('tier',''))" 2>/dev/null || echo "")
PEER_ONLINE=$(echo "$FIRST_PEER" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('online',''))" 2>/dev/null || echo "")

[ -n "$PEER_DID" ]    && pass "peer has non-empty did"    || fail "peer has non-empty did" "(empty)"
[ -n "$PEER_TIER" ]   && pass "peer has tier field"       || fail "peer has tier field" "(empty)"
# online=True (Python bool → string) for recently seen peers
[ "$PEER_ONLINE" = "True" ] || [ "$PEER_ONLINE" = "true" ] \
    && pass "peer is online" || fail "peer is online" "$PEER_ONLINE"

log ""
log "=== Results: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ]
```

**Step 2: Make executable and run**

```bash
chmod +x tests/e2e/wws_multinode_e2e.sh
bash tests/e2e/wws_multinode_e2e.sh
```

Expected: `=== Results: 10 passed, 0 failed ===`

**Step 3: Commit**

```bash
git add tests/e2e/wws_multinode_e2e.sh
git commit -m "test(e2e): add multi-node peer discovery E2E test"
```

---

## Task 3: Write `tests/playwright/wws-features-e2e.spec.js` — UI Panel Tests

**Files:**
- Create: `tests/playwright/wws-features-e2e.spec.js`

**Context:** This spec runs against a single running connector on `http://127.0.0.1:19371` (started by the master script in Task 4). It tests all UI panels.

**What this test covers:**
1. Web console loads (brand visible)
2. Header shows identity name (or "—") and tier badge
3. Header stats show peer count (even if 0)
4. LeftColumn "My Agent" section shows DID (truncated) and reputation score
5. LeftColumn "Names" section exists with "+ Register name" button
6. LeftColumn "Key Health" section shows keypair status "ok"
7. LeftColumn "Network" section shows NAT type
8. KeyManagementPanel: opens via ⚙ button, shows DID, key_type Ed25519, closes
9. ReputationPanel: opens via rep score click, shows score/tier, closes
10. NameRegistryPanel: opens via "+ Register name" click, can register a name, name appears in list, closes
11. AuditPanel: opens via Audit button, closes
12. View switching: Graph → Directory → Activity all render without crash

**Step 1: Write the spec**

```js
/**
 * WWS Features UI E2E — tests all WWS-specific UI panels.
 *
 * Runs against a single connector on http://127.0.0.1:19371 with no agents.
 * Tests every LeftColumn section and each slide-over panel.
 */
const { test, expect } = require('@playwright/test')

test.setTimeout(120000)

test('WWS Features: UI panels and identity display', async ({ page, request }) => {

  // ── Step 1: Load the web console ────────────────────────────────────────
  await test.step('Load web console', async () => {
    await page.goto('/')
    await expect(page.locator('.brand')).toBeVisible({ timeout: 15000 })
    const brandText = await page.locator('.brand').textContent()
    expect(brandText).toContain('WWS')
  })

  // ── Step 2: Wait for identity data to load (polling until DID shows) ────
  await test.step('Identity loads in LeftColumn', async () => {
    // The LeftColumn identity section shows a truncated DID in .id-field
    // Wait for it to appear (needs one API poll to complete)
    await page.waitForTimeout(7000) // let the 5s poll cycle complete

    // "My Agent" section header must be visible
    await expect(page.getByText('My Agent', { exact: true })).toBeVisible()

    // .tier-badge must show a valid tier
    const badge = page.locator('.col-left .tier-badge').first()
    await expect(badge).toBeVisible()
    const tier = await badge.textContent()
    expect(['Newcomer','Member','Trusted','Established','Veteran','Suspended']).toContain(tier)
  })

  // ── Step 3: Header stats show peer/agent counts ──────────────────────────
  await test.step('Header stats render', async () => {
    const stats = page.locator('.header-stats')
    await expect(stats).toBeVisible()
    // Stats contain "agents" and "peers" labels
    await expect(stats.getByText(/agents/)).toBeVisible()
    await expect(stats.getByText(/peers/)).toBeVisible()
  })

  // ── Step 4: Reputation score visible in LeftColumn ──────────────────────
  await test.step('Reputation score in LeftColumn', async () => {
    // .rep-score shows "10 pts" for a fresh node
    const repScore = page.locator('.rep-score').first()
    await expect(repScore).toBeVisible()
    const repText = await repScore.textContent()
    expect(repText).toMatch(/\d+ pts/)
  })

  // ── Step 5: DID appears in LeftColumn identity section ──────────────────
  await test.step('DID shows in LeftColumn', async () => {
    // DID row shows label "DID" and truncated value
    await expect(page.locator('.col-left .id-label').first()).toBeVisible()
    // First id-label should say "DID"
    const labels = page.locator('.col-left .id-label')
    const firstLabel = await labels.first().textContent()
    expect(firstLabel).toBe('DID')
  })

  // ── Step 6: "+ Register name" button visible ─────────────────────────────
  await test.step('Names section: register button visible', async () => {
    const addBtn = page.getByRole('button', { name: /Register name/i })
    await expect(addBtn).toBeVisible()
  })

  // ── Step 7: Key Health section shows keypair status ──────────────────────
  await test.step('Key Health section visible', async () => {
    await expect(page.getByText('Key Health', { exact: false })).toBeVisible()
    // status-dot for keypair should exist in LeftColumn
    const dots = page.locator('.col-left .status-dot')
    await expect(dots.first()).toBeVisible()
  })

  // ── Step 8: Network section exists ───────────────────────────────────────
  await test.step('Network section visible', async () => {
    await expect(page.getByText('Network', { exact: true })).toBeVisible()
    // "NAT" label should appear
    await expect(page.locator('.col-left').getByText('NAT')).toBeVisible()
  })

  // ── Step 9: Open KeyManagementPanel via ⚙ button ────────────────────────
  await test.step('KeyManagementPanel: opens and shows key info', async () => {
    // Click ⚙ button in header
    await page.getByRole('button', { name: '⚙' }).click()

    // Panel should become visible — look for DID or Ed25519 text
    // KeyManagementPanel renders key details
    await page.waitForTimeout(1000)

    // The panel should show "did:swarm:" text somewhere
    const didText = page.getByText(/did:swarm:/, { exact: false })
    await expect(didText.first()).toBeVisible({ timeout: 5000 })

    // Close: find a close button (× or Close)
    const closeBtn = page.getByRole('button', { name: /close|×|✕/i })
    if (await closeBtn.count() > 0) {
      await closeBtn.first().click()
    } else {
      // Escape to close
      await page.keyboard.press('Escape')
    }
    await page.waitForTimeout(500)
  })

  // ── Step 10: Open ReputationPanel via rep score click ───────────────────
  await test.step('ReputationPanel: opens via rep score click', async () => {
    await page.locator('.rep-score').first().click()
    await page.waitForTimeout(1000)

    // Panel should show "Reputation" heading or score
    const repPanel = page.getByText(/Reputation/i)
    await expect(repPanel.first()).toBeVisible({ timeout: 5000 })

    // Close
    const closeBtn = page.getByRole('button', { name: /close|×|✕/i })
    if (await closeBtn.count() > 0) {
      await closeBtn.first().click()
    } else {
      await page.keyboard.press('Escape')
    }
    await page.waitForTimeout(500)
  })

  // ── Step 11: Open NameRegistryPanel, register a name ────────────────────
  await test.step('NameRegistryPanel: register wws:e2e-test-ui', async () => {
    // Click "+ Register name"
    await page.getByRole('button', { name: /Register name/i }).click()
    await page.waitForTimeout(500)

    // Input field should appear
    const input = page.locator('input[type="text"]').first()
    await expect(input).toBeVisible({ timeout: 5000 })
    await input.fill('e2e-test-ui')

    // Submit the form
    await page.getByRole('button', { name: /^Register$|^Submit$/i }).click()
    await page.waitForTimeout(2000)

    // Success message should appear
    const success = page.getByText(/Registered wws:e2e-test-ui/i)
    await expect(success).toBeVisible({ timeout: 10000 })

    // Close panel
    const closeBtn = page.getByRole('button', { name: /close|×|✕/i })
    if (await closeBtn.count() > 0) {
      await closeBtn.first().click()
    } else {
      await page.keyboard.press('Escape')
    }
    await page.waitForTimeout(2000) // let refresh cycle
  })

  // ── Step 12: Name appears in LeftColumn Names section ───────────────────
  await test.step('Registered name appears in LeftColumn', async () => {
    // After refresh, "e2e-test-ui" should appear in .name-row
    const nameRow = page.locator('.name-row').filter({ hasText: 'e2e-test-ui' })
    await expect(nameRow).toBeVisible({ timeout: 10000 })
  })

  // ── Step 13: AuditPanel opens ────────────────────────────────────────────
  await test.step('AuditPanel opens via Audit button', async () => {
    await page.getByRole('button', { name: 'Audit' }).click()
    await page.waitForTimeout(500)

    // Audit panel heading
    const auditHeading = page.getByText(/Audit/i)
    await expect(auditHeading.first()).toBeVisible({ timeout: 5000 })

    const closeBtn = page.getByRole('button', { name: /close|×|✕/i })
    if (await closeBtn.count() > 0) {
      await closeBtn.first().click()
    } else {
      await page.keyboard.press('Escape')
    }
    await page.waitForTimeout(500)
  })

  // ── Step 14: View switching (Graph, Directory, Activity) ─────────────────
  await test.step('View tabs switch without crash', async () => {
    // Switch to Directory
    await page.getByRole('button', { name: 'Directory' }).click()
    await page.waitForTimeout(1000)
    await expect(page.locator('.brand')).toBeVisible() // page didn't crash

    // Switch to Activity
    await page.getByRole('button', { name: 'Activity' }).click()
    await page.waitForTimeout(1000)
    await expect(page.locator('.brand')).toBeVisible()

    // Switch back to Graph
    await page.getByRole('button', { name: 'Graph' }).click()
    await page.waitForTimeout(1000)
    await expect(page.locator('.brand')).toBeVisible()
  })

  // ── Step 15: Verify name in API after UI registration ───────────────────
  await test.step('API confirms name registered', async () => {
    const resp = await request.get('/api/names')
    const data = await resp.json()
    const names = data.names || []
    const found = names.find(n => n.name === 'e2e-test-ui')
    expect(found, 'e2e-test-ui must appear in /api/names').toBeTruthy()
  })
})
```

**Step 2: Run only this spec to verify**

```bash
cd tests/playwright
WEB_BASE_URL=http://127.0.0.1:19371 npx playwright test wws-features-e2e.spec.js \
  --workers=1 --reporter=list --retries=0 --timeout=120000
```

Expected: `1 passed`

**Step 3: Commit**

```bash
git add tests/playwright/wws-features-e2e.spec.js
git commit -m "test(playwright): add WWS features UI panel E2E spec"
```

---

## Task 4: Write `tests/e2e/wws_all_e2e.sh` — Master Orchestrator

**Files:**
- Create: `tests/e2e/wws_all_e2e.sh`

**What this does:**
1. Builds release binary
2. Starts a 3-node swarm on 19xxx ports (separate from phase3 on 9xxx)
3. Runs single-node API tests (Task 1) — but single-node starts its own connector on 19370-19371; the multinode test also uses 19370-19375. To avoid port conflicts: the master script runs the single-node test FIRST on its own ports, then cleans up, then runs the multi-node test, then starts a single connector for the Playwright UI test.

**Step 1: Write the script**

```bash
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

# ── Build ─────────────────────────────────────────────────────────────────────
log "Building release binary..."
cd "$ROOT_DIR"
cargo build --release -p wws-connector 2>&1 | tail -3
log "Binary ready: $BINARY"

# ── Phase 1: Single-node API tests ───────────────────────────────────────────
sep
log "PHASE 1: Single-node API tests"
sep
if bash "$ROOT_DIR/tests/e2e/wws_e2e.sh"; then
    log "PHASE 1: PASSED"
    ((PASS_TOTAL++))
else
    log "PHASE 1: FAILED"
    ((FAIL_TOTAL++))
fi
pkill -f "wws-connector" 2>/dev/null || true; sleep 2

# ── Phase 2: Multi-node peer discovery tests ──────────────────────────────────
sep
log "PHASE 2: Multi-node peer discovery tests"
sep
if bash "$ROOT_DIR/tests/e2e/wws_multinode_e2e.sh"; then
    log "PHASE 2: PASSED"
    ((PASS_TOTAL++))
else
    log "PHASE 2: FAILED"
    ((FAIL_TOTAL++))
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
    ((PASS_TOTAL++))
else
    log "PHASE 3: FAILED (exit=$PW_EXIT)"
    ((FAIL_TOTAL++))
fi

# ── Final summary ─────────────────────────────────────────────────────────────
sep
echo ""
echo "  WWS ALL E2E SUMMARY"
echo "  Phases passed: $PASS_TOTAL"
echo "  Phases failed: $FAIL_TOTAL"
sep

[ "$FAIL_TOTAL" -eq 0 ]
```

**Step 2: Make executable and run**

```bash
chmod +x tests/e2e/wws_all_e2e.sh
bash tests/e2e/wws_all_e2e.sh 2>&1 | tee /tmp/wws-all-e2e.log
```

Expected output ends with:
```
  Phases passed: 3
  Phases failed: 0
```

**Step 3: Commit**

```bash
git add tests/e2e/wws_all_e2e.sh
git commit -m "test(e2e): add wws_all_e2e.sh master orchestrator for all 3 WWS test phases"
```
