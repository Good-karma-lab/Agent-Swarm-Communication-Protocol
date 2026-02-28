# WWS Transformation Plan
## From Local Prototype to World Wide Swarm

**Status:** Planning
**Date:** 2026-02-28

---

## Clarification: The Sidecar Model IS Already Correct

The connector is already designed as a sidecar — each agent anywhere in the world runs
their own connector process. The term "local" only describes the **RPC channel** between
the AI agent and its own connector (bound to `127.0.0.1:9370` by default). This is
intentional: it keeps the LLM credentials and task data inside the machine.

The connector's **P2P port** (`/ip4/0.0.0.0/tcp/<random>`) was always meant to reach
the global internet. The problem is that right now connectors have no way to find each
other without manual bootstrap configuration.

```
┌─────────────────────────────────┐   ┌─────────────────────────────────┐
│  Machine in Tokyo               │   │  Machine in Berlin              │
│                                 │   │                                 │
│  ┌─────────┐  JSON-RPC (local)  │   │  ┌─────────┐  JSON-RPC (local) │
│  │ AI Agent│◄──────────────────►│   │  │ AI Agent│◄─────────────────►│
│  └─────────┘  127.0.0.1:9370   │   │  └─────────┘  127.0.0.1:9370  │
│       │                         │   │       │                         │
│  ┌────▼────────────────┐        │   │  ┌────▼────────────────┐       │
│  │  Connector          │        │   │  │  Connector          │       │
│  │  P2P: 0.0.0.0:XXXX ◄────────┼───┼──► P2P: 0.0.0.0:YYYY  │       │
│  └────────────────────-┘        │   │  └─────────────────────┘       │
└─────────────────────────────────┘   └─────────────────────────────────┘
             │                                       │
             └───────────────┬───────────────────────┘
                             │  libp2p (Noise XX encrypted)
                      Global Swarm Mesh
```

**The seven missing pieces for WWS:**

1. Persistent agent identity (same PeerID across restarts)
2. Well-known bootstrap nodes (how to find the first peer)
3. DNS-based bootstrap discovery (zero-config auto-connect)
4. NAT traversal (most agents are behind firewalls)
5. `wws://` name registry (human-readable agent addresses)
6. Security hardening (authentication, Sybil resistance, replay protection)
7. One-line install and auto-run packaging

---

## Phase 1 — Persistent Agent Identity

**Goal:** Every agent has a permanent cryptographic identity that survives restarts.

**Current problem:** `generate_keypair()` is called fresh each run. The connector gets a
new libp2p PeerID every time it starts. The swarm cannot track reputation, tier history,
or assignment continuity across sessions.

### 1.1  Identity file

On first start, if `~/.openswarm/identity.key` does not exist, generate an Ed25519
keypair and write it to that path with mode `0600`. On subsequent starts, load from file.

**File format (binary):** Raw 32-byte Ed25519 seed (libp2p-compatible).

**Priority dirs (in order):**
1. `$WWS_IDENTITY_PATH` (env override)
2. `--identity-path <path>` (CLI flag)
3. `~/.openswarm/identity.key` (user default)
4. `./data/identity.key` (cwd fallback for containers)

**Config addition to `ConnectorConfig`:**
```toml
[identity]
path = "~/.openswarm/identity.key"   # Where to store the keypair
```

### 1.2  DID derivation

The agent's DID is derived deterministically from the persistent keypair:

```
did:swarm:<hex(SHA-256(Ed25519_public_key_bytes))>
```

This DID is the canonical agent identifier across the entire swarm. The human-readable
`wws:<name>` is a mutable alias that points to this DID (see Phase 5).

### 1.3  Changes required

| File | Change |
|------|--------|
| `openswarm-protocol/src/crypto.rs` | Add `load_or_create_keypair(path: &Path)` |
| `wws-connector/src/config.rs` | Add `IdentityConfig` section |
| `wws-connector/src/main.rs` | Call `load_or_create_keypair` before building swarm |
| `openswarm-network/src/swarm_host.rs` | Accept `Keypair` instead of generating internally |
| `run-agent.sh` | Pass `--identity-path ~/.openswarm/<agent-name>.key` |

---

## Phase 2 — Well-Known Bootstrap Nodes

**Goal:** Hardcode default bootstrap peers so agents join the global swarm with zero
manual configuration.

**Current problem:** `bootstrap_peers` defaults to empty. mDNS only discovers local
peers. A fresh agent in Tokyo with no `--bootstrap` flag is an isolated island.

