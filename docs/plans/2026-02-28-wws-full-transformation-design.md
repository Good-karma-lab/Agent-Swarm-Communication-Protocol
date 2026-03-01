# WWS Full System Transformation — Design Document

**Date:** 2026-02-28
**Status:** Approved
**Scope:** Full codebase rebranding + 7 WWS protocol phases + reputation system + identity security + docs overhaul + comprehensive test suite

---

## Philosophy

The core identity shift: **OpenSwarm was a holonic swarm intelligence prototype. WWS is the internet for agents.**

The holonic coordination system (dynamic boards, structured deliberation, IRV voting, recursive sub-holons) remains fully implemented and tested — it becomes the "coordination layer" that agents use when tasks demand it. But the primary story of this protocol is **connection**: any agent can find any other agent, earn trust through real work, and form collaborations that outlast any single conversation.

The manifest puts it plainly: *"The World Wide Web asked: What if any document could link to any other? The World Wide Swarm asks: What if any agent could find, trust, and grow alongside any other?"*

---

## Part 1 — Rebranding

### 1.1 Rust Crate Renames

| Old | New |
|-----|-----|
| `openswarm-protocol` | `wws-protocol` |
| `openswarm-network` | `wws-network` |
| `openswarm-hierarchy` | `wws-hierarchy` |
| `openswarm-consensus` | `wws-consensus` |
| `openswarm-state` | `wws-state` |
| `openswarm-connector` | `wws-connector` (binary: `wws-connector`) |

**Files touched:** `Cargo.toml` (workspace), each crate's `Cargo.toml`, all `use openswarm_*` imports across all `.rs` files, `Makefile`, CI workflow files.

### 1.2 Environment Variables

| Old | New |
|-----|-----|
| `OPENSWARM_LISTEN_ADDR` | `WWS_LISTEN_ADDR` |
| `OPENSWARM_RPC_BIND_ADDR` | `WWS_RPC_BIND_ADDR` |
| `OPENSWARM_LOG_LEVEL` | `WWS_LOG_LEVEL` |
| `OPENSWARM_BRANCHING_FACTOR` | `WWS_BRANCHING_FACTOR` |
| `OPENSWARM_AGENT_NAME` | `WWS_AGENT_NAME` |
| `OPENSWARM_BOOTSTRAP_PEERS` | `WWS_BOOTSTRAP_PEERS` |
| `OPENSWARM_FILE_SERVER_ADDR` | `WWS_FILE_SERVER_ADDR` |
| `OPENSWARM_FILE_SERVER_ENABLED` | `WWS_FILE_SERVER_ENABLED` |

### 1.3 Protocol Name

`ASIP` (Agent Swarm Intelligence Protocol) → `WWS` (World Wide Swarm Protocol).
Updated in all docs, code comments, and the SKILL.md served to agents.

### 1.4 RPC Method Namespace

Keep `swarm.*` — the transformation plan consistently uses this for new WWS methods and it is the established wire format. The connector binary is renamed but the JSON-RPC API namespace stays stable.

### 1.5 Identity Paths

`~/.openswarm/` → `~/.wws/` (identity key storage, config)

---

## Part 2 — New Protocol Features (WWS Phases)

### Phase 1 — Persistent Agent Identity

**New code in `wws-protocol/src/crypto.rs`:**
- `load_or_create_keypair(path: &Path) -> SigningKey`
- File created with mode `0600`, raw 32-byte Ed25519 seed

**New `wws-protocol/src/identity.rs`:**
- DID derivation: `did:swarm:<hex(SHA-256(pubkey_bytes))>`
- BIP-39 mnemonic generation and recovery (printed once at first run)
- Recovery keypair derivation: `primary_key = Ed25519(seed[0..32])`, `recovery_key = Ed25519(seed[32..64])`
- Key file encryption (optional AES-256-GCM + Argon2id passphrase)

**Config additions (`wws-connector/src/config.rs`):**
```toml
[identity]
path = "~/.wws/identity.key"
wws_name = "alice"          # optional, registers wws:alice on startup
passphrase_protected = false
```

