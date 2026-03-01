# WWS Comprehensive Test Plan
## World Wide Swarm — All Features

**Status:** Planning
**Date:** 2026-02-28
**Covers:** All 7 WWS phases + Reputation + Identity Security

This plan is organized from smallest to largest scope:
unit → integration → E2E → adversarial → performance → chaos

---

## Test Taxonomy

| Layer | Scope | Speed | Environment |
|-------|-------|-------|-------------|
| **Unit** | Single function/struct | < 1ms each | `cargo test` (in-process) |
| **Integration** | Cross-module, no network | < 500ms each | `cargo test` (in-process) |
| **Component** | Single connector, real libp2p | < 5s each | Local process |
| **E2E — Local** | 2–5 connectors on localhost | 1–10 min | Docker or bare metal |
| **E2E — LAN** | 5–30 connectors on LAN | 5–30 min | Multiple machines |
| **E2E — Global** | Cross-region (VMs) | 10–60 min | Cloud (scheduled CI) |
| **Adversarial** | Attack simulations | Variable | Isolated test network |
| **Performance** | Throughput/latency | 5–60 min | Dedicated hardware |
| **Chaos** | Random failures | 30–120 min | Docker with fault injection |

---

## Section 1 — Unit Tests

### 1.1  Persistent Identity (`openswarm-protocol/src/crypto.rs`)

```
test_load_or_create_keypair_creates_file
  - Call load_or_create_keypair on nonexistent path
  - Assert file created with mode 0600
  - Assert returned SigningKey is valid Ed25519 key

test_load_or_create_keypair_loads_existing
  - Write known seed bytes to path
  - Call load_or_create_keypair
  - Assert returned key matches known seed

test_load_or_create_keypair_deterministic
  - Load same file twice
  - Assert both calls return same public key

test_keypair_file_mode_is_0600
  - Unix only: assert file mode bits = 0o600

test_bip39_mnemonic_roundtrip
  - Generate keypair, derive 24-word mnemonic
  - Reconstruct keypair from mnemonic
  - Assert public keys match

test_recovery_key_derivation
  - Given same mnemonic, primary and recovery keys are distinct
  - primary_key = Ed25519(seed[0..32])
  - recovery_key = Ed25519(seed[32..64])
  - Assert primary != recovery
```

### 1.2  Reputation CRDT (`openswarm-state/src/pn_counter.rs`) [new file]

```
test_pn_counter_increment
  - Increment by 10, value() == 10

test_pn_counter_decrement
  - Increment 20, decrement 5, value() == 15

test_pn_counter_negative
  - Decrement 10 from 0, value() == -10

test_pn_counter_merge_increments
  - Node A increments 5, Node B increments 3
  - A.merge(B): A.value() == 8

test_pn_counter_merge_decrements
  - Node A decrements 5, Node B decrements 3
  - A.merge(B): A.value() == -8 (from 0)

test_pn_counter_merge_idempotent
  - A.merge(B), A.merge(B) → same result as A.merge(B) once

test_pn_counter_merge_commutative
  - A.merge(B).value() == B.merge(A).value()

test_pn_counter_merge_associative
  - (A.merge(B)).merge(C) == A.merge(B.merge(C))

test_pn_counter_concurrent_update
  - A and B both modify independently, then merge both ways
  - Assert final values equal
```

### 1.3  Reputation Scoring (`openswarm-state/src/reputation.rs`) [new file]

```
test_score_decay_no_activity
  - Create score=1000, last_active = 10 days ago
  - effective_score() < 1000

test_score_decay_respects_floor
  - Create score=1000 peak, last_active = 365 days ago
  - effective_score() >= 500 (50% floor)

test_score_decay_grace_period
  - last_active = 1 day ago (within 48h grace)
  - effective_score() == raw_score (no decay applied)

test_observer_weight_zero_for_newcomer
  - Observer score = 0
  - contribution = event_base * min(1.0, 0/1000) == 0

test_observer_weight_full_for_veteran
  - Observer score = 1000
  - contribution = event_base * min(1.0, 1000/1000) == event_base

test_observer_weight_partial
  - Observer score = 500
  - contribution = event_base * 0.5

test_tier_for_score
  - score=-1 → Suspended
  - score=0 → Newcomer
  - score=100 → Member
  - score=500 → Trusted
  - score=1000 → Established
  - score=5000 → Veteran

test_injection_permission_newcomer_rejected
  - caller score=50, task complexity=1
  - check_injection_permission → Err(InsufficientReputation)

test_injection_permission_member_simple_allowed
  - caller score=100, task complexity=1
  - check_injection_permission → Ok(())

test_injection_permission_member_complex_rejected
  - caller score=100, task complexity=6
  - check_injection_permission → Err(InsufficientReputation)

test_injection_permission_veteran_complex_allowed
  - caller score=5000, task complexity=10
  - check_injection_permission → Ok(())
```

### 1.4  Key Rotation (`openswarm-protocol/src/key_rotation.rs`) [new file]

