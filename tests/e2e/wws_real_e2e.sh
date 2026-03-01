#!/usr/bin/env bash
# WWS Real E2E Infrastructure Launcher
#
# Starts 9 wws-connector Docker containers across 3 subnets:
#   - wws-bootstrap:         relay node on all 3 subnets (172.20, 172.21, 172.22)
#   - wws-connector-alpha/beta: Tier1 coordinators (172.21.0.0/24)
#   - wws-connector-raft/pbft/paxos/tendermint/hashgraph/synth: Tier2 (172.22.0.0/24)
#
# Usage: bash tests/e2e/wws_real_e2e.sh
#
# Outputs /tmp/wws-real-e2e-env.sh with all exported variables for orchestrators.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/docker/wws-real-e2e/docker-compose.yml"
RESULTS_DIR="$ROOT_DIR/docs/e2e-results"
ENV_FILE="/tmp/wws-real-e2e-env.sh"

log()  { echo "[$(date +%H:%M:%S)] [wws-e2e] $*"; }
die()  { echo "[ERROR] $*" >&2; exit 1; }

# Helper: extract a JSON field value from stdin using python3
jfield() {
    local field="$1"
    python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('$field',''))" 2>/dev/null
}

mkdir -p "$RESULTS_DIR"

# ---------------------------------------------------------------------------
# Step 1: Build Docker image (skip if wws-connector:latest already exists)
# ---------------------------------------------------------------------------
log "Checking Docker image wws-connector:latest..."
if ! docker image inspect wws-connector:latest >/dev/null 2>&1; then
    log "Image not found — building wws-connector:latest from $ROOT_DIR ..."
    docker build -t wws-connector:latest "$ROOT_DIR" 2>&1 | tail -10
    log "Build complete."
else
    log "Image wws-connector:latest already exists, skipping build."
fi

# ---------------------------------------------------------------------------
# Step 2: Clean up any previous run so we start with a clean state
# ---------------------------------------------------------------------------
log "Cleaning up any previous containers/volumes..."
docker compose -f "$COMPOSE_FILE" down -v --remove-orphans 2>/dev/null || true
sleep 2

# ---------------------------------------------------------------------------
# Step 3: Start bootstrap with a placeholder peer ID first, wait for health,
#         then extract its real peer ID from /api/identity
# ---------------------------------------------------------------------------
log "Starting bootstrap node (placeholder peer ID for now)..."
BOOTSTRAP_PEER="12D3KooWDummyPlaceholder"
export BOOTSTRAP_PEER

docker compose -f "$COMPOSE_FILE" up -d bootstrap

log "Waiting for bootstrap to become healthy (up to 90s)..."
HEALTHY=0
for i in $(seq 1 45); do
    if curl -sf http://127.0.0.1:19371/api/health >/dev/null 2>&1; then
        HEALTHY=1
        log "Bootstrap is healthy after $((i * 2))s."
        break
    fi
    sleep 2
done
[ "$HEALTHY" -eq 1 ] || die "Bootstrap did not become healthy within 90s."