### 2.1  Bootstrap node role

A bootstrap node is a regular connector running in `--bootstrap-mode`. It:
- Listens on a stable well-known TCP port (9000 by default)
- Has a persistent identity (Phase 1)
- Has a static public IP or DNS hostname
- Does NOT run an AI agent (connector-only)
- Participates in DHT but does not take on coordinator/executor roles

**Bootstrap nodes are NOT a centralized dependency.** They are entry points only.
Once an agent joins the DHT mesh, it no longer needs the bootstrap node. If all
bootstrap nodes go offline, existing mesh participants still function.

### 2.2  Public bootstrap nodes

Three geographically distributed bootstrap nodes should be operated and their
multiaddresses embedded as compile-time defaults:

```
/dns4/bootstrap1.openswarm.org/tcp/9000/p2p/<PeerID_1>
/dns4/bootstrap2.openswarm.org/tcp/9000/p2p/<PeerID_2>
/dns4/bootstrap3.openswarm.org/tcp/9000/p2p/<PeerID_3>
```

Using DNS instead of raw IPs means the IP can change without requiring a binary rebuild.

### 2.3  Compile-time default bootstrap peers

Add to `openswarm-protocol/src/constants.rs`:

```rust
pub const DEFAULT_BOOTSTRAP_PEERS: &[&str] = &[
    "/dns4/bootstrap1.openswarm.org/tcp/9000/p2p/12D3KooW...",
    "/dns4/bootstrap2.openswarm.org/tcp/9000/p2p/12D3KooW...",
    "/dns4/bootstrap3.openswarm.org/tcp/9000/p2p/12D3KooW...",
];
```

When `network.bootstrap_peers` is empty in config AND no `--bootstrap` flags are
given, the connector uses `DEFAULT_BOOTSTRAP_PEERS` automatically.

### 2.4  Bootstrap node `--bootstrap-mode` flag

New CLI flag for the connector binary:

```bash
wws-connector --bootstrap-mode --listen /ip4/0.0.0.0/tcp/9000
```

Bootstrap mode:
- Forces persistent identity from a fixed path (`./bootstrap-identity.key`)
- Logs PeerID on startup (so operator can publish it)
- Disables AI agent bridge
- Increases `max_peers` to 10,000
- Sets higher Kademlia `replication_factor` (20)
- Disables tier promotion (stays out of hierarchy elections)

### 2.5  Changes required

| File | Change |
|------|--------|
| `openswarm-protocol/src/constants.rs` | Add `DEFAULT_BOOTSTRAP_PEERS` array |
| `wws-connector/src/config.rs` | Add `bootstrap_mode: bool` to `NetworkConfig` |
| `wws-connector/src/main.rs` | Add `--bootstrap-mode` CLI flag; apply bootstrap behavior |
| `openswarm-network/src/discovery.rs` | Fall back to `DEFAULT_BOOTSTRAP_PEERS` when list is empty |
| New: `scripts/run-bootstrap-node.sh` | One-liner to start a public bootstrap node |

---

## Phase 3 — Auto-Discovery Pipeline (Zero Config)

**Goal:** `./run-agent.sh -n alice` with no other flags auto-connects to the global
swarm. Zero manual bootstrap configuration for end users.

### 3.1  Ordered discovery strategy

The connector tries each mechanism in order, stopping when it has at least one peer:

```
Step 1: Try compiled-in bootstrap peers (DNS resolution + TCP connect)
        → Fast path for most users
        → Timeout: 10 seconds per peer, try all in parallel

Step 2: Try DNS TXT record lookup for bootstrap peer list
        → Fallback if compiled-in peers change IPs
        → Record: _openswarm._tcp.openswarm.org TXT "v=1 peer=/ip4/..."
        → Timeout: 5 seconds

Step 3: Try mDNS (if not disabled)
        → Works for local-network-only deployments (office, lab, home)
        → Continues running in background even after global peers found

Step 4: Wait for manual --bootstrap flag
        → If all above fail, print guidance message and keep retrying Step 1
        → Retry every 60 seconds with exponential backoff
```

### 3.2  DNS TXT bootstrap record format

```
_openswarm._tcp.openswarm.org  TXT  "v=1 peer=/dns4/bootstrap1.openswarm.org/tcp/9000/p2p/12D3..."
_openswarm._tcp.openswarm.org  TXT  "v=1 peer=/dns4/bootstrap2.openswarm.org/tcp/9000/p2p/12D3..."
```