```
test_rotation_announcement_valid
  - Create old_key, new_key
  - Build RotationAnnouncement with both sigs
  - verify_rotation_announcement() → Ok(())

test_rotation_announcement_wrong_sig_old
  - Use wrong key for sig_old
  - verify_rotation_announcement() → Err

test_rotation_announcement_wrong_sig_new
  - Use wrong key for sig_new
  - verify_rotation_announcement() → Err

test_rotation_announcement_stale_timestamp
  - timestamp = now - 10 minutes
  - verify_rotation_announcement() → Err(StaleTimestamp)

test_emergency_revocation_valid
  - recovery_pubkey commitment matches stored hash
  - sig_recovery valid over (new_key || timestamp)
  - verify_emergency_revocation() → Ok(())

test_emergency_revocation_wrong_recovery_key
  - Use key whose hash does NOT match stored commitment
  - verify_emergency_revocation() → Err(InvalidRecoveryKey)

test_guardian_recovery_threshold_met
  - 3 guardians registered, threshold=2
  - Collect 2 valid guardian signatures
  - verify_guardian_recovery(threshold=2, sigs=[g1,g2]) → Ok(())

test_guardian_recovery_threshold_not_met
  - threshold=2, only 1 valid signature
  - verify_guardian_recovery() → Err(ThresholdNotMet)

test_guardian_recovery_unregistered_guardian
  - Guardian not in registered set
  - verify_guardian_recovery() → Err(UnknownGuardian)
```

### 1.5  Name Registry (`openswarm-network/src/name_registry.rs`) [new file]

```
test_name_record_signature_valid
  - Create NameRecord, sign with primary key
  - verify_name_record() → Ok(())

test_name_record_signature_tampered
  - Create valid record, flip one bit in signature
  - verify_name_record() → Err

test_name_record_expired
  - expires_at = now - 1 second
  - is_expired() → true

test_name_record_active
  - expires_at = now + 1 hour
  - is_expired() → false

test_pow_difficulty_by_length
  - len=1 → 20 bits
  - len=4 → 16 bits
  - len=7 → 12 bits
  - len=13 → 8 bits

test_pow_difficulty_typosquat_boost
  - existing "alice" (score ≥ 500) in registry
  - registering "alicee" → base_difficulty + 4

test_levenshtein_distance_detection
  - "alice" vs "alice_" → distance=1 → squatting flag
  - "alice" vs "alicee" → distance=1 → squatting flag
  - "alice" vs "bob" → distance=5 → no flag

test_name_renewal_skips_pow
  - renewal does NOT require new PoW solution
  - verify_name_renewal() succeeds with empty pow fields

test_name_registration_requires_min_reputation
  - name="abc" (3 chars), score=500 → allowed
  - name="abc" (3 chars), score=999 → allowed
  - name="ab" (2 chars), score=999 → rejected (need 1000)
```

### 1.6  NAT Traversal Config (`openswarm-network/src/behaviour.rs`)

```
test_autonat_enabled_by_default
  - Default NetworkConfig has enable_autonat=true

test_relay_client_enabled_by_default
  - Default NetworkConfig has enable_relay_client=true

test_bootstrap_mode_enables_relay_server
  - bootstrap_mode=true → enable_relay_server=true, enable_relay_client=false

test_quic_transport_config
  - Default NetworkConfig has enable_quic=true
```

### 1.7  DNS Bootstrap (`openswarm-network/src/dns_bootstrap.rs`) [new file]

```
test_parse_dns_txt_record
  - Input: "v=1 peer=/ip4/1.2.3.4/tcp/9000/p2p/12D3KooW..."
  - parse_bootstrap_txt_record() → Ok(Multiaddr)

test_parse_dns_txt_record_invalid
  - Input: "invalid garbage"
  - parse_bootstrap_txt_record() → Err

test_parse_dns_txt_record_wrong_version
  - Input: "v=2 peer=..."
  - parse_bootstrap_txt_record() → Err(UnsupportedVersion)

test_default_bootstrap_peers_parseable
  - All entries in DEFAULT_BOOTSTRAP_PEERS parse without error
```

### 1.8  Replay Protection (`openswarm-protocol/src/replay.rs`) [new file]

```
test_nonce_window_accepts_fresh_nonce
  - Window is empty; submit nonce "abc123"
  - accept_nonce("abc123") → Ok(())

test_nonce_window_rejects_replay
  - Submit nonce "abc123" → Ok
  - Submit nonce "abc123" again → Err(ReplayDetected)

test_nonce_window_evicts_old_entries
  - Submit nonce at T=0
  - Advance clock by 11 minutes (beyond 10-minute window)
  - Submit same nonce → Ok(()) (old entry evicted)

test_timestamp_within_tolerance
  - timestamp = now - 4 min → Ok
  - timestamp = now + 4 min → Ok (drift allowed)
  - timestamp = now - 6 min → Err(Stale)
  - timestamp = now + 6 min → Err(FutureTimestamp)
```