# Extract the real peer ID from the DID: "did:swarm:<peer_id>"
BOOTSTRAP_IDENTITY=$(curl -sf http://127.0.0.1:19371/api/identity 2>/dev/null) \
    || die "Failed to call /api/identity on bootstrap"
BOOTSTRAP_DID=$(echo "$BOOTSTRAP_IDENTITY" | jfield did)
[ -n "$BOOTSTRAP_DID" ] || die "Bootstrap /api/identity returned empty 'did' field. Response: $BOOTSTRAP_IDENTITY"

# The DID format is "did:swarm:<peer_id>" — strip the prefix to get the libp2p peer ID
BOOTSTRAP_PEER="${BOOTSTRAP_DID#did:swarm:}"
[ -n "$BOOTSTRAP_PEER" ] || die "Could not strip prefix from DID: $BOOTSTRAP_DID"
log "Bootstrap DID:     $BOOTSTRAP_DID"
log "Bootstrap peer ID: $BOOTSTRAP_PEER"
export BOOTSTRAP_PEER

# ---------------------------------------------------------------------------
# Step 4: Start all 8 connector nodes with the real BOOTSTRAP_PEER exported.
#         docker-compose uses ${BOOTSTRAP_PEER:-12D3KooWDummyPlaceholder} in its
#         command fields, so the exported env var is picked up automatically.
# ---------------------------------------------------------------------------
log "Starting 8 connector nodes with BOOTSTRAP_PEER=$BOOTSTRAP_PEER ..."
docker compose -f "$COMPOSE_FILE" up -d \
    connector-alpha connector-beta \
    connector-raft connector-pbft connector-paxos \
    connector-tendermint connector-hashgraph connector-synth

# ---------------------------------------------------------------------------
# Step 5: Wait for all 8 connector containers to become healthy
# ---------------------------------------------------------------------------
log "Waiting for all connector nodes to become healthy (up to 3 min)..."

# Container names as defined in docker-compose.yml (container_name: fields)
CONTAINER_NAMES=(
    wws-connector-alpha
    wws-connector-beta
    wws-connector-raft
    wws-connector-pbft
    wws-connector-paxos
    wws-connector-tendermint
    wws-connector-hashgraph
    wws-connector-synth
)

for container in "${CONTAINER_NAMES[@]}"; do
    NOT_HEALTHY=1
    for i in $(seq 1 36); do
        STATUS=$(docker inspect --format='{{.State.Health.Status}}' "$container" 2>/dev/null || echo "unknown")
        if [ "$STATUS" = "healthy" ]; then
            log "  $container: healthy (after $((i * 5))s)"
            NOT_HEALTHY=0
            break
        fi
        sleep 5
    done
    if [ "$NOT_HEALTHY" -eq 1 ]; then
        LAST_STATUS=$(docker inspect --format='{{.State.Health.Status}}' "$container" 2>/dev/null || echo "unknown")
        log "  WARNING: $container not healthy after 3 min (status=$LAST_STATUS), continuing anyway."
    fi
done

# Give P2P peer discovery extra time to propagate across subnets via bootstrap relay
log "Waiting 20s for cross-subnet peer discovery via bootstrap relay..."
sleep 20

# ---------------------------------------------------------------------------
# Step 6: Collect DIDs from all 9 nodes via /api/identity
# ---------------------------------------------------------------------------
log "Collecting agent identities from /api/identity..."

# Map: logical name -> HTTP port (host-side)
#   bootstrap -> 19371
#   alpha     -> 19381
#   beta      -> 19383
#   raft      -> 19385
#   pbft      -> 19387
#   paxos     -> 19389
#   tendermint-> 19391
#   hashgraph -> 19393
#   synth     -> 19395
AGENTS="bootstrap alpha beta raft pbft paxos tendermint hashgraph synth"

HTTP_PORT_bootstrap=19371
HTTP_PORT_alpha=19381
HTTP_PORT_beta=19383
HTTP_PORT_raft=19385
HTTP_PORT_pbft=19387
HTTP_PORT_paxos=19389
HTTP_PORT_tendermint=19391
HTTP_PORT_hashgraph=19393
HTTP_PORT_synth=19395

RPC_PORT_alpha=19370
RPC_PORT_beta=19372
RPC_PORT_raft=19374
RPC_PORT_pbft=19376
RPC_PORT_paxos=19378
RPC_PORT_tendermint=19380
RPC_PORT_hashgraph=19382
RPC_PORT_synth=19384

# Collect DIDs into individual variables (avoid declare -A issues in heredocs)
for agent in $AGENTS; do
    port_var="HTTP_PORT_${agent}"
    port="${!port_var}"
    resp=$(curl -sf "http://127.0.0.1:${port}/api/identity" 2>/dev/null || true)
    if [ -n "$resp" ]; then
        did=$(echo "$resp" | jfield did)
    else
        did=""
    fi
    # Assign to DID_<agent> variable
    printf -v "DID_${agent}" '%s' "$did"
    log "  $agent (port $port): ${did:-NOT_FOUND}"
done

# ---------------------------------------------------------------------------
# Step 7: Write /tmp/wws-real-e2e-env.sh for Claude orchestrator / test scripts
# ---------------------------------------------------------------------------
log "Writing environment file to $ENV_FILE ..."
cat > "$ENV_FILE" <<ENVEOF
#!/usr/bin/env bash
# WWS Real E2E environment — generated by wws_real_e2e.sh
# Source this file: source $ENV_FILE

# Bootstrap peer identity
export BOOTSTRAP_PEER="${BOOTSTRAP_PEER}"
export BOOTSTRAP_DID="${DID_bootstrap}"

# HTTP ports (host-side, mapped to container's 9371)
export HTTP_BOOTSTRAP=19371
export HTTP_ALPHA=19381
export HTTP_BETA=19383
export HTTP_RAFT=19385
export HTTP_PBFT=19387
export HTTP_PAXOS=19389
export HTTP_TENDERMINT=19391
export HTTP_HASHGRAPH=19393
export HTTP_SYNTH=19395

# RPC ports (host-side, mapped to container's 9370)
export RPC_ALPHA=19370
export RPC_BETA=19372
export RPC_RAFT=19374
export RPC_PBFT=19376
export RPC_PAXOS=19378
export RPC_TENDERMINT=19380
export RPC_HASHGRAPH=19382
export RPC_SYNTH=19384

# Agent DIDs
export DID_BOOTSTRAP="${DID_bootstrap}"
export DID_ALPHA="${DID_alpha}"
export DID_BETA="${DID_beta}"
export DID_RAFT="${DID_raft}"
export DID_PBFT="${DID_pbft}"
export DID_PAXOS="${DID_paxos}"
export DID_TENDERMINT="${DID_tendermint}"
export DID_HASHGRAPH="${DID_hashgraph}"
export DID_SYNTH="${DID_synth}"

# Paths
export RESULTS_DIR="${RESULTS_DIR}"
export COMPOSE_FILE="${COMPOSE_FILE}"
ENVEOF
chmod +x "$ENV_FILE"
log "Environment file written."

# ---------------------------------------------------------------------------
# Step 8: Verify cross-subnet connectivity
#   - connector-alpha (Tier1, 172.21.0.0/24) should see peers
#   - connector-raft  (Tier2, 172.22.0.0/24) should see peers
# ---------------------------------------------------------------------------
log "Verifying cross-subnet peer connectivity..."

ALPHA_NET=$(curl -sf "http://127.0.0.1:${HTTP_PORT_alpha}/api/network" 2>/dev/null || echo '{}')
RAFT_NET=$(curl -sf  "http://127.0.0.1:${HTTP_PORT_raft}/api/network"  2>/dev/null || echo '{}')

ALPHA_PEERS=$(echo "$ALPHA_NET" | jfield peer_count || echo "0")
RAFT_PEERS=$(echo "$RAFT_NET"  | jfield peer_count || echo "0")

log "  connector-alpha peer_count: ${ALPHA_PEERS:-0}  (Tier1)"
log "  connector-raft  peer_count: ${RAFT_PEERS:-0}   (Tier2)"

if [ "${ALPHA_PEERS:-0}" -gt 0 ] && [ "${RAFT_PEERS:-0}" -gt 0 ]; then
    log "Cross-subnet connectivity: OK"
else
    log "WARNING: One or more nodes report 0 peers — peer discovery may still be in progress."
fi

# ---------------------------------------------------------------------------
# Step 9: Print summary
# ---------------------------------------------------------------------------
echo ""
echo "======================================================================"
echo "  WWS REAL E2E INFRASTRUCTURE READY"
echo "======================================================================"
echo "  Bootstrap relay:"
echo "    HTTP:  http://127.0.0.1:19371"
echo "    DID:   ${DID_bootstrap:-unknown}"
echo ""
echo "  Tier1 coordinators:"
echo "    connector-alpha  RPC=127.0.0.1:19370  HTTP=http://127.0.0.1:19381"
echo "    connector-beta   RPC=127.0.0.1:19372  HTTP=http://127.0.0.1:19383"
echo ""
echo "  Tier2 researchers/synth:"
echo "    connector-raft        RPC=127.0.0.1:19374  HTTP=http://127.0.0.1:19385"
echo "    connector-pbft        RPC=127.0.0.1:19376  HTTP=http://127.0.0.1:19387"
echo "    connector-paxos       RPC=127.0.0.1:19378  HTTP=http://127.0.0.1:19389"
echo "    connector-tendermint  RPC=127.0.0.1:19380  HTTP=http://127.0.0.1:19391"
echo "    connector-hashgraph   RPC=127.0.0.1:19382  HTTP=http://127.0.0.1:19393"
echo "    connector-synth       RPC=127.0.0.1:19384  HTTP=http://127.0.0.1:19395"
echo ""
echo "  Env file: $ENV_FILE"
echo "    -> source $ENV_FILE"
echo ""
echo "  Cross-subnet peers:"
echo "    connector-alpha: ${ALPHA_PEERS:-0} peers"
echo "    connector-raft:  ${RAFT_PEERS:-0} peers"
echo ""
echo "  Tear down: docker compose -f $COMPOSE_FILE down -v"
echo "======================================================================"