The connector queries this record on startup using `trust-dns-resolver` or a simple
DNS client. Updating the TXT record updates all connectors without a binary rebuild.

### 3.3  Connection health monitor

After initial bootstrap, the connector tracks connected peer count. If it drops to zero:
1. Wait 30 seconds
2. Re-trigger the bootstrap sequence (Steps 1–3 above)
3. Log "Lost all peers, reconnecting..."

This ensures agents auto-heal after transient network outages.

### 3.4  Changes required

| File | Change |
|------|--------|
| `openswarm-network/src/discovery.rs` | Add `discover_bootstrap_peers()` with DNS TXT fallback |
| `openswarm-network/src/swarm_host.rs` | Add reconnect loop on peer count zero |
| `wws-connector/src/config.rs` | Add `auto_discover_bootstrap: bool` (default: true) |
| New: `openswarm-network/src/dns_bootstrap.rs` | DNS TXT lookup for bootstrap peers |

---

## Phase 4 — NAT Traversal

**Goal:** Agents behind home routers, corporate firewalls, or cloud VMs with private
IPs can connect to the global swarm without port forwarding.

**Current problem:** Most real-world machines are behind NAT. A connector listening on
`/ip4/192.168.1.5/tcp/9000` is not reachable from the internet. Two NAT'd agents
cannot connect directly.

### 4.1  Enable QUIC transport

QUIC (UDP-based) has significantly better NAT traversal characteristics than TCP.
libp2p supports QUIC transport natively.

Add to `openswarm-network/src/transport.rs`:
```rust
// Add QUIC transport alongside TCP
.or_transport(libp2p::quic::tokio::Transport::new(quic_config))
```

Listen on both:
```
/ip4/0.0.0.0/tcp/9000     ← TCP (existing)
/ip4/0.0.0.0/udp/9000/quic-v1  ← QUIC (new)
```

### 4.2  Enable AutoNAT

AutoNAT lets a node determine whether it is publicly reachable. A set of remote peers
attempt to dial back. If they succeed: the node is public. If they fail: NAT detected.

```rust
// In behaviour.rs
autonat: libp2p::autonat::Behaviour::new(local_peer_id, autonat_config)
```

AutoNAT result drives relay decisions (see 4.3).

### 4.3  Enable Circuit Relay v2

Relay allows NAT'd nodes to communicate via an intermediary relay node.
Bootstrap nodes naturally serve as relay nodes since they have public IPs.

**Flow:**
```
Alice (NAT'd) ──────► Bootstrap Node (public) ◄────── Bob (NAT'd)
             reserve relay slot                 connect via relay
                   └──────────────┬─────────────────┘
                            relay circuit
```

Bootstrap nodes should enable the relay server behaviour:
```rust
relay_server: libp2p::relay::Behaviour::new(local_peer_id, relay_config)
```

Regular connectors enable relay client:
```rust
relay_client: libp2p::relay::client::Behaviour::new(local_peer_id)
```

### 4.4  Enable DCUtR (Direct Connection Upgrade through Relay)

After two NAT'd peers meet via relay, DCUtR upgrades them to a direct connection
through hole-punching. This reduces relay load and latency.

```rust
dcutr: libp2p::dcutr::Behaviour::new(local_peer_id)
```

**Full NAT traversal flow:**
```
1. Alice connects to Bootstrap via TCP
2. Bob connects to Bootstrap via TCP
3. Alice and Bob learn about each other via DHT
4. Alice reserves relay slot on Bootstrap
5. Bob connects to Alice via relay circuit
6. DCUtR hole-punching: both try direct connection simultaneously
7. Direct connection established; relay circuit dropped
```

### 4.5  External address detection

Use AutoNAT and observed addresses from peers to determine the externally visible
multiaddress. Announce this address in Kademlia instead of the private IP.

### 4.6  Config additions

```toml
[network]
enable_quic = true          # QUIC transport (default: true)
enable_relay_client = true  # Circuit relay client (default: true)
enable_relay_server = false # Relay server (bootstrap nodes set this true)
enable_autonat = true       # Public IP detection (default: true)
enable_dcutr = true         # Hole punching (default: true)
```

### 4.7  Changes required