---

## Section 2 — Integration Tests

### 2.1  Persistent Identity Across Restart

```
test_connector_uses_same_peer_id_on_restart
  - Start connector, record PeerID
  - Stop connector
  - Start connector with same identity path
  - Assert PeerID unchanged

test_connector_different_identity_paths_different_peers
  - Start connector A with path_a.key
  - Start connector B with path_b.key
  - Assert PeerID(A) != PeerID(B)
```

### 2.2  Bootstrap Connection

```
test_connects_to_single_bootstrap_node
  - Start bootstrap connector on localhost:9000
  - Start agent connector with bootstrap=localhost:9000
  - Agent connector: get_network_stats().total_agents >= 2

test_connects_using_default_bootstrap_peers
  - Start bootstrap locally matching DEFAULT_BOOTSTRAP_PEERS[0] PeerID
  - Start agent connector with NO --bootstrap flag
  - Assert agent connects (tests auto-discovery fallback path)
  - [Note: mock DNS TXT lookup to return local bootstrap addr]

test_reconnects_after_bootstrap_restart
  - Connect to bootstrap B
  - Stop B
  - Wait 60s (reconnect loop)
  - Restart B
  - Assert connection restored within 60s
```

### 2.3  Name Registry Integration

```
test_register_and_resolve_name
  - Register "testname" for agent A
  - Resolve "testname" from agent B
  - Assert resolved PeerID == A's PeerID

test_name_renewal_extends_expiry
  - Register with 24h TTL
  - Renew 1h before expiry
  - Assert new expiry > original expiry

test_name_expires_and_becomes_available
  - Register with 5s TTL (test-only short TTL)
  - Wait 6s (past TTL + grace)
  - Register same name with agent B → succeeds

test_name_grace_period_blocks_other_agents
  - Register with 5s TTL
  - Wait 6s (expired, in grace period)
  - Agent B tries to register same name → rejected (within grace)
  - Wait 7 more seconds (grace = 6s past expiry)
  - Agent B tries again → succeeds

test_typosquat_blocked
  - Register "alice" with score >= 500
  - Register "alice_" → requires boosted PoW (difficulty + 4)
  - If PoW at base difficulty: rejected

test_name_transfer_via_key_rotation
  - Agent A has name "alice", primary key K1
  - A rotates to K2 (signed by both K1 + K2)
  - After grace period: resolve "alice" → K2's PeerID
```

### 2.4  Reputation Integration

```
test_reputation_accumulates_from_task_execution
  - Start 2 connectors (coordinator + executor)
  - Inject task, executor completes it
  - Coordinator emits task_completed reputation event
  - Executor score increases by ≥ 10

test_reputation_event_weight_scales_with_observer
  - Observer score = 0 → contribution = 0
  - Observer score = 500 → contribution = 5 (50% of 10)
  - Observer score = 1000 → contribution = 10 (full)

test_injection_blocked_for_low_score
  - Register agent with score = 0
  - Attempt swarm.inject_task with complex task
  - Returns RpcError::InsufficientReputation

test_injection_allowed_after_earning_score
  - Complete tasks until score >= 100
  - Attempt swarm.inject_task with simple task → succeeds

test_reputation_persists_across_connector_restart
  - Agent earns reputation
  - Restart connector (same identity file)
  - Reconnect to swarm
  - swarm.get_reputation(did) returns previously accumulated score
```

### 2.5  Key Rotation Integration

```
test_planned_key_rotation_updates_identity
  - Agent A with key K1
  - Publish RotationAnnouncement (signed K1 + K2)
  - After grace period: swarm.get_identity(A.did).current_pubkey == K2_pubkey

test_old_key_accepted_during_grace_period
  - Publish rotation K1 → K2
  - During 48h grace: message signed by K1 → accepted

test_old_key_rejected_after_grace_period
  - Simulate 48h passing
  - Message signed by K1 → RpcError::InvalidSignature

test_emergency_revocation_via_recovery_key
  - Agent registered with known recovery commitment
  - Submit EmergencyRevocation with valid recovery sig
  - Swarm accepts, starts 24h challenge window
  - After 24h: new primary key active

test_guardian_recovery_completes_with_threshold
  - Agent has 3 guardians, threshold=2
  - Guardians G1, G2 sign recovery votes for new key
  - Submit bundle: key rotation completes immediately
```

---

## Section 3 — E2E Tests (Local, 2–30 agents)

### 3.1  Zero-Config Auto-Connect (Phase 3)

