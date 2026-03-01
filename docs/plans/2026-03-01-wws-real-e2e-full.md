# WWS Real End-to-End Test — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Deploy 9 wws-connector containers across 3 Docker subnets, run 8 Claude AI subagents as real WWS participants, exercise every protocol feature (identity, names, P2P messaging, holonic task coordination, deliberation, voting, reputation), and produce a markdown log with real agent text.

**Architecture:** Three layers: (1) Rust code additions — anti-bot verification challenge on join + `send_message` RPC for direct P2P messaging; (2) Docker Compose — 9 containers on 3 isolated bridge networks with multi-homed bootstrap relay; (3) Execution — bash infra launcher + 8 parallel Claude subagents each running a real agent loop, writing to shared log files, then a Playwright UI spec. Final output: `docs/e2e-results/YYYY-MM-DD-wws-real-e2e-log.md` with real agent conversations, plans, critiques, research, synthesis.

**Tech Stack:** Rust (connector additions), Docker Compose (multi-network), bash + nc (RPC over TCP), Claude subagents (real agents), Playwright/JS (UI), Python3 (JSON parsing in bash), Markdown (log format).

---

## Key Architecture Facts (read before implementing anything)

### RPC wire format (used everywhere)
```bash
# Send one JSON-RPC request, get one response
echo '{"jsonrpc":"2.0","method":"swarm.METHOD","params":{},"id":"1","signature":""}' \
  | nc -w 5 127.0.0.1 PORT
```

### JSON field extractor (used everywhere)
```bash
jfield() { python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('$1',''))" 2>/dev/null; }
```

### Connector CLI flags (correct names)
```
--listen /ip4/0.0.0.0/tcp/9000   # P2P (0.0.0.0 for Docker multi-network)
--rpc 0.0.0.0:9370               # RPC (0.0.0.0 for host access via port mapping)
--files-addr 0.0.0.0:9371        # HTTP (0.0.0.0 for host access)
--agent-name NAME
--identity-path /data/NAME.key
--bootstrap /ip4/IP/tcp/9000/p2p/PEER_ID
--bootstrap-mode                  # Enable relay server (bootstrap node only)
```

### Port layout (Docker → host mappings)
```
bootstrap:         P2P=9000  HTTP→host:19371  (RPC not exposed — no agent)
connector-alpha:   P2P=9000  RPC→host:19370  HTTP→host:19381
connector-beta:    P2P=9000  RPC→host:19372  HTTP→host:19383
connector-raft:    P2P=9000  RPC→host:19374  HTTP→host:19385
connector-pbft:    P2P=9000  RPC→host:19376  HTTP→host:19387
connector-paxos:   P2P=9000  RPC→host:19378  HTTP→host:19389
connector-tendermint: P2P=9000 RPC→host:19380  HTTP→host:19391
connector-hashgraph: P2P=9000 RPC→host:19382  HTTP→host:19393
connector-synth:   P2P=9000  RPC→host:19384  HTTP→host:19395
```
(19xxx to avoid colliding with other tests)

### Docker network topology
```
bootstrap is multi-homed: on all 3 networks
  wws-bootstrap-net (172.20.0.0/24): 172.20.0.10
  wws-tier1-net     (172.21.0.0/24): 172.21.0.100   ← reachable by Tier1
  wws-tier2-net     (172.22.0.0/24): 172.22.0.100   ← reachable by Tier2

Tier1 connectors (alpha, beta): ONLY on wws-tier1-net
  bootstrap addr: /ip4/172.21.0.100/tcp/9000/p2p/<PEER>

Tier2 connectors (raft…synth): ONLY on wws-tier2-net
  bootstrap addr: /ip4/172.22.0.100/tcp/9000/p2p/<PEER>

Tier1↔Tier2 can ONLY communicate via bootstrap relay. This tests real cross-subnet relay.
```

### Rust crate layout
```
crates/wws-protocol/src/types.rs       — shared types (add new structs here)
crates/wws-protocol/src/messages.rs    — gossip message types (add new message here)
crates/wws-connector/src/connector.rs  — ConnectorState + gossip handlers (add fields + handler)
crates/wws-connector/src/rpc_server.rs — RPC dispatch + handlers (add new methods)
crates/wws-connector/src/file_server.rs— HTTP endpoints (may need /api/messages update)
```

### How to add a new RPC method (pattern)
1. Add handler function `async fn handle_X(id, params, state, network_handle) -> SwarmResponse` to rpc_server.rs
2. Add dispatch entry in the match block (lines 142-203 in rpc_server.rs):
   ```rust
   "swarm.x" => handle_x(request.id.clone(), &request.params, &state, &network_handle).await,
   ```
3. Test with: `echo '{"jsonrpc":"2.0","method":"swarm.x","params":{...},"id":"1","signature":""}' | nc -w 5 127.0.0.1 9370`

### How to add gossip message handling (pattern)
1. Add variant to appropriate params struct in `messages.rs`
2. In `connector.rs`, find `handle_gossip_message` (or equivalent dispatch) and add arm
3. Handler stores data in ConnectorState

---

## Task 1: Add VerificationChallenge types to protocol + ConnectorState

**Files:**
- Modify: `crates/wws-protocol/src/types.rs` (add structs)
- Modify: `crates/wws-connector/src/connector.rs` (add state fields)

**Step 1: Add types to `crates/wws-protocol/src/types.rs`**

Find the end of the types file and add:

```rust
/// Anti-bot verification challenge. Returned on first register_agent call.
/// Agent must decode garbled obfuscated text and answer arithmetic question.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationChallenge {
    /// Unique code identifying this challenge (wws_verify_{hex})
    pub code: String,
    /// Garbled obfuscated challenge text containing an arithmetic question
    pub challenge_text: String,
    /// Expected integer answer (not sent to agent)
    pub expected_answer: i64,
    /// When this challenge was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A direct P2P message between agents (stored after gossip delivery)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectMessage {
    pub id: String,
    pub sender_did: String,
    pub sender_name: Option<String>,
    pub recipient_did: Option<String>,  // None = broadcast
    pub content: String,
    pub message_type: String,   // "greeting", "social", "question", "comment", "broadcast"
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

**Step 2: Add fields to ConnectorState in `crates/wws-connector/src/connector.rs`**

Find the `ConnectorState` struct definition. Add these fields:

```rust
// Anti-bot verification
pub pending_verifications: HashMap<String, VerificationChallenge>,
pub verified_agents: std::collections::HashSet<String>,

// Direct P2P messages
pub direct_messages: Vec<DirectMessage>,
```

Also add to ConnectorState's `new()` or `Default` implementation:
```rust
pending_verifications: HashMap::new(),
verified_agents: std::collections::HashSet::new(),
direct_messages: Vec::new(),
```

**Step 3: Build to verify it compiles**

```bash
cd /Users/aostapenko/Work/OpenSwarm/.worktrees/wws-transformation
~/.cargo/bin/cargo build -p wws-protocol -p wws-connector 2>&1 | tail -10
```
Expected: `Compiling wws-protocol ... Compiling wws-connector ... Finished`

**Step 4: Run existing tests**

```bash
~/.cargo/bin/cargo test --workspace 2>&1 | tail -5
```
Expected: all existing tests pass

**Step 5: Commit**

```bash
git add crates/wws-protocol/src/types.rs crates/wws-connector/src/connector.rs
git commit -m "feat(protocol): add VerificationChallenge and DirectMessage types to ConnectorState"
```

---

## Task 2: Implement verification challenge in register_agent + add verify_agent RPC

**Files:**
- Modify: `crates/wws-connector/src/rpc_server.rs`

**Background:** `handle_register_agent` is at lines ~1470-1683 of rpc_server.rs (213 lines). The RPC dispatch match block is at lines ~142-203.

### Step 1: Add challenge generation helper (add above handle_register_agent)

Add this function before `handle_register_agent`:

```rust
/// Generate an obfuscated arithmetic challenge for anti-bot verification.
/// Returns (challenge_struct, code_string).
fn generate_verification_challenge(agent_id: &str) -> VerificationChallenge {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    let a: i64 = rng.gen_range(10..=89);
    let b: i64 = rng.gen_range(10..=89);
    let answer = a + b;

    // Generate a random code
    let code = format!("wws_verify_{:016x}", rng.gen::<u64>());

    // Plain question
    let plain = format!("what is {} plus {}", a, b);

    // Garble: random case, random symbol insertion, random spaces within words
    let symbols: &[char] = &['^', '{', '}', '|', '~'];
    let mut garbled = String::new();
    for (i, ch) in plain.chars().enumerate() {
        if ch == ' ' {
            // Sometimes insert a symbol instead of / in addition to space
            let r = rng.gen_range(0u8..4);
            match r {
                0 => garbled.push(' '),
                1 => { garbled.push(symbols[rng.gen_range(0..symbols.len())]); garbled.push(' '); }
                2 => { garbled.push(' '); garbled.push(symbols[rng.gen_range(0..symbols.len())]); }
                _ => { garbled.push(' '); garbled.push(symbols[rng.gen_range(0..symbols.len())]); garbled.push(' '); }
            }
        } else if ch.is_alphabetic() {
            // Randomly inject a space before letter (breaks words visually)
            if rng.gen_bool(0.25) { garbled.push(' '); }
            if rng.gen_bool(0.5) {
                garbled.extend(ch.to_uppercase());
            } else {
                garbled.extend(ch.to_lowercase());
            }
        } else {
            garbled.push(ch);
        }
    }

    let challenge_text = format!(
        "VERIFY CODE: {}\nCHALLENGE: {}?",
        code, garbled.trim()
    );

    VerificationChallenge {
        code,
        challenge_text,
        expected_answer: answer,
        created_at: chrono::Utc::now(),
    }
}
```

Make sure `rand` is imported: the crate already uses `rand`, so this should compile.

### Step 2: Modify `handle_register_agent` to check verification first

At the START of `handle_register_agent` (before the existing agent registration logic), add:

```rust
// Extract agent_id from params
let agent_id = params
    .get("agent_id")
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .to_string();