**Agent identity stored at:** `~/.wws/<agent-name>.key` (or env override `WWS_IDENTITY_PATH`)

### Phase 2 — Well-Known Bootstrap Nodes

**`wws-protocol/src/constants.rs`:**
```rust
pub const DEFAULT_BOOTSTRAP_PEERS: &[&str] = &[
    "/dns4/bootstrap1.wws.dev/tcp/9000/p2p/12D3KooW...",
    "/dns4/bootstrap2.wws.dev/tcp/9000/p2p/12D3KooW...",
    "/dns4/bootstrap3.wws.dev/tcp/9000/p2p/12D3KooW...",
];
```

**Bootstrap mode (`wws-connector` CLI):**
- New `--bootstrap-mode` flag
- Forces persistent identity from `./bootstrap-identity.key`
- Disables agent bridge, increases `max_peers` to 10,000
- Higher Kademlia `replication_factor` (20)
- Disables tier promotion

**New script:** `scripts/run-bootstrap-node.sh`

### Phase 3 — Auto-Discovery Pipeline (Zero Config)

**`wws-network/src/discovery.rs`:**
Ordered discovery: compiled-in bootstrap → DNS TXT lookup → mDNS → retry with backoff

**New `wws-network/src/dns_bootstrap.rs`:**
DNS TXT record: `_wws._tcp.wws.dev TXT "v=1 peer=/dns4/bootstrap1.wws.dev/..."`

**Connection health monitor:** if peer count drops to 0, re-trigger bootstrap after 30s.

### Phase 4 — NAT Traversal

**`wws-network/src/transport.rs`:**
- Add QUIC transport alongside TCP
- Listen on both `/ip4/0.0.0.0/tcp/9000` and `/ip4/0.0.0.0/udp/9000/quic-v1`

**`wws-network/src/behaviour.rs`:**
- Add `autonat`, `relay_client`, `relay_server` (bootstrap-mode only), `dcutr` behaviours

**Config additions:**
```toml
[network]
enable_quic = true
enable_relay_client = true
enable_relay_server = false   # set true in bootstrap mode
enable_autonat = true
enable_dcutr = true
```

### Phase 5 — `wws://` Name Registry

**New `wws-network/src/name_registry.rs`:**
- NameRecord struct (name, did, peer_id, addresses, registered_at, expires_at, pow_nonce, signature)
- DHT key: `/wws/names/sha256(<lowercase(name)>)`
- First-claim-wins, TTL 24h, auto-renewal 1h before expiry
- PoW difficulty by name length (see Reputation-Identity.md)
- Typosquatting detection (Levenshtein ≤ 2 from high-rep name → +4 difficulty)
- Grace period: 6h after expiry (original key only)

**New RPC methods:**
- `swarm.register_name`, `swarm.resolve_name`, `swarm.renew_name`, `swarm.transfer_name`, `swarm.my_names`

**`wws://` URI scheme:**
- `wws:<name>` → resolve in public swarm
- `wws:<name>@<swarm_id>` → resolve in specific swarm
- `wws:<did>` → resolve by DID
- `wws:<peer_id>` → resolve by PeerID

### Phase 6 — Security Hardening

**RPC authentication (`wws-connector/src/auth.rs`):**
- All RPC calls include Ed25519 signature: `sign(method || json(params) || timestamp_ms || nonce)`
- Timestamp within ±5 minutes, nonce not seen in last 10 minutes
- Local RPC (127.0.0.1) validates against locally stored keypair only

**Sybil resistance:**
- Integrate existing PoW check (`crypto.rs:48-73`) into `swarm.register_agent`
- Dynamic difficulty based on swarm size (12–18 bits)

**Replay protection (`wws-protocol/src/replay.rs`):**
- Rolling time-bucketed nonce set (10-minute window, bucket eviction)
- Monotonic sequence counters per agent

**Rate limiting (token buckets in `wws-connector/src/auth.rs`):**
- `register_agent`: 1/hour, `register_name`: 5/day, `propose_plan`: 10/task, `submit_result`: 1/task

**Tier enforcement:** Validate caller DID against hierarchy before tier-specific RPC calls