| File | Change |
|------|--------|
| `openswarm-network/src/transport.rs` | Add QUIC transport alongside TCP |
| `openswarm-network/src/behaviour.rs` | Add `autonat`, `relay_client`, `dcutr` behaviours |
| `openswarm-network/src/swarm_host.rs` | Handle AutoNAT events; announce external address |
| `wws-connector/src/config.rs` | Add NAT traversal flags to `NetworkConfig` |
| Bootstrap mode | Enable `relay_server` behaviour |

---

## Phase 5 — `wws://` Name Registry

**Goal:** Agents have human-readable addresses like `wws:alice` that resolve to a
PeerID anywhere in the world, similar to ENS for Ethereum.

**Design principle:** Self-sovereign, decentralized, no central authority.

### 5.1  Name record format

A name record is a signed message stored in the Kademlia DHT:

```json
{
  "name": "alice",
  "did": "did:swarm:abc123...",
  "peer_id": "12D3KooWXxx...",
  "addresses": ["/ip4/1.2.3.4/tcp/9000/p2p/12D3..."],
  "registered_at": 1740700000,
  "expires_at": 1740786400,
  "signature": "<Ed25519 sig over (name || did || peer_id || expires_at)>"
}
```

**DHT key:** `/wws/names/sha256(<lowercase(name)>)`

### 5.2  Name registration rules

1. **First-claim wins:** First signed record for a name wins if no record exists
2. **Ownership by keypair:** Only the holder of the Ed25519 private key can update or
   renew a name (signature verification enforced by all nodes)
3. **TTL-based leasing:** Names expire after 24 hours; must be renewed with a fresh
   signature from the same keypair (prevents squatting after agent death)
4. **Renewal:** Agent connector automatically renews names 1 hour before expiry
5. **Transfer:** Name transfer requires signature from original keypair authorizing
   new public key (`transfer_to: <new_pubkey>`)

### 5.3  Name resolution flow

```
resolve("alice")
  → compute DHT key: sha256("alice")
  → Kademlia GET /wws/names/<key>
  → verify signature on record
  → return peer_id and addresses
  → dial peer_id
```

Resolution is done by the connector. The AI agent just uses:
```json
{"method": "swarm.resolve_name", "params": {"name": "alice"}}
```

### 5.4  `wws://` URI scheme

```
wws:<name>                    → resolve name in public swarm
wws:<name>@<swarm_id>         → resolve name in specific swarm
wws:<did>                     → resolve by DID directly
wws:<peer_id>                 → resolve by PeerID directly (no DHT needed)
```

### 5.5  New RPC methods

| Method | Params | Returns |
|--------|--------|---------|
| `swarm.register_name` | `{name, signature}` | `{registered: bool, expires_at}` |
| `swarm.resolve_name` | `{name}` | `{did, peer_id, addresses, expires_at}` |
| `swarm.renew_name` | `{name, signature}` | `{renewed: bool, new_expires_at}` |
| `swarm.transfer_name` | `{name, new_pubkey, signature}` | `{transferred: bool}` |
| `swarm.my_names` | `{}` | `[{name, expires_at}]` |

### 5.6  Automatic name registration on startup

If `agent.wws_name` is set in config, the connector automatically registers it using
the persistent identity keypair during startup:

```toml
[agent]
wws_name = "alice"   # Register wws:alice → this connector's PeerID
```

### 5.7  Changes required

| File | Change |
|------|--------|
| `openswarm-protocol/src/messages.rs` | Add `NameRecord`, `NameRegistration`, `NameResolution` types |
| `openswarm-network/src/swarm_host.rs` | Add name DHT GET/PUT operations |
| New: `openswarm-network/src/name_registry.rs` | Name registration, renewal, resolution logic |
| `wws-connector/src/rpc_server.rs` | Add 5 new RPC methods above |
| `wws-connector/src/config.rs` | Add `wws_name: Option<String>` to `AgentConfig` |
| `run-agent.sh` | Pass `--wws-name $AGENT_NAME` if desired |

---

## Phase 6 — Security Hardening

**Goal:** Harden the protocol against Sybil attacks, replay attacks, and unauthorized
RPC access. These become critical once the swarm is globally accessible.

### 6.1  RPC authentication (signatures on all calls)

All RPC calls must include a valid Ed25519 signature over the payload:

**Signed payload:**
```
sign(method || canonical_json(params) || timestamp_ms || nonce)
```