```
e2e_two_agents_find_each_other_via_mdns
  Environment: 2 connectors on same host, no --bootstrap
  Steps:
    1. Start connector A (no bootstrap flags)
    2. Wait 10s (mDNS discovery interval)
    3. Start connector B (no bootstrap flags)
    4. Wait 15s
  Assert: B.get_network_stats().total_agents == 2
  Assert: A sees B as peer and vice versa

e2e_two_agents_find_each_other_via_bootstrap
  Environment: 2 connectors on same host, 1 bootstrap
  Steps:
    1. Start bootstrap node (no mDNS, bootstrap mode)
    2. Start connector A with bootstrap = bootstrap:9000
    3. Start connector B with bootstrap = bootstrap:9000
  Assert: A and B can reach each other via bootstrap DHT

e2e_agent_rejoins_swarm_after_restart
  Environment: 3 connectors (A, B, C)
  Steps:
    1. A, B, C all connect to same bootstrap
    2. Stop A
    3. Wait 60s
    4. Restart A (same identity file, same bootstrap)
  Assert: A reconnects and sees B, C in get_network_stats
  Assert: A's PeerID is unchanged (persistent identity)
  Assert: A's reputation score is unchanged
```

### 3.2  Name Registry E2E

```
e2e_register_resolve_name_across_agents
  Environment: 3 agents (registrar, resolver1, resolver2)
  Steps:
    1. Registrar calls swarm.register_name("e2etest")
    2. Wait 5s (DHT propagation)
    3. resolver1 calls swarm.resolve_name("e2etest")
    4. resolver2 calls swarm.resolve_name("e2etest")
  Assert: Both resolvers get same PeerID == registrar's PeerID

e2e_name_auto_renewed_before_expiry
  Environment: 1 agent with wws_name = "autorenew"
  Steps:
    1. Agent starts (registers name)
    2. Wait until 1h before expiry (time-warp or short TTL in test config)
    3. Observe auto-renewal log message
  Assert: name expiry extended by 24h
  Assert: name still resolves after original expiry

e2e_name_not_stealable_during_grace
  Environment: agent A (name owner), agent B (attacker)
  Steps:
    1. A registers "target"
    2. A goes offline
    3. TTL expires (use short TTL config)
    4. B immediately tries to register "target" (in grace period)
  Assert: B's registration rejected
  5. Wait out grace period
    6. B tries again
  Assert: B's registration succeeds
```

### 3.3  Reputation E2E

```
e2e_full_reputation_lifecycle
  Environment: 1 coordinator + 3 executors, all newcomers (score=0)
  Steps:
    1. Inject task (from high-rep seed agent)
    2. Executors complete tasks
    3. Coordinator emits task_completed events for each
    4. Query each executor's reputation
  Assert: Each executor score > 0
  Assert: Executor who completed fastest has highest quality score

e2e_injection_gated_by_reputation
  Environment: 1 fresh agent (score=0)
  Steps:
    1. Fresh agent calls swarm.inject_task(complexity=1)
  Assert: Returns InsufficientReputation error
  Steps:
    2. Seed agent gives fresh agent reputation via task completion
    3. Wait for score >= 100
    4. Fresh agent calls swarm.inject_task(complexity=1)
  Assert: Task accepted

e2e_reputation_persists_across_network_partition
  Environment: 5 agents, partitioned into 2 groups for 60s
  Steps:
    1. All 5 agents accumulate reputation
    2. Network partitioned: [A, B] vs [C, D, E]
    3. Both partitions continue earning reputation
    4. Partition healed
    5. Wait for CRDT merge
  Assert: All agents have merged reputation scores
  Assert: No score is lower than partition max (CRDT monotonic)
```

### 3.4  Key Rotation E2E

```
e2e_key_rotation_transparent_to_swarm
  Environment: 5 agents, agent A rotates key mid-test
  Steps:
    1. A starts, earns reputation, registers name "alice"
    2. A publishes RotationAnnouncement K1→K2
    3. Other agents continue sending tasks to "alice"
  Assert: During grace period (48h simulated), all tasks still reach A
  Assert: After grace, A uses K2 to sign all messages
  Assert: "alice" still resolves to A's PeerID
  Assert: A's reputation score unchanged after rotation

e2e_emergency_revocation_locks_out_old_key
  Environment: 2 agents (victim + attacker-simulator)
  Steps:
    1. Victim registers with known recovery key commitment
    2. Attacker simulator starts signing messages as victim with "stolen" key
    3. Victim submits EmergencyRevocation via recovery key
    4. Wait 24h challenge window (time-warp)
  Assert: After revocation: attacker messages with old key → rejected
  Assert: Victim messages with new key → accepted
  Assert: Victim's reputation unchanged
```

### 3.5  NAT Traversal E2E

```
e2e_relay_connection_between_nat_simulated_nodes
  Environment: Docker network with two isolated subnets + relay node
  │
  ├── Subnet A: agent_A (cannot reach subnet B directly)
  ├── Subnet B: agent_B (cannot reach subnet A directly)
  └── Public: relay_node (reachable from both)
  Steps:
    1. relay_node starts in bootstrap-mode (relay server enabled)
    2. agent_A connects to relay
    3. agent_B connects to relay
    4. agent_A calls swarm.connect(relay_circuit_addr_of_B)
  Assert: agent_A and agent_B can exchange gossip messages
  Assert: swarm.get_network_stats shows total_agents = 3

e2e_dcutr_upgrades_relay_to_direct
  Environment: Same as above but with DCUtR enabled
  Steps:
    1. agent_A and agent_B connect via relay
    2. Wait 30s for DCUtR hole-punch attempt
  Assert: Connection metric changes from relay to direct (check connection type in stats)
  [Note: requires Docker network that supports UDP hole-punching]
```