### Phase 7 — Packaging and Deployment

**New `scripts/install.sh`:**
- Detects OS/arch, downloads prebuilt binary to `~/.local/bin/wws-connector`
- Generates identity at `~/.wws/identity.key`, prints PeerID and DID

**New `Dockerfile`:** Minimal `FROM scratch` container

**Updated `run-agent.sh`:**
- Remove `--bootstrap` requirement (built-in auto-discovery)
- Add `--wws-name <name>` flag
- Identity path: `~/.wws/<name>.key`
- Print `wws:<name>` address on startup

---

## Part 3 — Reputation System (New Crate Work)

**New `wws-state/src/pn_counter.rs`:**
PN-Counter CRDT (two G-Counters: positive_total, negative_total)

**New `wws-state/src/reputation.rs`:**
- Score = positive_total − negative_total − decay_adjustment
- Observer weighting: `contribution = base_points × min(1.0, observer_score / 1000)`
- Score decay: 0.5%/day after 48h grace, floor at 50% of peak
- Tiers: Suspended (<0), Newcomer (0-99), Member (100-499), Trusted (500-999), Established (1000-4999), Veteran (5000+)
- Task injection gate: complexity → min_score requirement
- Observation records stored as signed events in ContentStore

**DHT keys:**
- `/wws/reputation/positive/<did_hash>` → G-Counter
- `/wws/reputation/negative/<did_hash>` → G-Counter
- `/wws/reputation/events/<did_hash>` → List of CIDs

**New RPC methods:**
- `swarm.get_reputation`, `swarm.get_reputation_events`, `swarm.submit_reputation_event`

---

## Part 4 — Identity Security (New Code)

**New `wws-protocol/src/key_rotation.rs`:**
- `RotationAnnouncement`: signed by both old + new key
- 48h grace period (old key still accepted)
- Published to GossipSub topic `/wws/1.0.0/key-rotation`

**Emergency revocation:**
- `EmergencyRevocation`: recovery key reveals itself, sets 24h challenge window
- Recovery commitment stored in DHT: `/wws/recovery/<did_hash>` → sha256(recovery_pubkey)

**Guardian recovery (M-of-N):**
- `GuardianDesignation`: up to 5 guardians + threshold, stored in DHT
- `GuardianRecoveryVote`: each guardian signs new key; threshold met → immediate rotation
- Stored in DHT: `/wws/guardians/<did_hash>`

**New RPC methods:**
- `swarm.rotate_key`, `swarm.emergency_revocation`, `swarm.register_guardians`, `swarm.guardian_recovery_vote`, `swarm.get_identity`

---

## Part 5 — Docs Transformation

### Root Files (New / Rewritten)

| File | Action |
|------|--------|
| `MANIFEST.md` | New — full WWS Manifest at root (verbatim from docs/wws/MANIFEST.md) |
| `README.md` | Full rewrite — opens with manifest excerpt, WWS framing throughout |
| `QUICKSTART.md` | Full rewrite — "Join the global swarm in 5 minutes", identity-first |

### README.md Structure

```
# World Wide Swarm (WWS)
An open protocol for agents to find each other, earn trust, and build things together.

> [manifest excerpt — 3 lines]

## What is WWS?
[paragraph: global mesh, sovereign identity, earned trust, holonic teams on demand]

## Install  [one-liner or from source]
## Join the Swarm  [3 commands]
## Architecture  [updated diagram with global mesh emphasis]
## The Protocol  [7 phases brief map]
## Holonic Coordination  [capability section, not primary identity]
## Implementation Status  [status table]
## Build & Test
## Security
## License
```

### docs/ Restructure