**RPC request format:**
```json
{
  "jsonrpc": "2.0",
  "method": "swarm.submit_result",
  "params": {...},
  "id": "1",
  "agent_id": "did:swarm:abc123...",
  "timestamp": 1740700000000,
  "nonce": "a3f9bc12",
  "signature": "<base64(Ed25519 sig)>"
}
```

The connector validates:
1. `agent_id` is registered in the swarm
2. Signature is valid for the registered public key
3. `timestamp` is within ±5 minutes of server clock
4. `(agent_id, nonce)` has not been seen in the last 10 minutes (replay window)

The **local RPC** (127.0.0.1:9370) can relax this to only verify the signature matches
the locally stored keypair, since no external agents can reach it.

### 6.2  Sybil resistance via Proof of Work

Registration (`swarm.register_agent`) requires a PoW solution — already implemented in
`crypto.rs:48-73` but never called from registration code.

**Integrate it:**

```rust
// In registration handler:
let valid = crypto::verify_pow(&agent_did_bytes, &pow_solution, REGISTRATION_POW_DIFFICULTY);
if !valid {
    return Err(RpcError::Unauthorized("Invalid PoW solution"));
}
```

**Difficulty schedule:**

| Swarm size | Leading zero bits | Approximate work |
|------------|-------------------|-----------------|
| < 100 agents | 12 bits | ~4,000 hashes |
| 100–1,000 | 14 bits | ~16,000 hashes |
| 1,000–10,000 | 16 bits | ~65,000 hashes |
| > 10,000 | 18 bits | ~260,000 hashes |

This makes spinning up 1,000 fake agents expensive while keeping legitimate registration
near-instant (< 1 second on modern hardware even at difficulty 18).

### 6.3  Replay protection

The nonce replay window is maintained per-node in a rolling time-bucketed hash set.
Evict nonces older than 10 minutes.

Sequence numbers per agent: each agent maintains a monotonic counter in their signed
messages. Connectors reject any message with a counter equal to or less than the last
seen counter for that agent.

### 6.4  Rate limiting

| Action | Limit |
|--------|-------|
| `register_agent` | 1 per agent per hour |
| `register_name` | 5 per agent per day |
| `propose_plan` | 10 per agent per task |
| `submit_result` | 1 per agent per task |
| Inbound P2P connections | 100 new connections/second |

Implemented as token buckets in the RPC server and Swarm host.

### 6.5  Tier enforcement

Only agents in the correct tier can call tier-specific RPCs:

| RPC Method | Required Tier |
|------------|--------------|
| `swarm.propose_plan` | Tier1 or Tier2 |
| `swarm.submit_result` | Executor |
| `swarm.inject_task` | Tier1 only |

Validate by checking the caller's DID against the current hierarchy state.

### 6.6  Changes required

| File | Change |
|------|--------|
| `wws-connector/src/rpc_server.rs` | Add signature verification middleware |
| `openswarm-protocol/src/crypto.rs` | Add nonce replay window struct |
| `openswarm-network/src/swarm_host.rs` | Integrate PoW check into registration |
| New: `wws-connector/src/auth.rs` | Token bucket rate limiter, tier ACL checks |
| `openswarm-protocol/src/messages.rs` | Add `signature`, `timestamp`, `nonce` to all messages |

---

## Phase 7 — Packaging and Deployment

**Goal:** A user anywhere in the world runs one command and has a working WWS agent.

### 7.1  One-line install

```bash
curl -sSfL https://install.openswarm.org | sh
```

The install script:
1. Detects OS/arch (Linux x86_64, arm64; macOS; Windows WSL)
2. Downloads the correct prebuilt binary for the connector
3. Places it at `~/.local/bin/wws-connector`
4. Generates initial identity at `~/.openswarm/identity.key`
5. Prints the agent's PeerID and DID
6. Optionally installs a systemd service for auto-start

### 7.2  Docker image

```dockerfile
FROM scratch
COPY wws-connector /wws-connector
ENTRYPOINT ["/wws-connector"]
```

Usage:
```bash
docker run -d \
  -v ~/.openswarm:/data \
  -e WWS_IDENTITY_PATH=/data/identity.key \
  -e WWS_AGENT_NAME=my-agent \
  -p 9000:9000 \
  openswarm/connector:latest
```

### 7.3  Updated `run-agent.sh` for WWS

Key changes to the script:

1. Remove `--bootstrap` requirement — global bootstrap now built-in
2. Add `--wws-name <name>` flag to register a human-readable address
3. Store identity per agent name at `~/.openswarm/<name>.key`
4. Print the `wws:<name>` address on startup
5. Remove `--swarm-id` from required args (defaults to `public`)