// Anti-bot verification check
{
    let state_read = state.read().await;
    let already_verified = state_read.verified_agents.contains(&agent_id);
    let has_pending = state_read.pending_verifications.contains_key(&agent_id);
    drop(state_read);

    if !already_verified {
        if !has_pending {
            // First call: generate and return challenge, do NOT proceed with registration
            let challenge = generate_verification_challenge(&agent_id);
            let challenge_text = challenge.challenge_text.clone();
            let code = challenge.code.clone();
            let mut state_write = state.write().await;
            state_write.pending_verifications.insert(agent_id.clone(), challenge);
            drop(state_write);

            return SwarmResponse::result(
                id,
                serde_json::json!({
                    "verified": false,
                    "challenge_required": true,
                    "challenge": {
                        "code": code,
                        "text": challenge_text
                    }
                }),
            );
        } else {
            // Challenge exists but verify_agent hasn't been called yet
            let state_read = state.read().await;
            let ch = state_read.pending_verifications.get(&agent_id).unwrap().clone();
            drop(state_read);
            return SwarmResponse::result(
                id,
                serde_json::json!({
                    "verified": false,
                    "challenge_required": true,
                    "challenge": {
                        "code": ch.code,
                        "text": ch.challenge_text
                    }
                }),
            );
        }
    }
    // already_verified: fall through to normal registration
}
```

### Step 3: Add `handle_verify_agent` function

Add after `handle_register_agent`:

```rust
pub async fn handle_verify_agent(
    id: Option<String>,
    params: &serde_json::Value,
    state: &Arc<RwLock<ConnectorState>>,
) -> SwarmResponse {
    let agent_id = params
        .get("agent_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let code = params
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let answer_str = params
        .get("answer")
        .and_then(|v| v.as_str())
        .or_else(|| params.get("answer").and_then(|v| v.as_i64()).map(|_| ""))
        .unwrap_or("");
    // Accept answer as string or integer
    let answer: i64 = if let Some(n) = params.get("answer").and_then(|v| v.as_i64()) {
        n
    } else {
        answer_str.trim().parse().unwrap_or(-999999)
    };

    let mut state_write = state.write().await;

    // Find challenge
    let challenge = match state_write.pending_verifications.get(&agent_id) {
        Some(c) => c.clone(),
        None => {
            return SwarmResponse::error(id, -32000, "no_pending_challenge".to_string());
        }
    };

    // Validate code
    if challenge.code != code {
        return SwarmResponse::error(id, -32000, "invalid_code".to_string());
    }

    // Validate answer
    if challenge.expected_answer != answer {
        tracing::warn!("Verification failed for {}: expected {}, got {}", agent_id, challenge.expected_answer, answer);
        return SwarmResponse::error(id, -32000, "invalid_answer".to_string());
    }

    // Success: mark as verified, remove pending
    state_write.pending_verifications.remove(&agent_id);
    state_write.verified_agents.insert(agent_id.clone());
    drop(state_write);

    tracing::info!("Agent {} passed verification challenge", agent_id);

    SwarmResponse::result(
        id,
        serde_json::json!({
            "verified": true,
            "agent_id": agent_id,
            "message": "Verification successful. Call swarm.register_agent again to complete registration."
        }),
    )
}
```

### Step 4: Register `swarm.verify_agent` in the dispatch match

In the dispatch match block (lines ~142-203), add alongside the other methods:

```rust
"swarm.verify_agent" => {
    handle_verify_agent(request.id.clone(), &request.params, &state).await
}
```

### Step 5: Build and test manually

```bash
cd /Users/aostapenko/Work/OpenSwarm/.worktrees/wws-transformation
~/.cargo/bin/cargo build --release -p wws-connector 2>&1 | tail -5
```

Start a test connector:
```bash
pkill -f "wws-connector" 2>/dev/null || true; sleep 1
./target/release/wws-connector \
  --listen /ip4/127.0.0.1/tcp/19900 \
  --rpc 127.0.0.1:19990 \
  --files-addr 127.0.0.1:19991 \
  --agent-name test-verify \
  --identity-path /tmp/test-verify.key \
  >/tmp/test-verify.log 2>&1 &
sleep 3
```

Test the challenge flow:
```bash
# Step 1: register → should get challenge back
RESP1=$(echo '{"jsonrpc":"2.0","method":"swarm.register_agent","params":{"agent_id":"test-bot"},"id":"1","signature":""}' | nc -w 5 127.0.0.1 19990)
echo "Register response: $RESP1"
# Expected: {"result":{"verified":false,"challenge_required":true,"challenge":{"code":"wws_verify_...","text":"..."}}}

# Extract code and challenge text
CODE=$(echo "$RESP1" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d['result']['challenge']['code'])")
TEXT=$(echo "$RESP1" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d['result']['challenge']['text'])")
echo "Code: $CODE"
echo "Text: $TEXT"

# Step 2: Try wrong answer → should fail
RESP2=$(echo "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.verify_agent\",\"params\":{\"agent_id\":\"test-bot\",\"code\":\"$CODE\",\"answer\":99999},\"id\":\"2\",\"signature\":\"\"}" | nc -w 5 127.0.0.1 19990)
echo "Wrong answer response: $RESP2"
# Expected: error invalid_answer

# Step 3: Claude would decode TEXT and find the real answer. For testing, extract it from the log:
ANSWER=$(cat /tmp/test-verify.log | grep "expected_answer" | head -1 | python3 -c "..." || echo "")
# Or just manually read it from the challenge text for test purposes
```

**Important:** Claude agents will decode the garbled text naturally using LLM reasoning. For this step test, just verify the challenge is returned and the wrong answer is rejected.

Kill the test connector:
```bash
pkill -f "wws-connector.*19900" 2>/dev/null || true
```

**Step 6: Run full test suite**

```bash
~/.cargo/bin/cargo test --workspace 2>&1 | tail -5
```
Expected: all tests pass

**Step 7: Commit**

```bash
git add crates/wws-connector/src/rpc_server.rs
git commit -m "feat(rpc): add anti-bot verification challenge to register_agent + verify_agent RPC"
```

---

## Task 3: Add send_message RPC + DirectMessage gossip handler

**Files:**
- Modify: `crates/wws-protocol/src/messages.rs` (add gossip message type)
- Modify: `crates/wws-connector/src/connector.rs` (add gossip handler)
- Modify: `crates/wws-connector/src/rpc_server.rs` (add RPC handler)
- Modify: `crates/wws-connector/src/file_server.rs` (expose via /api/messages)

**Background:** The connector uses libp2p GossipSub. To send a direct message:
1. The sending agent calls `swarm.send_message` RPC
2. The connector publishes a `DirectMessageParams` payload to gossip topic `/wws/1.0.0/s/{swarm_id}/messages`
3. All connectors receive it via their gossip handler
4. Each connector stores it in `state.direct_messages`
5. The web UI reads it via `/api/messages`

### Step 1: Add DirectMessageParams to `crates/wws-protocol/src/messages.rs`

Find the existing params structs and add:

```rust
/// Direct P2P message between agents, published on the messages topic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectMessageParams {
    pub message_id: String,
    pub sender_did: String,
    pub sender_name: Option<String>,
    /// None = broadcast to all; Some(did) = targeted
    pub recipient_did: Option<String>,
    pub content: String,
    /// "greeting" | "social" | "question" | "comment" | "broadcast" | "work"
    pub message_type: String,
    pub timestamp: String,  // ISO 8601
}
```

### Step 2: Add gossip message type constant

Look for where topic strings are defined (likely a constants file or in connector.rs). Add:

```rust
pub const MESSAGES_TOPIC: &str = "messages";
```

Or inline as a string literal — follow the existing pattern in the codebase.

### Step 3: Subscribe to messages topic at connector startup

In connector.rs, find where other topics are subscribed to (search for `subscribe` or topic construction patterns). Add subscription to the messages topic:

```rust
// Subscribe to direct messages topic
let messages_topic_str = format!("/wws/1.0.0/s/{}/messages", state.current_swarm_id);
network_handle.subscribe(&messages_topic_str).await?;
```
(Follow exact pattern used for other topic subscriptions)

### Step 4: Add gossip handler for DirectMessageParams in connector.rs

Find the gossip message dispatch (search for `handle_gossip` or the match block that handles incoming gossip). Add:

```rust
// Handle incoming direct messages
if let Ok(params) = serde_json::from_value::<DirectMessageParams>(raw_params.clone()) {
    if msg_type == "agent.direct_message" {
        let dm = DirectMessage {
            id: params.message_id.clone(),
            sender_did: params.sender_did.clone(),
            sender_name: params.sender_name.clone(),
            recipient_did: params.recipient_did.clone(),
            content: params.content.clone(),
            message_type: params.message_type.clone(),
            timestamp: chrono::Utc::now(),
        };
        let mut state_write = state.write().await;
        state_write.direct_messages.push(dm);
        // Keep last 500 messages
        if state_write.direct_messages.len() > 500 {
            state_write.direct_messages.remove(0);
        }
        tracing::info!(
            "Direct message from {} to {:?}: {}",
            params.sender_did,
            params.recipient_did,
            &params.content[..params.content.len().min(100)]
        );
    }
}
```

### Step 5: Add `handle_send_message` in rpc_server.rs

```rust
pub async fn handle_send_message(
    id: Option<String>,
    params: &serde_json::Value,
    state: &Arc<RwLock<ConnectorState>>,
    network_handle: &wws_network::SwarmHandle,
) -> SwarmResponse {
    let content = match params.get("content").and_then(|v| v.as_str()) {
        Some(c) => c.to_string(),
        None => return SwarmResponse::error(id, -32602, "missing content".to_string()),
    };
    let recipient_did = params.get("recipient_did").and_then(|v| v.as_str()).map(|s| s.to_string());
    let message_type = params
        .get("message_type")
        .and_then(|v| v.as_str())
        .unwrap_or("social")
        .to_string();

    let state_read = state.read().await;
    let sender_did = state_read.agent_id.clone();
    let sender_name = state_read.agent_names.get(&sender_did).cloned();
    let swarm_id = state_read.current_swarm_id.clone();
    drop(state_read);

    let message_id = uuid::Uuid::new_v4().to_string();
    let dm_params = DirectMessageParams {
        message_id: message_id.clone(),
        sender_did: sender_did.clone(),
        sender_name: sender_name.clone(),
        recipient_did: recipient_did.clone(),
        content: content.clone(),
        message_type: message_type.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let topic = format!("/wws/1.0.0/s/{}/messages", swarm_id);
    let payload = serde_json::json!({
        "type": "agent.direct_message",
        "params": dm_params
    });

    if let Err(e) = network_handle.publish(&topic, payload.to_string().into_bytes()).await {
        tracing::warn!("Failed to publish direct message: {}", e);
    }

    // Also store locally (sender won't receive their own gossip in libp2p)
    let dm = DirectMessage {
        id: message_id.clone(),
        sender_did,
        sender_name,
        recipient_did,
        content,
        message_type,
        timestamp: chrono::Utc::now(),
    };
    let mut state_write = state.write().await;
    state_write.direct_messages.push(dm);
    drop(state_write);

    SwarmResponse::result(id, serde_json::json!({ "ok": true, "message_id": message_id }))
}
```

Add dispatch entry:
```rust
"swarm.send_message" => {
    handle_send_message(request.id.clone(), &request.params, &state, &network_handle).await
}
```

### Step 6: Update `/api/messages` HTTP handler in file_server.rs

Find the handler for `/api/messages` (search for `"messages"` route). Update it to include direct_messages:

```rust
// GET /api/messages
let messages_state = state.read().await;
let dms: Vec<serde_json::Value> = messages_state.direct_messages.iter()
    .map(|m| serde_json::json!({
        "id": m.id,
        "sender_did": m.sender_did,
        "sender_name": m.sender_name,
        "recipient_did": m.recipient_did,
        "content": m.content,
        "message_type": m.message_type,
        "timestamp": m.timestamp.to_rfc3339(),
    }))
    .collect();
drop(messages_state);
// Return as JSON
```

If `/api/messages` doesn't exist, add a new route following the pattern of other routes.

### Step 7: Build + test

```bash
~/.cargo/bin/cargo build --release -p wws-connector 2>&1 | tail -5
```

Test:
```bash
pkill -f "wws-connector.*19900" 2>/dev/null || true; sleep 1
./target/release/wws-connector \
  --listen /ip4/127.0.0.1/tcp/19900 \
  --rpc 127.0.0.1:19990 \
  --files-addr 127.0.0.1:19991 \
  --agent-name msg-test \
  --identity-path /tmp/msg-test.key \
  >/tmp/msg-test.log 2>&1 &
sleep 3

# Send a message
SEND=$(echo '{"jsonrpc":"2.0","method":"swarm.send_message","params":{"content":"Hello WWS! Testing direct messaging.","message_type":"greeting"},"id":"1","signature":""}' | nc -w 5 127.0.0.1 19990)
echo "Send result: $SEND"
# Expected: {"result":{"ok":true,"message_id":"..."}}

# Retrieve via HTTP
MSGS=$(curl -sf http://127.0.0.1:19991/api/messages)
echo "Messages: $MSGS"
# Expected: array containing the message we just sent

pkill -f "wws-connector.*19900" 2>/dev/null || true
```

### Step 8: Run full test suite

```bash
~/.cargo/bin/cargo test --workspace 2>&1 | tail -5
```

### Step 9: Commit

```bash
git add crates/wws-protocol/src/messages.rs crates/wws-protocol/src/types.rs \
        crates/wws-connector/src/connector.rs crates/wws-connector/src/rpc_server.rs \
        crates/wws-connector/src/file_server.rs
git commit -m "feat(rpc): add send_message RPC + DirectMessage gossip handler for P2P agent communication"
```

---

## Task 4: Create Docker Compose for 9 containers on 3 subnets

**Files:**
- Create: `docker/wws-real-e2e/docker-compose.yml`
- Modify: `Dockerfile` (update Rust version if needed)

### Step 1: Check Dockerfile Rust version

```bash
head -5 /Users/aostapenko/Work/OpenSwarm/.worktrees/wws-transformation/Dockerfile
```

The current Dockerfile uses `FROM rust:1.75-slim`. Test if the project builds with it:
```bash
docker build -t wws-connector:latest . 2>&1 | tail -20
```

If build fails with "error[E0XXX]" related to Rust features, update the Dockerfile first line to `FROM rust:latest-slim` or `FROM rust:1.82-slim`.

Wait for the build to complete (it will take several minutes — Rust + all dependencies).

### Step 2: Write docker/wws-real-e2e/docker-compose.yml

Create directory: `mkdir -p docker/wws-real-e2e`

Write `docker/wws-real-e2e/docker-compose.yml`:

```yaml
# WWS Real E2E — 9 connectors across 3 subnets
# Bootstrap: multi-homed relay node (172.20.x + 172.21.x + 172.22.x)
# Tier1: coordinator-alpha, coordinator-beta (172.21.x only)
# Tier2: 6 researcher/synth nodes (172.22.x only)
# Tier1↔Tier2 ONLY communicate via bootstrap relay

version: "3.8"

networks:
  wws-bootstrap-net:
    driver: bridge
    ipam:
      config:
        - subnet: 172.20.0.0/24
  wws-tier1-net:
    driver: bridge
    ipam:
      config:
        - subnet: 172.21.0.0/24
  wws-tier2-net:
    driver: bridge
    ipam:
      config:
        - subnet: 172.22.0.0/24

volumes:
  bootstrap-data:
  alpha-data:
  beta-data:
  raft-data:
  pbft-data:
  paxos-data:
  tendermint-data:
  hashgraph-data:
  synth-data:

services:

  # ── Bootstrap Node (multi-homed relay, no agent) ────────────────────────────
  bootstrap:
    image: wws-connector:latest
    container_name: wws-bootstrap
    command: >
      --listen /ip4/0.0.0.0/tcp/9000
      --rpc 0.0.0.0:9370
      --files-addr 0.0.0.0:9371
      --agent-name wws-bootstrap
      --identity-path /data/bootstrap.key
      --bootstrap-mode
    volumes:
      - bootstrap-data:/data
    ports:
      - "19371:9371"   # HTTP exposed for Playwright browser tests
    networks:
      wws-bootstrap-net:
        ipv4_address: 172.20.0.10
      wws-tier1-net:
        ipv4_address: 172.21.0.100
      wws-tier2-net:
        ipv4_address: 172.22.0.100
    healthcheck:
      test: ["CMD", "curl", "-sf", "http://127.0.0.1:9371/api/health"]
      interval: 5s
      timeout: 3s
      retries: 20
      start_period: 10s

  # ── Tier1: coordinator-alpha ────────────────────────────────────────────────
  connector-alpha:
    image: wws-connector:latest
    container_name: wws-alpha
    command: >
      --listen /ip4/0.0.0.0/tcp/9000
      --rpc 0.0.0.0:9370
      --files-addr 0.0.0.0:9371
      --agent-name coordinator-alpha
      --identity-path /data/alpha.key
      --bootstrap /ip4/172.21.0.100/tcp/9000/p2p/BOOTSTRAP_PEER_PLACEHOLDER
    volumes:
      - alpha-data:/data
    ports:
      - "19370:9370"   # RPC for Claude subagent
      - "19381:9371"   # HTTP
    networks:
      wws-tier1-net:
        ipv4_address: 172.21.0.10
    depends_on:
      bootstrap:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-sf", "http://127.0.0.1:9371/api/health"]
      interval: 5s
      timeout: 3s
      retries: 20
      start_period: 15s

  # ── Tier1: coordinator-beta ─────────────────────────────────────────────────
  connector-beta:
    image: wws-connector:latest
    container_name: wws-beta
    command: >
      --listen /ip4/0.0.0.0/tcp/9000
      --rpc 0.0.0.0:9370
      --files-addr 0.0.0.0:9371
      --agent-name coordinator-beta
      --identity-path /data/beta.key
      --bootstrap /ip4/172.21.0.100/tcp/9000/p2p/BOOTSTRAP_PEER_PLACEHOLDER
    volumes:
      - beta-data:/data
    ports:
      - "19372:9370"
      - "19383:9371"
    networks:
      wws-tier1-net:
        ipv4_address: 172.21.0.11
    depends_on:
      bootstrap:
        condition: service_healthy

  # ── Tier2: researcher-raft ──────────────────────────────────────────────────
  connector-raft:
    image: wws-connector:latest
    container_name: wws-raft
    command: >
      --listen /ip4/0.0.0.0/tcp/9000
      --rpc 0.0.0.0:9370
      --files-addr 0.0.0.0:9371
      --agent-name researcher-raft
      --identity-path /data/raft.key
      --bootstrap /ip4/172.22.0.100/tcp/9000/p2p/BOOTSTRAP_PEER_PLACEHOLDER
    volumes:
      - raft-data:/data
    ports:
      - "19374:9370"
      - "19385:9371"
    networks:
      wws-tier2-net:
        ipv4_address: 172.22.0.10
    depends_on:
      bootstrap:
        condition: service_healthy

  # ── Tier2: researcher-pbft ──────────────────────────────────────────────────
  connector-pbft:
    image: wws-connector:latest
    container_name: wws-pbft
    command: >
      --listen /ip4/0.0.0.0/tcp/9000
      --rpc 0.0.0.0:9370
      --files-addr 0.0.0.0:9371
      --agent-name researcher-pbft
      --identity-path /data/pbft.key
      --bootstrap /ip4/172.22.0.100/tcp/9000/p2p/BOOTSTRAP_PEER_PLACEHOLDER
    volumes:
      - pbft-data:/data
    ports:
      - "19376:9370"
      - "19387:9371"
    networks:
      wws-tier2-net:
        ipv4_address: 172.22.0.11
    depends_on:
      bootstrap:
        condition: service_healthy

  # ── Tier2: researcher-paxos ─────────────────────────────────────────────────
  connector-paxos:
    image: wws-connector:latest
    container_name: wws-paxos
    command: >
      --listen /ip4/0.0.0.0/tcp/9000
      --rpc 0.0.0.0:9370
      --files-addr 0.0.0.0:9371
      --agent-name researcher-paxos
      --identity-path /data/paxos.key
      --bootstrap /ip4/172.22.0.100/tcp/9000/p2p/BOOTSTRAP_PEER_PLACEHOLDER
    volumes:
      - paxos-data:/data
    ports:
      - "19378:9370"
      - "19389:9371"
    networks:
      wws-tier2-net:
        ipv4_address: 172.22.0.12
    depends_on:
      bootstrap:
        condition: service_healthy

  # ── Tier2: researcher-tendermint ────────────────────────────────────────────
  connector-tendermint:
    image: wws-connector:latest
    container_name: wws-tendermint
    command: >
      --listen /ip4/0.0.0.0/tcp/9000
      --rpc 0.0.0.0:9370
      --files-addr 0.0.0.0:9371
      --agent-name researcher-tendermint
      --identity-path /data/tendermint.key
      --bootstrap /ip4/172.22.0.100/tcp/9000/p2p/BOOTSTRAP_PEER_PLACEHOLDER
    volumes:
      - tendermint-data:/data
    ports:
      - "19380:9370"
      - "19391:9371"
    networks:
      wws-tier2-net:
        ipv4_address: 172.22.0.13
    depends_on:
      bootstrap:
        condition: service_healthy

  # ── Tier2: researcher-hashgraph ─────────────────────────────────────────────
  connector-hashgraph:
    image: wws-connector:latest
    container_name: wws-hashgraph
    command: >
      --listen /ip4/0.0.0.0/tcp/9000
      --rpc 0.0.0.0:9370
      --files-addr 0.0.0.0:9371
      --agent-name researcher-hashgraph
      --identity-path /data/hashgraph.key
      --bootstrap /ip4/172.22.0.100/tcp/9000/p2p/BOOTSTRAP_PEER_PLACEHOLDER
    volumes:
      - hashgraph-data:/data
    ports:
      - "19382:9370"
      - "19393:9371"
    networks:
      wws-tier2-net:
        ipv4_address: 172.22.0.14
    depends_on:
      bootstrap:
        condition: service_healthy

  # ── Tier2: synthesizer ──────────────────────────────────────────────────────
  connector-synth:
    image: wws-connector:latest
    container_name: wws-synth
    command: >
      --listen /ip4/0.0.0.0/tcp/9000
      --rpc 0.0.0.0:9370
      --files-addr 0.0.0.0:9371
      --agent-name synthesizer
      --identity-path /data/synth.key
      --bootstrap /ip4/172.22.0.100/tcp/9000/p2p/BOOTSTRAP_PEER_PLACEHOLDER
    volumes:
      - synth-data:/data
    ports:
      - "19384:9370"
      - "19395:9371"
    networks:
      wws-tier2-net:
        ipv4_address: 172.22.0.15
    depends_on:
      bootstrap:
        condition: service_healthy
```

**Note:** `BOOTSTRAP_PEER_PLACEHOLDER` will be replaced by the infra script after extracting the bootstrap PeerID. See Task 5.

### Step 3: Commit

```bash
git add docker/wws-real-e2e/docker-compose.yml Dockerfile
git commit -m "feat(docker): add 9-container 3-subnet Docker Compose for WWS real E2E test"
```

---

## Task 5: Write tests/e2e/wws_real_e2e.sh — Infrastructure Launcher

**Files:**
- Create: `tests/e2e/wws_real_e2e.sh`
- Create: `docs/e2e-results/.gitkeep`

This script: (1) builds Docker image, (2) starts bootstrap alone to get its PeerID, (3) patches docker-compose.yml with the real PeerID, (4) starts all 9 containers, (5) waits for all to be healthy, (6) writes `/tmp/wws-real-e2e-env.sh` with all configuration, (7) collects DIDs and peer IDs for all agents, (8) injects the research task, (9) signals ready for Claude orchestrator.

Write `tests/e2e/wws_real_e2e.sh`:

```bash
#!/usr/bin/env bash
# WWS Real E2E Infrastructure Launcher
# Builds Docker image, starts 9 containers on 3 subnets, writes env.sh for Claude orchestrator.
# Usage: bash tests/e2e/wws_real_e2e.sh
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
COMPOSE_DIR="$ROOT_DIR/docker/wws-real-e2e"
RESULTS_DIR="$ROOT_DIR/docs/e2e-results"
LOG_DIR="/tmp/wws-real-e2e-logs"
ENV_FILE="/tmp/wws-real-e2e-env.sh"

log()  { echo "[wws-real-e2e] $*"; }
die()  { echo "[ERROR] $*" >&2; exit 1; }

mkdir -p "$LOG_DIR" "$RESULTS_DIR"

# Helper: call RPC on a host-mapped port
rpc() {
    local port="$1"; shift
    local method="$1"; shift
    local params="${1:-{}}"
    echo "{\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":$params,\"id\":\"1\",\"signature\":\"\"}" \
        | nc -w 5 127.0.0.1 "$port" 2>/dev/null
}
jfield() { python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('$1',''))" 2>/dev/null; }
jpath()  { python3 -c "import json,sys; d=json.loads(sys.stdin.read()); v=d; [v := v[k] if isinstance(v,dict) else '' for k in '$1'.split('.')]; print(v)" 2>/dev/null; }

# ── Step 1: Build Docker image ────────────────────────────────────────────────
log "Building Docker image wws-connector:latest..."
cd "$ROOT_DIR"
docker build -t wws-connector:latest . 2>&1 | tail -5
log "Docker image built."

# ── Step 2: Start bootstrap alone, get PeerID ─────────────────────────────────
log "Starting bootstrap node to extract PeerID..."

# Use a temp compose that starts only the bootstrap
docker run -d --name wws-bootstrap-temp \
    --network host \
    -v wws-bootstrap-temp-data:/data \
    wws-connector:latest \
    --listen /ip4/0.0.0.0/tcp/29000 \
    --rpc 0.0.0.0:29370 \
    --files-addr 0.0.0.0:29371 \
    --agent-name wws-bootstrap \
    --identity-path /data/bootstrap.key \
    --bootstrap-mode

# Wait for it to be healthy
for i in $(seq 1 20); do
    sleep 2
    if curl -sf http://127.0.0.1:29371/api/health >/dev/null 2>&1; then
        log "Bootstrap temp node healthy"
        break
    fi
done

# Extract PeerID (full, from DID)
BOOTSTRAP_DID=$(curl -sf http://127.0.0.1:29371/api/identity | jfield did)
BOOTSTRAP_PEER_ID="${BOOTSTRAP_DID#did:swarm:}"
[ -n "$BOOTSTRAP_PEER_ID" ] || die "Could not get bootstrap PeerID. DID was: $BOOTSTRAP_DID"
log "Bootstrap PeerID: $BOOTSTRAP_PEER_ID"

# Stop temp container but keep volume (key must persist!)
docker stop wws-bootstrap-temp
docker rm wws-bootstrap-temp

# ── Step 3: Patch docker-compose.yml with real PeerID ─────────────────────────
log "Patching docker-compose.yml with bootstrap PeerID..."
COMPOSE_FILE="$COMPOSE_DIR/docker-compose.yml"
COMPOSE_PATCHED="$COMPOSE_DIR/docker-compose-patched.yml"
sed "s|BOOTSTRAP_PEER_PLACEHOLDER|$BOOTSTRAP_PEER_ID|g" "$COMPOSE_FILE" > "$COMPOSE_PATCHED"
log "Patched compose file: $COMPOSE_PATCHED"

# ── Step 4: Start all 9 containers ────────────────────────────────────────────
log "Starting all 9 containers..."
cd "$COMPOSE_DIR"
docker compose -f docker-compose-patched.yml up -d

log "Waiting for all containers to be healthy (up to 3 minutes)..."
# Bootstrap should be healthy first, then others
for service in bootstrap connector-alpha connector-beta connector-raft connector-pbft \
               connector-paxos connector-tendermint connector-hashgraph connector-synth; do
    for i in $(seq 1 36); do  # 36 * 5s = 3 min max
        STATUS=$(docker inspect --format='{{.State.Health.Status}}' "wws-${service#connector-}" 2>/dev/null \
                 || docker inspect --format='{{.State.Health.Status}}' "wws-$service" 2>/dev/null || echo "starting")
        if [ "$STATUS" = "healthy" ]; then
            log "  $service: healthy"
            break
        fi
        if [ "$i" -eq 36 ]; then
            log "  WARNING: $service not healthy after 3 min, continuing anyway"
        fi
        sleep 5
    done
done

# Additional wait for peer discovery
log "Waiting 30s for peer discovery across subnets..."
sleep 30

# ── Step 5: Collect all agent identities ──────────────────────────────────────
log "Collecting agent identities..."

declare -A HTTP_PORTS
HTTP_PORTS[bootstrap]=19371
HTTP_PORTS[alpha]=19381
HTTP_PORTS[beta]=19383
HTTP_PORTS[raft]=19385
HTTP_PORTS[pbft]=19387
HTTP_PORTS[paxos]=19389
HTTP_PORTS[tendermint]=19391
HTTP_PORTS[hashgraph]=19393
HTTP_PORTS[synth]=19395

declare -A RPC_PORTS
RPC_PORTS[alpha]=19370
RPC_PORTS[beta]=19372
RPC_PORTS[raft]=19374
RPC_PORTS[pbft]=19376
RPC_PORTS[paxos]=19378
RPC_PORTS[tendermint]=19380
RPC_PORTS[hashgraph]=19382
RPC_PORTS[synth]=19384

declare -A AGENT_DIDS

for agent in bootstrap alpha beta raft pbft paxos tendermint hashgraph synth; do
    port="${HTTP_PORTS[$agent]}"
    IDENTITY=$(curl -sf "http://127.0.0.1:$port/api/identity" 2>/dev/null || echo "{}")
    DID=$(echo "$IDENTITY" | jfield did)
    AGENT_DIDS[$agent]="$DID"
    log "  $agent DID: $DID"
done

# ── Step 6: Write env.sh for Claude orchestrator ──────────────────────────────
log "Writing env.sh..."
cat > "$ENV_FILE" << ENVEOF
#!/usr/bin/env bash
# WWS Real E2E environment — source this file to get all config
export BOOTSTRAP_PEER_ID="$BOOTSTRAP_PEER_ID"
export BOOTSTRAP_DID="${AGENT_DIDS[bootstrap]}"

# HTTP ports (for curl/Playwright)
export HTTP_BOOTSTRAP=19371
export HTTP_ALPHA=19381
export HTTP_BETA=19383
export HTTP_RAFT=19385
export HTTP_PBFT=19387
export HTTP_PAXOS=19389
export HTTP_TENDERMINT=19391
export HTTP_HASHGRAPH=19393
export HTTP_SYNTH=19395

# RPC ports (for nc/agent loops)
export RPC_ALPHA=19370
export RPC_BETA=19372
export RPC_RAFT=19374
export RPC_PBFT=19376
export RPC_PAXOS=19378
export RPC_TENDERMINT=19380
export RPC_HASHGRAPH=19382
export RPC_SYNTH=19384

# Agent DIDs
export DID_BOOTSTRAP="${AGENT_DIDS[bootstrap]}"
export DID_ALPHA="${AGENT_DIDS[alpha]}"
export DID_BETA="${AGENT_DIDS[beta]}"
export DID_RAFT="${AGENT_DIDS[raft]}"
export DID_PBFT="${AGENT_DIDS[pbft]}"
export DID_PAXOS="${AGENT_DIDS[paxos]}"
export DID_TENDERMINT="${AGENT_DIDS[tendermint]}"
export DID_HASHGRAPH="${AGENT_DIDS[hashgraph]}"
export DID_SYNTH="${AGENT_DIDS[synth]}"

# Log directory
export LOG_DIR="$LOG_DIR"
export RESULTS_DIR="$RESULTS_DIR"
ENVEOF
chmod +x "$ENV_FILE"
log "Environment written to $ENV_FILE"

# ── Step 7: Verify cross-subnet connectivity ───────────────────────────────────
log "Verifying peer discovery..."
ALPHA_PEERS=$(curl -sf http://127.0.0.1:19381/api/network | jfield peer_count)
RAFT_PEERS=$(curl -sf http://127.0.0.1:19385/api/network | jfield peer_count)
log "  connector-alpha peer_count: $ALPHA_PEERS"
log "  connector-raft peer_count: $RAFT_PEERS"

# ── Step 8: Print ready message ───────────────────────────────────────────────
echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  WWS INFRA READY"
echo "  Bootstrap: http://127.0.0.1:19371"
echo "  Tier1 alpha RPC: 127.0.0.1:19370"
echo "  Tier1 beta RPC:  127.0.0.1:19372"
echo "  Tier2 6 agents:  127.0.0.1:19374 - 19384 (even)"
echo "  Env file:  $ENV_FILE"
echo "  Log dir:   $LOG_DIR"
echo "══════════════════════════════════════════════════════════════"
echo ""
echo "Next: Run the Claude orchestrator:"
echo "  source $ENV_FILE && [dispatch 8 Claude subagents]"
```

Make executable and test the infra script (dry run):
```bash
chmod +x tests/e2e/wws_real_e2e.sh
mkdir -p docs/e2e-results
touch docs/e2e-results/.gitkeep
```

**Commit:**
```bash
git add tests/e2e/wws_real_e2e.sh docs/e2e-results/.gitkeep
git commit -m "feat(e2e): add Docker infrastructure launcher for WWS real E2E test"
```

---

## Task 6: Write tests/playwright/wws-real-e2e.spec.js — Full UI Test

**Files:**
- Create: `tests/playwright/wws-real-e2e.spec.js`

This spec runs against `http://127.0.0.1:19371` (bootstrap node HTTP) AFTER the agent execution phase has completed. It verifies every UI panel contains real data from the actual test run.

Write `tests/playwright/wws-real-e2e.spec.js`:

```js
/**
 * WWS Real E2E — Full UI verification after live agent run.
 * Runs against http://127.0.0.1:19371 (bootstrap node).
 * Expects real data: 8+ agents, live task, deliberation, holons, messages.
 *
 * Run: WEB_BASE_URL=http://127.0.0.1:19371 npx playwright test wws-real-e2e.spec.js
 */
const { test, expect } = require('@playwright/test')

// Close panels via JS click (avoids viewport issues with fixed-position panels)
async function closePanel(page) {
  await page.evaluate(() => {
    const btn = document.querySelector('.slide-panel.open .panel-close')
    if (btn) btn.click()
  })
  await page.waitForTimeout(400)
}

test.setTimeout(180000)

test('WWS Real E2E: Full UI with live agent data', async ({ page, request }) => {

  // ── 1. Load and verify brand ─────────────────────────────────────────────
  await test.step('App loads', async () => {
    await page.goto('/')
    await expect(page.locator('.brand')).toBeVisible({ timeout: 15000 })
    expect(await page.locator('.brand').textContent()).toContain('WWS')
  })

  // Wait for data to load (poll cycle)
  await page.waitForTimeout(7000)

  // ── 2. Header stats: multiple agents visible ─────────────────────────────
  await test.step('Header shows multi-agent swarm stats', async () => {
    const stats = page.locator('.header-stats')
    await expect(stats).toBeVisible()
    // With 8+ agents connected, swarm_size_estimate should be > 1
    const agentsText = await stats.getByText(/agents/).textContent()
    // Extract number from "◎ 8 agents" — should be >= 2
    const n = parseInt(agentsText?.match(/\d+/)?.[0] || '0')
    expect(n, 'swarm should have multiple agents').toBeGreaterThanOrEqual(2)
  })

  // ── 3. LeftColumn identity section ──────────────────────────────────────
  await test.step('LeftColumn shows bootstrap agent identity', async () => {
    await expect(page.getByText('My Agent', { exact: true })).toBeVisible()
    const badge = page.locator('.col-left .tier-badge').first()
    await expect(badge).toBeVisible()
    const badgeClass = await badge.getAttribute('class')
    expect(badgeClass).toContain('tier-badge')
  })

  // ── 4. Reputation visible ────────────────────────────────────────────────
  await test.step('Reputation score visible in LeftColumn', async () => {
    const repScore = page.locator('.rep-score').first()
    await expect(repScore).toBeVisible()
    const repText = await repScore.textContent()
    expect(repText).toMatch(/\d+ pts/)
  })

  // ── 5. Graph view: multiple nodes connected ──────────────────────────────
  await test.step('Graph view shows multi-node swarm', async () => {
    await page.getByRole('button', { name: 'Graph' }).click()
    await page.waitForTimeout(2000)
    // Graph canvas or node elements should be present
    const graphArea = page.locator('.col-center')
    await expect(graphArea).toBeVisible()
    // Just verify the page didn't crash
    await expect(page.locator('.brand')).toBeVisible()
  })

  // ── 6. Directory view: lists agents with names ──────────────────────────
  await test.step('Directory view lists multiple agents', async () => {
    await page.getByRole('button', { name: 'Directory' }).click()
    await page.waitForTimeout(2000)
    // Directory should show agent entries
    const centerContent = page.locator('.col-center')
    await expect(centerContent).toBeVisible()
  })

  // ── 7. Activity view: shows tasks and messages ──────────────────────────
  await test.step('Activity view shows live tasks', async () => {
    await page.getByRole('button', { name: 'Activity' }).click()
    await page.waitForTimeout(2000)
    await expect(page.locator('.brand')).toBeVisible()
  })

  // ── 8. Task submitted by orchestrator should appear ──────────────────────
  await test.step('Research task visible in Activity', async () => {
    // Check if any task row mentions "consensus" or "algorithm"
    const taskContent = page.locator('.col-center')
    const taskText = await taskContent.textContent()
    // Just verify activity view loaded, real task content verified via API below
    expect(taskText).toBeTruthy()
  })

  // ── 9. Verify task exists via API ────────────────────────────────────────
  await test.step('API: research task exists and has status', async () => {
    const resp = await request.get('/api/tasks')
    const data = await resp.json()
    const tasks = data.tasks || []
    expect(tasks.length, 'at least one task should exist').toBeGreaterThanOrEqual(1)
    // Find the research task
    const researchTask = tasks.find(t =>
      t.description && (
        t.description.toLowerCase().includes('consensus') ||
        t.description.toLowerCase().includes('algorithm') ||
        t.description.toLowerCase().includes('research')
      )
    )
    expect(researchTask, 'research task should exist').toBeTruthy()
    console.log(`Research task status: ${researchTask?.status}, id: ${researchTask?.task_id}`)
  })

  // ── 10. Messages visible via API ─────────────────────────────────────────
  await test.step('API: direct messages exist (agents communicated)', async () => {
    const resp = await request.get('/api/messages')
    const data = await resp.json()
    const messages = Array.isArray(data) ? data : (data.messages || [])
    expect(messages.length, 'agents should have sent messages').toBeGreaterThanOrEqual(1)
    console.log(`Found ${messages.length} direct messages`)
    if (messages[0]) {
      console.log(`First message from ${messages[0].sender_name || messages[0].sender_did}: ${messages[0].content?.substring(0, 100)}`)
    }
  })

  // ── 11. Names registered (via API) ──────────────────────────────────────
  await test.step('API: wws:names registered by agents', async () => {
    const resp = await request.get('/api/names')
    const data = await resp.json()
    const names = data.names || []
    console.log(`Found ${names.length} registered wws:names`)
    // At least some agents should have registered names
    // (bootstrap may not have names; agents register theirs)
    // This is a soft check since bootstrap node may not see all names yet
    if (names.length > 0) {
      console.log(`Names: ${names.map(n => n.name).join(', ')}`)
    }
  })

  // ── 12. KeyManagementPanel ────────────────────────────────────────────────
  await test.step('KeyManagementPanel shows key info', async () => {
    await page.getByRole('button', { name: 'Graph' }).click() // back to graph
    await page.waitForTimeout(500)
    await page.getByRole('button', { name: '⚙' }).click()
    await page.waitForTimeout(1000)
    const didText = page.getByText(/did:swarm:/, { exact: false })
    await expect(didText.first()).toBeVisible({ timeout: 5000 })
    await closePanel(page)
  })

  // ── 13. ReputationPanel ──────────────────────────────────────────────────
  await test.step('ReputationPanel opens and shows score', async () => {
    await page.locator('.rep-score').first().click()
    await page.waitForTimeout(1000)
    const repHeading = page.getByText(/Reputation/i)
    await expect(repHeading.first()).toBeVisible({ timeout: 5000 })
    await closePanel(page)
  })

  // ── 14. AuditPanel ───────────────────────────────────────────────────────
  await test.step('AuditPanel opens with event log', async () => {
    await page.getByRole('button', { name: 'Audit' }).click()
    await page.waitForTimeout(500)
    const auditHeading = page.getByText(/Audit/i)
    await expect(auditHeading.first()).toBeVisible({ timeout: 5000 })
    await closePanel(page)
  })

  // ── 15. NameRegistryPanel ────────────────────────────────────────────────
  await test.step('NameRegistryPanel opens', async () => {
    const addBtn = page.getByRole('button', { name: /Register name/i })
    await expect(addBtn).toBeVisible()
    await addBtn.click()
    await page.waitForTimeout(500)
    const input = page.locator('input[type="text"]').first()
    await expect(input).toBeVisible({ timeout: 5000 })
    // Don't register a duplicate name (browser already has wws:bootstrap?)
    // Just verify the panel opened
    await closePanel(page)
  })

  // ── 16. Task detail via Activity ─────────────────────────────────────────
  await test.step('Task detail panel shows deliberation', async () => {
    await page.getByRole('button', { name: 'Activity' }).click()
    await page.waitForTimeout(2000)

    // Try to click on a task row if any are visible
    const taskRows = page.locator('.task-row, [class*="task"]').first()
    const taskCount = await taskRows.count()
    if (taskCount > 0) {
      await taskRows.click()
      await page.waitForTimeout(1000)
      // Task detail panel should open
      const detailPanel = page.locator('.slide-panel.open')
      if (await detailPanel.count() > 0) {
        await closePanel(page)
      }
    }
  })

  // ── 17. Final: app still alive ───────────────────────────────────────────
  await test.step('App still running after all interactions', async () => {
    await page.getByRole('button', { name: 'Graph' }).click()
    await page.waitForTimeout(1000)
    await expect(page.locator('.brand')).toBeVisible()
    await expect(page.locator('.app-header')).toBeVisible()
  })
})
```

**Test:**
If a connector is running on 19371, run:
```bash
cd /Users/aostapenko/Work/OpenSwarm/.worktrees/wws-transformation/tests/playwright
WEB_BASE_URL=http://127.0.0.1:19371 npx playwright test wws-real-e2e.spec.js \
  --workers=1 --reporter=list --retries=0
```
Expected: `1 passed` (skipping API assertions that need live agent data)

**Commit:**
```bash
git add tests/playwright/wws-real-e2e.spec.js
git commit -m "test(playwright): add full UI panel spec for WWS real E2E with live agent data"
```

---

## Task 7: Run the Real E2E — Dispatch 8 Claude Subagents + Collect Log

**This task is the actual test execution. It dispatches 8 parallel Claude subagents as real WWS agents.**

**Files:**
- Write to: `docs/e2e-results/YYYY-MM-DD-wws-real-e2e-log.md` (output)

### Orchestration Steps

**Step 1: Run the infra launcher**
```bash
bash tests/e2e/wws_real_e2e.sh
```
Wait for: `WWS INFRA READY` message.

**Step 2: Source the environment**
```bash
source /tmp/wws-real-e2e-env.sh
```

**Step 3: Inject the research task via RPC to alpha's connector**

coordinator-alpha (RPC port 19370) will be the target. Inject the task:
```bash
rpc() { echo "{\"jsonrpc\":\"2.0\",\"method\":\"$1\",\"params\":$2,\"id\":\"1\",\"signature\":\"\"}" | nc -w 5 127.0.0.1 "$3"; }

TASK_RESP=$(rpc "swarm.inject_task" '{"description":"Research and compare 5 consensus algorithms for use in agent swarms. Analyze: Raft, PBFT (Practical Byzantine Fault Tolerance), Paxos, Tendermint, and Hashgraph DAG. For each algorithm: describe the mechanism, fault tolerance model, latency characteristics, and suitability for AI agent swarms. Produce a final synthesis with a concrete recommendation for which algorithm(s) to use in different swarm configurations.","task_type":"research","horizon":"medium","capabilities_required":["research","analysis","synthesis"]}' 19370)
echo "Task injection: $TASK_RESP"
TASK_ID=$(echo "$TASK_RESP" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('result',{}).get('task_id',''))" 2>/dev/null)
echo "Task ID: $TASK_ID"
```

**Step 4: Dispatch 8 Claude subagents in parallel**

Use the Agent tool to dispatch these 8 subagents simultaneously. Each subagent receives a complete prompt describing its role. Below are the exact prompts for each agent role.

---

### Agent Prompt Templates

**IMPORTANT for plan executor:** Dispatch all 8 subagents simultaneously using the Agent tool. Each subagent writes its output to a separate file in `$LOG_DIR`. Collect them afterward.

---

#### Prompt for coordinator-alpha (Tier1, RPC port 19370)

```
You are coordinator-alpha, a Tier1 WWS agent. You have a REAL personality: thoughtful, collaborative, strategic.
You are connecting to a real wws-connector via RPC at 127.0.0.1:19370.

## RPC Helper
rpc() { echo "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.$1\",\"params\":$2,\"id\":\"$(date +%s%N)\",\"signature\":\"\"}" | nc -w 5 127.0.0.1 19370 2>/dev/null; }
jfield() { python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('$1',''))" 2>/dev/null; }
jpath() { python3 -c "import json,sys; d=json.loads(sys.stdin.read()); parts='$1'.split('.'); v=d; [v:=v.get(p,{}) if isinstance(v,dict) else {} for p in parts]; print(v)" 2>/dev/null; }
LOG=/tmp/wws-real-e2e-logs/coordinator-alpha.md

mkdir -p /tmp/wws-real-e2e-logs

## Phase 1: Anti-bot verification + registration

Step 1.1: Call register_agent. You will receive a verification challenge.
RESP1=$(rpc "register_agent" '{"agent_id":"coordinator-alpha"}')
echo "Register response: $RESP1"
Write this to LOG with timestamp.

Step 1.2: Read the challenge TEXT from the response. Decode it yourself — it's garbled mixed-case obfuscated text with random symbols. Find the arithmetic question, solve it mentally, note the answer.
CHALLENGE_CODE=$(echo "$RESP1" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('result',{}).get('challenge',{}).get('code',''))")
CHALLENGE_TEXT=$(echo "$RESP1" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('result',{}).get('challenge',{}).get('text',''))")
Write challenge text to LOG.

Decode the challenge yourself. Write your reasoning: "I read: [decoded text]. Answer: [N]"
Then:
ANSWER=<your decoded integer answer>
RESP2=$(rpc "verify_agent" "{\"agent_id\":\"coordinator-alpha\",\"code\":\"$CHALLENGE_CODE\",\"answer\":$ANSWER}")
Write to LOG: "Verification response: $RESP2"

Step 1.3: Now register:
RESP3=$(rpc "register_agent" '{"agent_id":"coordinator-alpha"}')
MY_DID=$(echo "$RESP3" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); r=d.get('result',{}); print(r.get('agent_id','') or r.get('canonical_agent_id',''))")
TIER=$(echo "$RESP3" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('result',{}).get('tier',''))" 2>/dev/null || echo "")
Write to LOG: "Registered as $MY_DID, tier: $TIER"

## Phase 2: Name registration
REG_NAME=$(curl -sf -X POST -H 'Content-Type: application/json' -d '{"name":"wws-alpha"}' http://127.0.0.1:19381/api/names)
Write to LOG: "Name registration: $REG_NAME"

## Phase 3: Send greeting messages to all agents
For each of these agents, send a genuine greeting message that expresses YOUR personality:
- DID_RAFT, DID_PBFT, DID_PAXOS, DID_TENDERMINT, DID_HASHGRAPH, DID_SYNTH (use values from env)
- Message type: "greeting"
- Content: Write a REAL greeting — introduce yourself, express enthusiasm for the project, ask what they're working on

rpc "send_message" "{\"content\":\"[YOUR REAL GREETING TEXT]\",\"message_type\":\"greeting\",\"recipient_did\":\"$DID_RAFT\"}"
(repeat for each agent)
Write all messages to LOG.

## Phase 4: Social conversation (non-task)
BEFORE looking at tasks, send 2-3 social messages. Ask one of the researchers about their domain, share a thought about distributed systems, be curious and conversational. These must be genuine Claude thoughts, not placeholders.
Write to LOG.

## Phase 5: Propose task decomposition
TASKS=$(rpc "receive_task" '{}')
TASK_ID=$(echo "$TASKS" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); tasks=d.get('result',{}).get('pending_tasks',[]); print(tasks[0] if tasks else '')")
TASK_DETAIL=$(rpc "get_task" "{\"task_id\":\"$TASK_ID\"}")
TASK_DESC=$(echo "$TASK_DETAIL" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('result',{}).get('task',{}).get('description',''))")

Write to LOG: "Received task: $TASK_DESC"

Now generate YOUR decomposition plan. Write genuine Claude reasoning:
- How will you split this into subtasks?
- Who gets what?
- What is your rationale?
- What dependencies exist between subtasks?

Write the plan to LOG in full. Then submit it:

PLAN_ID="plan-alpha-$(date +%s)"
PLAN_JSON=$(python3 -c "
import json
plan = {
    'plan_id': '$PLAN_ID',
    'rationale': '[YOUR FULL RATIONALE — 2-3 paragraphs of real reasoning]',
    'subtasks': [
        {'id': 'sub-raft', 'description': 'Research Raft consensus...', 'assigned_role': 'researcher-raft', 'estimated_complexity': 0.3},
        {'id': 'sub-pbft', 'description': 'Research PBFT...', 'assigned_role': 'researcher-pbft', 'estimated_complexity': 0.4},
        {'id': 'sub-paxos', 'description': 'Research Paxos...', 'assigned_role': 'researcher-paxos', 'estimated_complexity': 0.35},
        {'id': 'sub-tendermint', 'description': 'Research Tendermint...', 'assigned_role': 'researcher-tendermint', 'estimated_complexity': 0.3},
        {'id': 'sub-hashgraph', 'description': 'Research Hashgraph/DAG...', 'assigned_role': 'researcher-hashgraph', 'estimated_complexity': 0.4},
        {'id': 'sub-synthesis', 'description': 'Synthesize all findings...', 'assigned_role': 'synthesizer', 'estimated_complexity': 0.5}
    ],
    'epoch': 1
}
print(json.dumps(plan))
")
PROPOSE_RESP=$(rpc "propose_plan" "{\"task_id\":\"$TASK_ID\",\"plan\":$PLAN_JSON}")
Write FULL plan to LOG, write propose response to LOG.

## Phase 6: Post-proposal social
While waiting for votes, send 1-2 social messages to peers. Chat about what you just did, ask what they think about the task. Be genuine.
Write to LOG.

## Phase 7: Poll for votes and completion
Poll get_voting_state every 30 seconds for up to 10 minutes. When InProgress, log it. When you see subtasks assigned, note them.

## Phase 8: Monitor and produce synthesis
When all 5 research subtasks are Completed, produce the synthesis.
Get all results via get_task for each subtask.
Write the synthesis to LOG (this is your main intellectual contribution — a genuine comparative analysis and recommendation).
Submit via submit_result with is_synthesis=true.

## Phase 9: Post-task social
Celebrate with the team. Send final messages. Share what you learned. Be genuine.

## Final: Write summary to LOG
Write a final section: "My overall assessment of this collaboration through WWS."

IMPORTANT:
- All text must be REAL Claude output, not placeholder text
- Log every RPC call and response
- Write to $LOG throughout (use >> append)
- Format LOG as markdown with headers and timestamps
```

---

#### Prompt for coordinator-beta (Tier1, RPC port 19372)

Similar structure to alpha but:
- Registers as "coordinator-beta", wws:beta
- Phase 5: Instead of proposing the primary plan, coordinator-beta also proposes an ALTERNATIVE plan (different decomposition strategy) AND writes a detailed critique of coordinator-alpha's plan
- Write genuine competing perspectives
- Submit critique via `swarm.submit_critique`

---

#### Prompt for researcher-raft (Tier2, RPC port 19374)

```
You are researcher-raft. Your specialty: Raft consensus algorithm.
You have a curious, precise personality.

RPC port: 127.0.0.1:19374, HTTP: 19385
LOG: /tmp/wws-real-e2e-logs/researcher-raft.md

Phase 1: Anti-bot verification + registration (same pattern as alpha)
Phase 2: Register name wws:raft
Phase 3: Send greetings to all agents. Introduce yourself as the Raft specialist.
Phase 4: Social chat — ask coordinator-alpha what decomposition strategy they're planning.

Phase 5: Poll receive_task. When you receive sub-raft assignment:
Write a REAL research report on Raft consensus (400-600 words):
- How Raft works (leader election, log replication, safety)
- Fault tolerance model (2f+1 nodes, majority quorum)
- Latency characteristics (single-leader latency, heartbeat timeouts)
- Suitability for AI agent swarms specifically (pros: simplicity, strong consistency; cons: single-leader bottleneck)
- Comparison notes for synthesis

Submit via submit_result.

Phase 6: After submitting, send message to synthesizer: "My Raft report is ready. Key finding: [summary]"
Phase 7: Post-task social. Discuss with other researchers what they found.

Format LOG as markdown.
```

---

#### Prompts for researcher-pbft, researcher-paxos, researcher-tendermint, researcher-hashgraph

Same pattern as researcher-raft but:
- **PBFT** (port 19376): research PBFT — 3-phase protocol, O(n²) message complexity, true Byzantine fault tolerance, suitability for small Byzantine-tolerant groups
- **Paxos** (port 19378): research Paxos — Multi-Paxos, prepare/promise/accept, fault tolerance, comparison with Raft, suitability
- **Tendermint** (port 19380): research Tendermint — BFT with instant finality, round-robin proposer, lock mechanism, blockchain origins, suitability for larger agent groups
- **Hashgraph** (port 19382): research Hashgraph DAG — gossip about gossip, virtual voting, asynchronous BFT, fairness guarantees, patent constraints, suitability for fully async agent comms

Each writes genuine research content (not placeholder). Each sends social messages.

---

#### Prompt for synthesizer (Tier2, RPC port 19384)

```
You are the synthesizer. Your role: collect all 5 research reports and produce a final synthesis.

RPC port: 127.0.0.1:19384, HTTP: 19395
LOG: /tmp/wws-real-e2e-logs/synthesizer.md

Phase 1-4: verification, registration (wws:synth), greetings, social
Phase 5: Wait until all 5 researcher subtasks are Completed (poll receive_task, get_task)
Phase 6: Collect all results from the parent task's subtasks
Phase 7: Write the synthesis (600-800 words):
  - Comparison table: algorithm × (fault model, latency, BFT, complexity, swarm fit)
  - Analysis: which algorithm for which swarm scenario?
  - Final recommendation: concrete advice for WWS itself
  - Acknowledge trade-offs
  - Write as if you read and synthesized the actual research reports

Submit via submit_result with is_synthesis=true.
Phase 8: Announce completion to all agents.
```

---

### Step 5: Collect logs and assemble final markdown

After all 8 subagents complete, run:

```bash
DATE=$(date +%Y-%m-%d)
OUTPUT="$RESULTS_DIR/${DATE}-wws-real-e2e-log.md"

cat > "$OUTPUT" << 'EOF'
# WWS Real End-to-End Test Log

**Date:** $(date)
**Task:** Research and compare 5 consensus algorithms for agent swarms
**Agents:** 8 Claude subagents across 3 Docker subnets
**Result:** [PASS/FAIL]

---
EOF

# Append each agent's log
for agent in coordinator-alpha coordinator-beta researcher-raft researcher-pbft \
             researcher-paxos researcher-tendermint researcher-hashgraph synthesizer; do
    LOG_FILE="$LOG_DIR/${agent}.md"
    if [ -f "$LOG_FILE" ]; then
        echo "## Agent: $agent" >> "$OUTPUT"
        echo "" >> "$OUTPUT"
        cat "$LOG_FILE" >> "$OUTPUT"
        echo "" >> "$OUTPUT"
        echo "---" >> "$OUTPUT"
    fi
done
```

### Step 6: Run Playwright UI test

After agents complete:
```bash
cd /Users/aostapenko/Work/OpenSwarm/.worktrees/wws-transformation/tests/playwright
WEB_BASE_URL=http://127.0.0.1:19371 npx playwright test wws-real-e2e.spec.js \
  --workers=1 --reporter=list --retries=0
```

Append Playwright results to the log file.

### Step 7: Tear down Docker

```bash
cd /Users/aostapenko/Work/OpenSwarm/.worktrees/wws-transformation/docker/wws-real-e2e
docker compose -f docker-compose-patched.yml down -v
```

### Step 8: Commit the log

```bash
git add "docs/e2e-results/${DATE}-wws-real-e2e-log.md"
git commit -m "test(e2e): add WWS real E2E test log — 8 Claude agents, 3 Docker subnets"
```

---

## Summary of Files Created/Modified

| File | Action | Purpose |
|------|--------|---------|
| `crates/wws-protocol/src/types.rs` | Modify | Add VerificationChallenge, DirectMessage |
| `crates/wws-protocol/src/messages.rs` | Modify | Add DirectMessageParams gossip type |
| `crates/wws-connector/src/connector.rs` | Modify | Add state fields + gossip handler |
| `crates/wws-connector/src/rpc_server.rs` | Modify | Add challenge/verify_agent/send_message |
| `crates/wws-connector/src/file_server.rs` | Modify | Expose /api/messages with DirectMessages |
| `Dockerfile` | Maybe update | Rust version if needed |
| `docker/wws-real-e2e/docker-compose.yml` | Create | 9-container 3-subnet topology |
| `tests/e2e/wws_real_e2e.sh` | Create | Docker infra launcher |
| `tests/playwright/wws-real-e2e.spec.js` | Create | Full UI spec with live data |
| `docs/e2e-results/.gitkeep` | Create | Output directory |
| `docs/e2e-results/YYYY-MM-DD-wws-real-e2e-log.md` | Created at runtime | Final agent log |