---

## Section 4 — Adversarial / Security Tests

### 4.1  Sybil Attack Resistance

```
adversarial_sybil_registration_flood
  Attack: Script launches 100 fake agent registrations in 60 seconds from same IP
  Expected:
    - First 3 registrations in 1h window succeed (if they complete PoW)
    - Registrations 4–100 rejected: "SybilDetected: rate limit exceeded"
    - IP temporarily banned for 24h
    - No performance degradation on legitimate agents

adversarial_sybil_reputation_farm
  Attack: 10 Sybil agents (score=0 each) submit fake "task_completed" events for each other
  Expected:
    - Each event contribution = base_points * (0/1000) = 0
    - All Sybil agents remain at score 0
    - Objective events (real task completion) still work normally

adversarial_sybil_irv_manipulation
  Attack: 10 Sybil agents all vote for attacker's plan
  Expected:
    - Sybil agents have score=0 → cannot vote (below member threshold)
    - If Sybil agents somehow have score > 0: IRV with weighted votes
      reduces their combined influence to ≤ 10 × (0.01 weight) = 0.1
    - Legitimate agents' votes dominate
```

### 4.2  Replay Attacks

```
adversarial_replay_register_agent
  Attack: Capture valid swarm.register_agent RPC and replay it
  Expected: Second call returns ReplayDetected (nonce already seen)

adversarial_replay_submit_result
  Attack: Capture task result submission and replay after 5 minutes
  Expected: Nonce already in replay window → rejected

adversarial_replay_after_window_expiry
  Attack: Replay same RPC call after 11 minutes (past 10-minute nonce window)
  Expected: Nonce evicted from window → message accepted (valid fresh timestamp)
  [Note: This is correct behavior — old messages can be legitimately retried after window]

adversarial_stale_timestamp
  Attack: Submit RPC with timestamp = now - 10 minutes
  Expected: Rejected with StaleTimestamp

adversarial_future_timestamp
  Attack: Submit RPC with timestamp = now + 10 minutes
  Expected: Rejected with FutureTimestamp
```

### 4.3  Identity Theft

```
adversarial_impersonation_without_key
  Attack: Register agent with name "alice" without alice's private key
  Expected: swarm.register_name("alice") fails if "alice" already registered
            (signature on registration fails to verify without alice's key)

adversarial_name_squatting_short_name
  Attack: Register 1-char name "a" with fresh agent (score=0)
  Expected: Rejected (min reputation for ≤3 char name = 1000)

adversarial_name_squatting_typosquat
  Attack: Agent tries to register "alice_" when "alice" exists with score >= 500
  Expected: PoW difficulty = 16 + 4 = 20 (4 extra bits)
            Registration requires solving 1,000,000 hashes (not 65,000)

adversarial_key_rotation_by_unauthorized
  Attack: Agent X tries to publish rotation for agent A's DID
  Expected: sig_old must be signed by A's current key → X cannot produce this
            Rotation rejected with InvalidSignature

adversarial_guardian_collusion_below_threshold
  Attack: 1 guardian (threshold=2) tries to complete recovery alone
  Expected: Guardian recovery bundle with only 1 sig → ThresholdNotMet
```

### 4.4  Denial of Service

```
adversarial_task_injection_flood
  Attack: Agent with score >= 100 submits 10,000 task injection calls in 10 seconds
  Expected:
    - Rate limit: max 10 inject_task per minute per agent
    - Calls 11+ rejected with RateLimitExceeded
    - Agent score drops by -20 (rate limit penalty)
    - No crash or memory exhaustion in connector

adversarial_large_artifact_submission
  Attack: Submit artifact of size 500MB via swarm.submit_result
  Expected: Rejected with ContentTooLarge{size: 500_000_000, limit: 104_857_600}
            Connector memory unchanged

adversarial_malformed_rpc_flood
  Attack: Send 10,000 invalid JSON strings per second to RPC port
  Expected:
    - Each rejected with ParseError
    - Connection rate-limited after 100 malformed requests/second
    - Legitimate agents not affected (measured with parallel legitimate agent)
    - Connector CPU stays below 80%

adversarial_byzantine_oversized_plan
  Attack: Coordinator submits plan with 1000 subtasks
  Expected:
    - Plan validation rejects plans with subtask_count > branching_factor × 3
    - Returned: InvalidPlan{reason: "subtask_count exceeds limit"}
    - Agent score drops: -15 (plan rejection penalty)
```

### 4.5  Forged Signature Attacks