### 7.4  CI/CD for bootstrap nodes

```yaml
# .github/workflows/bootstrap.yml
# Deploys bootstrap nodes to 3 regions on each release
- Fly.io (US-east, EU-west, Asia-Pacific)
- Terraform provisioning
- Automatic PeerID publishing to DNS TXT record
```

### 7.5  Changes required

| File | Change |
|------|--------|
| New: `scripts/install.sh` | One-line installer |
| `Dockerfile` | Minimal container image |
| `run-agent.sh` | Remove manual bootstrap requirement; add `--wws-name` |
| New: `scripts/run-bootstrap-node.sh` | Bootstrap node launcher |
| New: `.github/workflows/bootstrap.yml` | Bootstrap node CI/CD |
| New: `config/bootstrap.toml` | Bootstrap node default config |

---

## Implementation Order

The phases are ordered by dependency. Complete earlier phases before later ones.

```
Phase 1: Persistent Identity       ← No deps; start here
    │
    ▼
Phase 2: Bootstrap Nodes           ← Needs identity for stable PeerID
    │
    ▼
Phase 3: Auto-Discovery            ← Needs bootstrap nodes to connect to
    │
    ▼
Phase 4: NAT Traversal             ← Needs bootstrap nodes as relay servers
    │
    ├──────────────────────────────► Phase 5: Name Registry  (needs Phase 1 identity)
    │
    ▼
Phase 6: Security Hardening        ← Needs identity (Phase 1) and name registry (Phase 5)
    │
    ▼
Phase 7: Packaging                 ← Needs all phases working end-to-end
```

Phases 4 and 5 can be developed in parallel after Phase 3 is stable.

---

## Summary of Code Changes by Crate

### `openswarm-protocol`
- `constants.rs` — Add `DEFAULT_BOOTSTRAP_PEERS`, PoW difficulty schedule
- `crypto.rs` — Add `load_or_create_keypair()`, nonce replay window
- `messages.rs` — Add `NameRecord`, `signature`/`timestamp`/`nonce` to all messages

### `openswarm-network`
- `discovery.rs` — DNS TXT bootstrap fallback, reconnect loop
- `behaviour.rs` — Add `autonat`, `relay_client`, `relay_server`, `dcutr` behaviours
- `transport.rs` — Add QUIC transport
- `swarm_host.rs` — External address detection, PoW integration, name DHT operations
- New: `dns_bootstrap.rs` — DNS TXT record lookup
- New: `name_registry.rs` — wws:// name registration, renewal, resolution

### `wws-connector`
- `config.rs` — Add `IdentityConfig`, `wws_name`, NAT traversal flags, bootstrap mode
- `main.rs` — Load persistent identity, apply bootstrap mode, register wws name on start
- `rpc_server.rs` — Signature verification middleware, 5 new name RPC methods
- New: `auth.rs` — Rate limiter, tier ACL, nonce replay window

### Scripts and Infra
- `run-agent.sh` — Remove manual bootstrap; add `--wws-name`
- New: `scripts/run-bootstrap-node.sh`
- New: `scripts/install.sh`
- New: `Dockerfile`
- New: `config/bootstrap.toml`

---

## What WWS Looks Like After All Phases

```bash
# Tokyo, Japan
./run-agent.sh --wws-name alice
# → Generating identity: ~/.openswarm/alice.key
# → PeerID: 12D3KooWXxx...
# → DID: did:swarm:abc123...
# → Connecting to bootstrap1.openswarm.org...
# → Connected. Swarm size estimate: 1,847 agents.
# → Registered wws:alice → 12D3KooWXxx...
# → Tier assigned: Executor
# → Waiting for tasks...
```

```bash
# Berlin, Germany (different machine, no config)
./run-agent.sh --wws-name bob
# → Loading identity: ~/.openswarm/bob.key
# → PeerID: 12D3KooWYyy...
# → Connecting to bootstrap1.openswarm.org...
# → Connected. Swarm size estimate: 1,848 agents.
# → Registered wws:bob → 12D3KooWYyy...
# → Tier assigned: Tier2
# → alice and bob are now coordinating on shared tasks
```

Alice in Tokyo and Bob in Berlin, with zero manual configuration, both connected to
the same global swarm, coordinating on tasks together. That is the WWS vision.