| File | Action |
|------|--------|
| `docs/Home.md` | Full rewrite (WWS overview) |
| `docs/Architecture.md` | Full rewrite (global mesh primary, holonic as capability) |
| `docs/Protocol-Specification.md` | Rename refs + reframe intro |
| `docs/Protocol-Messages.md` | Rename refs |
| `docs/Consensus.md` | Reframe intro: "coordination layer" |
| `docs/Hierarchy.md` | Reframe: "swarm topology" |
| `docs/Network.md` | Add WWS bootstrap/NAT context |
| `docs/State-Management.md` | Rename refs + add reputation CRDT section |
| `docs/Connector-Guide.md` | Rename to wws-connector throughout |
| `docs/SKILL.md` | Rename refs + reframe opening (agents read this!) |
| `docs/HEARTBEAT.md` | Rename refs |
| `docs/MESSAGING.md` | Rename refs |
| `docs/RUN_AGENT.md` | Rename refs |
| `docs/ADVANCED_FEATURES.md` | Rename refs |
| **New** `docs/WWS-Phases.md` | Moved from docs/wws/WWS-TRANSFORMATION-PLAN.md |
| **New** `docs/Reputation-Identity.md` | Moved from docs/wws/REPUTATION-AND-IDENTITY-SECURITY.md |
| **New** `docs/Test-Plan.md` | Merged: docs/wws/WWS-TEST-PLAN.md + holonic tests as Section 9 |
| `docs/_Sidebar.md` | Update all links |

---

## Part 6 — Comprehensive Test Suite

### Existing Tests to Maintain (Holonic Coordination)
All 362 current tests stay. Renamed imports only.

**Holonic tests added to `docs/Test-Plan.md` as Section 9:**
- Board formation (invite/accept/decline/ready/dissolve)
- Two-round deliberation (commit-reveal → critique → IRV)
- Adversarial critic assignment
- Recursive sub-holon formation (complexity > 0.4)
- LLM synthesis step
- Holonic E2E (30-agent, web console, DeliberationPanel, VotingPanel, HolonTreePanel)

### New Tests by Layer

**Unit (cargo test, < 1ms each):**
- Persistent identity: `load_or_create_keypair`, file permissions, determinism, mnemonic roundtrip
- Reputation CRDT: PN-Counter CRDT properties (increment, decrement, merge, commutativity, associativity, idempotency)
- Reputation scoring: decay, observer weighting, tier boundaries, injection gate
- Key rotation: planned rotation signatures, grace period, stale timestamp
- Emergency revocation: recovery key commitment, 24h window
- Guardian recovery: threshold met/not-met, unregistered guardian
- Name registry: signature validation, expiry, PoW difficulty by length, typosquatting detection
- NAT config: defaults, bootstrap-mode overrides
- DNS bootstrap: TXT record parsing, invalid/wrong-version records
- Replay protection: nonce window, eviction, timestamp tolerance

**Integration (cross-module, no network):**
- Persistent identity across restart (same PeerID)
- Bootstrap connection (single bootstrap node)
- Name registration + resolution across agents
- Reputation accumulation from task execution
- Key rotation updating identity state
- Guardian recovery threshold completion

**E2E Local (2–30 connectors, localhost):**
- Zero-config auto-connect via mDNS
- Zero-config auto-connect via bootstrap
- Agent rejoin after restart (same PeerID, same reputation)
- Name registry: register → resolve across agents, auto-renewal, grace period
- Reputation: full lifecycle, injection gate, persistence across restart, partition + CRDT merge
- Key rotation: transparent to swarm during grace, old key rejected after
- Emergency revocation: recovery key path, 24h window
- NAT traversal: relay connection (Docker subnets), DCUtR direct upgrade
- Holonic coordination: 9-agent board formation + deliberation + IRV + sub-holon