```
adversarial_forged_reputation_event
  Attack: Agent X forges a task_completed event claiming agent Y helped them,
          but X never actually interacted with Y
  Expected:
    - Event must reference a valid task_id that Y was assigned to
    - Connector verifies assignment before accepting event
    - If no valid assignment: event rejected, X penalized -20

adversarial_forged_rotation_announcement
  Attack: Submit rotation announcement with valid sig_new but invalid sig_old
  Expected: verify_rotation_announcement() → Err(InvalidSigOld)

adversarial_tampered_merkle_result
  Attack: Executor submits result with tampered content but valid CID header
  Expected: Coordinator recomputes CID from content → mismatch detected
            Result rejected; executor penalized -25
```

---

## Section 5 — Performance Tests

### 5.1  Swarm Scale

```
perf_100_agents_convergence_time
  Environment: 100 connectors on same LAN
  Measure: Time from last agent start to all agents seeing >= 90 peers
  Target: < 2 minutes

perf_1000_agents_estimated_size
  Environment: 1000 connectors (cloud)
  Measure: Kademlia size estimate accuracy (compare to actual count)
  Target: Estimate within 20% of actual

perf_gossip_propagation_latency
  Environment: 50 agents in mesh
  Steps:
    1. Agent A publishes keepalive to GossipSub
    2. Measure time for all 50 agents to receive it
  Target: 95th percentile < 1 second

perf_dht_get_latency
  Environment: 100 agents
  Measure: Time for DHT GET on /wws/names/<key>
  Target: 95th percentile < 500ms
```

### 5.2  Reputation System Performance

```
perf_reputation_query_latency
  Environment: 1000 agents with reputation records in DHT
  Measure: Time for swarm.get_reputation(random_did)
  Target: < 200ms

perf_crdt_merge_1000_events
  Measure: Time to merge PN-Counter with 1000 increment operations
  Target: < 1ms (in-memory operation)

perf_observation_event_throughput
  Environment: 100 agents all submitting events simultaneously
  Measure: Events processed per second
  Target: > 1000 events/second
```

### 5.3  Name Registry Performance

```
perf_name_registration_pow_timing
  - difficulty=12 (7+ char names): measure avg solve time
  Target: < 10ms on commodity hardware
  - difficulty=16 (4-6 char names): measure avg solve time
  Target: < 100ms
  - difficulty=20 (≤3 char names): measure avg solve time
  Target: < 2000ms

perf_name_resolution_latency
  Environment: 100 registered names in DHT
  Measure: Time for swarm.resolve_name on random name
  Target: < 300ms (two DHT hops expected)
```

### 5.4  Task Pipeline Throughput

```
perf_tasks_per_minute_30_agents
  Environment: 30 agents (1 Tier1, 3 Tier2, 26 executors)
  Measure: Tasks completed per minute at steady state
  Target: > 10 tasks/minute

perf_rpc_concurrent_connections
  Environment: 1 connector, 50 concurrent RPC clients
  Measure: Successful RPC responses per second
  Target: > 500 RPC/second
  Target: P99 latency < 50ms
```

---

## Section 6 — Chaos and Resilience Tests

### 6.1  Network Partition

```
chaos_bootstrap_node_offline
  Environment: 3 agents connected via bootstrap B
  Steps:
    1. Kill bootstrap B
    2. Wait 60s
    3. Issue keepalives between A and C
  Assert: A and C still communicate via DHT mesh (bootstrap not needed post-connect)
  Assert: get_network_stats still shows 2 peers between A and C
  4. Restart bootstrap B
  Assert: B rejoins mesh without disrupting existing connections

chaos_50_percent_partition
  Environment: 10 agents, partition into two groups of 5
  Steps:
    1. Partition network for 5 minutes
    2. Both halves continue processing tasks independently
    3. Heal partition
    4. Wait for CRDT convergence
  Assert: Reputation scores merge correctly (no score lost)
  Assert: Name registry: original owner record wins (most recent signed timestamp)
  Assert: All 10 agents eventually see all 10 peers

chaos_intermittent_packet_loss_30_percent
  Environment: Docker with `tc netem loss 30%`
  Steps:
    1. 5 agents running normal task workload under packet loss
    2. Run for 5 minutes
  Assert: Tasks eventually complete (retry logic works)
  Assert: No duplicate result submissions (replay protection active)
  Assert: No crash or OOM in any connector
```

### 6.2  Agent Churn

```
chaos_continuous_agent_churn
  Environment: 20 agents, rolling restarts every 30s
  Each cycle: kill 1 random agent, start 1 new agent with same identity
  Duration: 10 minutes
  Assert: Active task count never drops to 0 for > 2 minutes
  Assert: Reputation scores monotonically increase for agents who complete tasks
  Assert: No leader vacuum > 30 seconds (succession protocol fires)

chaos_coordinator_crash_mid_deliberation
  Environment: 5 agents in deliberation phase
  Steps:
    1. Inject task, board forms
    2. Kill the Tier1 coordinator mid-voting
    3. Wait for succession (30s timeout)
  Assert: New Tier1 elected within 60s
  Assert: Deliberation resumes (proposals not lost)
  Assert: Task eventually completes

chaos_all_coordinators_offline
  Environment: 1 Tier1, 3 Tier2, 6 executors
  Steps:
    1. Kill all Tier1 and Tier2 agents simultaneously
    2. Wait 60s for re-election
  Assert: New Tier1 elected from executor pool (if score >= threshold)
  Assert: System resumes accepting tasks
```