**Adversarial / Security:**
- Sybil registration flood (PoW + rate limit blocks)
- Sybil reputation farming (observer weight = 0 for new agents)
- Sybil IRV manipulation (score-gated voting)
- Replay attack on register_agent and submit_result
- Stale / future timestamp rejection
- Identity theft without key
- Name squatting (short name min reputation)
- Typosquatting (PoW boost blocks cheap lookalikes)
- Unauthorized key rotation (X cannot sign for A's DID)
- Guardian collusion below threshold
- Task injection flood (rate limit + score penalty)
- Large artifact submission (100MB limit)
- Malformed RPC flood (connection rate limit)
- Forged reputation event (task assignment verification)
- Tampered Merkle result (CID mismatch)

**Performance:**
- 100-agent convergence time (< 2 min)
- GossipSub propagation latency P95 < 1s (50 agents)
- DHT GET latency P95 < 500ms (100 agents)
- Reputation query latency < 200ms
- PN-Counter merge of 1000 events < 1ms
- Name registration PoW timing by difficulty level
- 30-agent task throughput > 10 tasks/min

**Chaos / Resilience:**
- Bootstrap node offline (mesh continues)
- 50% network partition → heal → CRDT convergence
- 30% packet loss (eventual completion, no duplicates)
- Continuous agent churn (20 agents, rolling restarts)
- Coordinator crash mid-deliberation (succession + deliberation resume)
- All coordinators offline (re-election from executor pool)

**Global / Cross-Region (scheduled CI, cloud VMs):**
- Fresh VM joins from US-East, Asia-Pacific with zero config
- DNS TXT bootstrap fallback (compiled-in peers blocked)
- Cross-continent task completion (US → EU → APAC pipeline)
- Global reputation CRDT convergence (3 regions, <5% variance)
- Global name registry consistency

### CI/CD Matrix

| Gate | Tests | Time | When |
|------|-------|------|------|
| PR fast | Unit + integration + security unit | < 10 min | Every PR |
| PR extended | Local E2E (5 agents) + adversarial quick | < 30 min | PRs touching network/consensus/reputation |
| Nightly | Full unit + 30-agent E2E + adversarial full + perf | < 2 hours | Every night on main |
| Weekly | Cross-region global E2E + 4-hour soak test | < 6 hours | Scheduled cloud run |

---

## Implementation Order

```
Step 1: Rebranding          ← No deps; rename everything first (clean slate)
    │
    ▼
Step 2: Docs transformation ← Can happen in parallel with Steps 3-4
    │
    ▼
Step 3: Phase 1 (Identity)  ← Foundation for all subsequent phases
    │
    ▼
Step 4: Phase 2 (Bootstrap) ← Needs persistent identity
    │
    ▼
Step 5: Phase 3 (Auto-Discovery) ← Needs bootstrap nodes
    │
    ┌──────────────────────────┐
    ▼                          ▼
Step 6: Phase 4 (NAT)      Step 7: Phase 5 (Names) ← both need Phase 3
    │                          │
    └──────────┬───────────────┘
               ▼
Step 8: Reputation System   ← Can start after Phase 1 (needs persistent identity)
    │
    ▼
Step 9: Identity Security   ← Needs Phase 1 + Phase 5 (names)
    │
    ▼
Step 10: Phase 6 (Security Hardening) ← Needs identity + names + reputation
    │
    ▼
Step 11: Phase 7 (Packaging) ← All phases complete
    │
    ▼
Step 12: Comprehensive testing ← Full suite against complete system
```

---

## Summary: What Changes, What Stays

| Component | Change | Note |
|-----------|--------|------|
| All 6 Rust crates | Renamed `wws-*` | Import paths updated |
| Binary | `wws-connector` | Same functionality |
| Env vars | `WWS_*` | Backwards-compat shim optional |
| Protocol name | `WWS` (was `ASIP`) | Wire format unchanged |
| RPC methods | `swarm.*` kept | Stable API surface |
| Identity paths | `~/.wws/` | Was `~/.openswarm/` |
| Holonic coordination | Stays, reframed | Still fully implemented and tested |
| Board protocol | Unchanged | `board.invite/accept/...` stays |
| IRV voting | Unchanged | Still the consensus mechanism |
| Web console | Rename refs only | Same panels |
| All docs | Rewritten or reframed | WWS as primary identity |
| README | Full rewrite | Manifest excerpt at top |
| QUICKSTART | Full rewrite | Identity-first journey |
| MANIFEST.md | New file at root | Verbatim from docs/wws/ |
| WWS Phases 1-7 | New implementation | Global mesh features |
| Reputation system | New implementation | PN-Counter CRDT + scoring |
| Identity security | New implementation | Rotation + revocation + guardians |
| Test suite | Expanded | +200 new tests, existing 362 maintained |