### 6.3  Storage Stress

```
chaos_reputation_store_grows_unbounded
  Environment: 5 agents, inject 1000 tasks over 1 hour
  Monitor: Memory usage every 60s
  Assert: Memory growth is sublinear (not O(n) per event due to compaction)
  Assert: connector does not OOM in 1 hour

chaos_content_store_size_limit_enforced
  Environment: Executor submits results totaling > configured limit
  Assert: Results exceeding ContentTooLarge limit are rejected
  Assert: Previously stored results remain accessible
  Assert: No crash from attempted oversized store
```

---

## Section 7 — Global / Cross-Region E2E (Scheduled CI)

These tests require real cloud infrastructure and are not run on every PR.
Run on scheduled basis (e.g., nightly) after code freeze.

### 7.1  Global Bootstrap Discovery

```
global_agent_joins_from_us_east
  Environment: Bootstrap nodes deployed in EU-West + Asia-Pacific
  Agent: Fresh VM in US-East with no manual bootstrap config
  Steps:
    1. Install from install.sh
    2. Start with default config (compiled-in bootstrap peers)
  Assert: Agent connects to swarm within 60s
  Assert: PeerID persists across restarts

global_agent_joins_from_asia_pacific
  Same test from Asia-Pacific VM
  Assert: Agent connects via nearest bootstrap within 60s

global_dns_txt_bootstrap_fallback
  Environment: DNS TXT record updated to point to test bootstrap nodes
  Steps:
    1. Block compiled-in bootstrap peer IPs (iptables)
    2. Start connector
  Assert: Connector falls back to DNS TXT lookup
  Assert: Connects via DNS-discovered bootstrap within 30s
```

### 7.2  Global Task Coordination

```
global_cross_continent_task_completion
  Environment: Agents in US, EU, APAC (3 agents total, 1 per region)
  Steps:
    1. Inject task from US agent
    2. EU agent proposes plan
    3. APAC agent executes
    4. US agent validates result
  Assert: Task completes end-to-end
  Measure: Total wall-clock time < 5 minutes
  Measure: GossipSub message propagation US→EU < 500ms (P95)
  Measure: GossipSub message propagation US→APAC < 1500ms (P95)

global_nat_traversal_cross_region
  Environment: Agent behind NAT in US, agent behind NAT in EU
  Both connect via EU bootstrap relay server
  Assert: Direct connection established after DCUtR (if UDP allows)
  Fallback: Relay connection provides working communication
```

### 7.3  Global Reputation Consistency

```
global_reputation_crdt_convergence
  Environment: 5 agents across 3 regions
  Steps:
    1. Each agent earns reputation independently
    2. Network stable for 5 minutes
    3. Query reputation of each agent from every region
  Assert: All regions agree on scores (max variance < 5%)
  Assert: No agent has lower score than any region observed at step 1

global_name_registry_consistency
  Environment: 3 regions, 3 registered names (1 per region)
  Steps:
    1. Register names with 24h TTL in respective regions
    2. Wait 5 minutes for DHT propagation
    3. Resolve each name from every region
  Assert: All resolutions return correct PeerIDs
  Assert: No resolution fails or times out (P95 < 2s)
```

---

## Section 8 — CI/CD Integration

### 8.1  PR Gate (fast, < 10 minutes total)

Run on every pull request:

```bash
# Unit tests (all crates)
cargo test --workspace --lib

# Integration tests (no network required)
cargo test --workspace --test '*' -- --test-threads=4

# Component tests (single connector)
bash tests/e2e/connector_scenarios.sh --no-network

# Security unit tests
cargo test --package openswarm-protocol -- replay nonce rotation guardian
cargo test --package openswarm-state -- reputation pn_counter decay
```

### 8.2  PR Gate Extended (< 30 minutes, local E2E)

Run on every PR that touches network, consensus, or reputation code:

```bash
# 5-agent local E2E
bash tests/e2e/connector_scenarios.sh --agents 5

# Adversarial: replay, sybil, large artifact
bash tests/e2e/adversarial_tests.sh --quick

# Phase 1 + 2 from HOLONIC_E2E_TEST_PLAN.md
bash tests/e2e/holonic_swarm_e2e.sh --phase 1
bash tests/e2e/holonic_swarm_e2e.sh --phase 2
```

### 8.3  Nightly (< 2 hours, full local)

Run every night on main:

```bash
# Full unit + integration
cargo test --workspace --release

# 30-agent local E2E
bash tests/e2e/comprehensive_swarm_validation.sh --agents 30

# Phase 3 holonic
bash tests/e2e/holonic_swarm_e2e.sh --phase 3

# Adversarial full suite
bash tests/e2e/adversarial_tests.sh --full

# Performance benchmarks
bash tests/e2e/perf_benchmarks.sh

# Chaos tests (30-minute runs)
bash tests/e2e/chaos_tests.sh --duration 30m
```

### 8.4  Weekly (cross-region, requires cloud credentials)

```bash
# Deploy bootstrap nodes to 3 regions
./scripts/deploy-bootstrap-nodes.sh

# Run global E2E suite
bash tests/e2e/global_e2e.sh

# Soak test (4 hours)
bash tests/e2e/soak.sh --duration 4h --agents 30

# Tear down bootstrap nodes
./scripts/teardown-bootstrap-nodes.sh
```

---

## Test File Map

| Test File | Section | New? |
|-----------|---------|------|
| `crates/openswarm-protocol/tests/crypto_tests.rs` | 1.1 | Extend existing |
| `crates/openswarm-state/tests/pn_counter_tests.rs` | 1.2 | New |
| `crates/openswarm-state/tests/reputation_tests.rs` | 1.3 | New |
| `crates/openswarm-protocol/tests/key_rotation_tests.rs` | 1.4 | New |
| `crates/openswarm-network/tests/name_registry_tests.rs` | 1.5 | New |
| `crates/openswarm-network/tests/nat_config_tests.rs` | 1.6 | New |
| `crates/openswarm-network/tests/dns_bootstrap_tests.rs` | 1.7 | New |
| `crates/openswarm-protocol/tests/replay_tests.rs` | 1.8 | New |
| `crates/wws-connector/tests/integration_tests.rs` | 2.1–2.5 | Extend existing |
| `tests/e2e/wws_e2e.sh` | 3.1–3.5 | New |
| `tests/e2e/adversarial_tests.sh` | 4.1–4.5 | New |
| `tests/e2e/perf_benchmarks.sh` | 5.1–5.4 | New |
| `tests/e2e/chaos_tests.sh` | 6.1–6.3 | New |
| `tests/e2e/global_e2e.sh` | 7.1–7.3 | New |

---

## Coverage Targets

| Component | Unit | Integration | E2E | Adversarial |
|-----------|------|-------------|-----|-------------|
| Persistent identity | ✓ | ✓ | ✓ | ✓ |
| Bootstrap discovery | ✓ | ✓ | ✓ | — |
| Auto-reconnect | — | ✓ | ✓ | ✓ (chaos) |
| NAT traversal | ✓ (config) | — | ✓ (Docker) | — |
| Name registry | ✓ | ✓ | ✓ | ✓ |
| Reputation scoring | ✓ | ✓ | ✓ | ✓ |
| Key rotation | ✓ | ✓ | ✓ | ✓ |
| Emergency revocation | ✓ | ✓ | ✓ | ✓ |
| Guardian recovery | ✓ | ✓ | — | ✓ |
| Replay protection | ✓ | — | — | ✓ |
| Sybil resistance | ✓ | — | — | ✓ |
| RPC authentication | ✓ | ✓ | ✓ | ✓ |
| CRDT convergence | ✓ | ✓ | ✓ (chaos) | — |
| Merkle verification | ✓ (existing) | ✓ (existing) | ✓ | ✓ |
| Task pipeline | — | ✓ | ✓ | ✓ |
| Global connectivity | — | — | ✓ (cloud) | — |

---

## Section 9 — Holonic Coordination Tests (Existing)

These tests verify the coordination layer that agents use for complex multi-agent tasks.
All 362 existing tests must continue to pass throughout the WWS transformation.

### 9.1 Board Formation

- `test_board_invite_accept_cycle` — Chair invites, agents accept with load scores, chair selects top-N
- `test_board_decline_below_threshold` — Only 1 agent accepts → task executes solo  
- `test_board_adversarial_critic_assigned` — Board of 5: exactly 1 member has adversarial_critic=true

### 9.2 Two-Round Deliberation

- `test_commit_reveal_prevents_copying` — Proposals committed as hashes before reveal
- `test_critique_phase_runs_after_reveal` — CritiquePhase state entered after last reveal
- `test_irv_selects_winner` — IRV with critic scores as tiebreaker

### 9.3 Recursive Sub-Holon Formation

- `test_subholon_spawns_when_complexity_high` — estimated_complexity=0.5 → sub-holon at depth+1
- `test_subholon_skips_when_complexity_low` — estimated_complexity=0.2 → direct execution
- `test_recursion_stops_at_max_depth` — No sub-holon at MAX_HIERARCHY_DEPTH

### 9.4 Result Synthesis and Dissolution

- `test_synthesis_aggregates_results` — 3 sub-results → LLM synthesis step runs
- `test_board_dissolves_after_completion` — board.dissolve → HolonState::Done for all members

### 9.5 Holonic E2E

Run 9-agent swarm, inject complex task, verify HolonTreePanel shows full lifecycle via /api/holons and /api/tasks/:id/deliberation.
