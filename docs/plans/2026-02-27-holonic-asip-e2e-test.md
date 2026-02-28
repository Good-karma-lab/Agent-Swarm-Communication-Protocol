# Holonic ASIP End-to-End Test Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Run a 3-phase live E2E test verifying the complete Holonic ASIP workflow — board formation, critique, IRV voting, sub-holons, synthesis — using **Claude subagents** as the AI agents (no ZeroClaw, no OpenRouter key needed).

**Architecture:**
- Connectors start with `AGENT_IMPL=none` (connectors only, no external agent process)
- Main Claude session spawns N Task subagents in parallel, one per connector
- Each subagent connects to its connector via `nc` RPC, runs the full ASIP protocol (register → propose → vote → critique → execute → synthesize)
- Main session injects tasks and polls HTTP/RPC APIs to assert PASS/FAIL
- Phases: 1 node → 5 nodes → 20 nodes

**Tech Stack:** Bash connector startup, `nc` + `curl` for RPC/HTTP, `python3` for JSON, Claude Task subagents as ASIP agents.

**What each Claude subagent does:**
1. `swarm.get_status` → get agent_id
2. `swarm.register_agent` → register
3. Poll `swarm.receive_task` in a loop until a task appears
4. For coordinator role: `swarm.propose_plan` → `swarm.submit_vote` → `swarm.submit_critique`
5. For executor role: `swarm.submit_result` with real substantive content
6. For coordinator with completed subtasks: synthesize + `swarm.submit_result` with `is_synthesis: true`

---

## Port Layout

Each node gets sequential ports:
```
Node 1: p2p=9001, rpc=9370, files=9371
Node 2: p2p=9002, rpc=9372, files=9373
Node 3: p2p=9003, rpc=9374, files=9375
Node 4: p2p=9004, rpc=9376, files=9377
Node 5: p2p=9005, rpc=9378, files=9379
...
Node N: p2p=900N, rpc=9370+(N-1)*2, files=9371+(N-1)*2
```

Node 1's RPC (`9370`) is the bootstrap and the one used for task injection and verification.
Node 1's files port (`9371`) serves the web console.

---

## Timing

- Proposal stage timeout: **180s** (connector auto-selects sole proposal after 3 min)
- Voting stage timeout: **180s**
- Phase 1 expected: ~5–10 min
- Phase 2 expected: ~10–20 min
- Phase 3 expected: ~20–40 min

---

### Task 1: Add `AGENT_IMPL=none` to swarm-manager.sh

**Files:**
- Modify: `swarm-manager.sh`

The `start_agents` function currently requires one of `zeroclaw | opencode | claude-code-cli`. Add a `none` branch that skips agent startup but still writes the 6-column nodes.txt entry with `claude_pid=0`.

**Step 1: Find the agent selection block**

In `swarm-manager.sh`, after the connector startup block (~line 678), there is:
```bash
# Start AI agent (Claude Code CLI, OpenCode, or Zeroclaw)
if [ "$AGENT_IMPL" = "zeroclaw" ]; then
```

**Step 2: Add `none` branch before the zeroclaw branch**

Add at the very start of the agent selection block:
```bash
if [ "$AGENT_IMPL" = "none" ]; then
    echo -e "  ${YELLOW}ℹ${NC} AGENT_IMPL=none — connector only, no agent started"
    # claude_pid stays 0 in nodes.txt; nothing to update
elif [ "$AGENT_IMPL" = "zeroclaw" ]; then
```
And change the final `else` to properly close the chain.

**Step 3: Verify**

```bash
AGENT_IMPL=none ./swarm-manager.sh start-agents 1
# Expected: connector starts, no zeroclaw/claude process spawned
cat /tmp/openswarm-swarm/nodes.txt
# Expected: marie-curie|<pid>|0|9001|9370|9371
./swarm-manager.sh stop
```

---

### Task 2: Write the ASIP subagent instruction template

This is the instruction string that each Claude subagent receives. It must be self-contained — no file reads needed.

The template has placeholders: `{AGENT_INDEX}`, `{RPC_PORT}`, `{TASK_DESCRIPTION}` (injected by main session before spawning).

**Full subagent instruction:**

```
You are ASIP agent #{AGENT_INDEX}, connected to an ASIP.Connector at RPC port {RPC_PORT}.

Run the ASIP agent protocol. Use the Bash tool for ALL RPC calls via nc.

== SETUP ==

1. Get your agent identity:
   STATUS=$(echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"s","signature":""}' | nc -w 5 127.0.0.1 {RPC_PORT})
   Extract result.agent_id — this is your AGENT_ID (a "did:swarm:..." string).

2. Register:
   echo "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.register_agent\",\"params\":{\"agent_id\":\"$AGENT_ID\"},\"id\":\"reg\",\"signature\":\"\"}" | nc -w 5 127.0.0.1 {RPC_PORT}

== POLLING LOOP ==

Poll swarm.receive_task every 20 seconds (up to 30 minutes total).

RECEIVE=$(echo '{"jsonrpc":"2.0","method":"swarm.receive_task","params":{},"id":"recv","signature":""}' | nc -w 5 127.0.0.1 {RPC_PORT})

For each task_id in result.pending_tasks:

A. Get task details:
   TASK=$(echo "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.get_task\",\"params\":{\"task_id\":\"$TASK_ID\"},\"id\":\"gt\",\"signature\":\"\"}" | nc -w 5 127.0.0.1 {RPC_PORT})

B. Determine your role:
   - COORDINATOR: task.assigned_to is null/empty OR task.status is "Pending"
   - EXECUTOR: task.assigned_to == your AGENT_ID AND task.status is "InProgress"

== COORDINATOR WORKFLOW ==

Step 1 - PROPOSE a decomposition plan:
   - Create 3-5 subtasks with clear descriptions
   - Set estimated_complexity: 0.2-0.4 for simple subtasks, 0.5-0.8 for complex ones
   - A subtask with estimated_complexity > 0.4 will spawn a sub-holon (recursive board)

   PLAN_ID="plan-{AGENT_INDEX}-$(date +%s)"
   echo "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.propose_plan\",\"params\":{
     \"task_id\":\"$TASK_ID\",
     \"plan\":{
       \"plan_id\":\"$PLAN_ID\",
       \"proposer\":\"$AGENT_ID\",
       \"rationale\":\"[Your reasoning for this decomposition]\",
       \"subtasks\":[
         {\"index\":0,\"description\":\"[Subtask 1 description]\",\"required_capabilities\":[],\"estimated_complexity\":0.3},
         {\"index\":1,\"description\":\"[Subtask 2 description]\",\"required_capabilities\":[],\"estimated_complexity\":0.5},
         {\"index\":2,\"description\":\"[Subtask 3 description]\",\"required_capabilities\":[],\"estimated_complexity\":0.3}
       ],
       \"estimated_parallelism\":3,
       \"epoch\":1
     }
   },\"id\":\"prop\",\"signature\":\"\"}" | nc -w 5 127.0.0.1 {RPC_PORT}

Step 2 - VOTE after proposals are submitted (wait ~30s then check):
   VS=$(echo "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.get_voting_state\",\"params\":{\"task_id\":\"$TASK_ID\"},\"id\":\"vs\",\"signature\":\"\"}" | nc -w 5 127.0.0.1 {RPC_PORT})
   Extract all plan_ids from rfp_coordinators[].plan_ids.
   Rank them with OTHER agents' plans first (avoid self-vote).

   echo "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.submit_vote\",\"params\":{
     \"task_id\":\"$TASK_ID\",
     \"rankings\":[\"other-plan-id\",\"$PLAN_ID\"],
     \"epoch\":1
   },\"id\":\"vote\",\"signature\":\"\"}" | nc -w 5 127.0.0.1 {RPC_PORT}

Step 3 - CRITIQUE all proposals (IMPORTANT - this is new functionality being tested):
   Check if you are the adversarial critic:
   BOARD=$(echo '{"jsonrpc":"2.0","method":"swarm.get_board_status","params":{},"id":"board","signature":""}' | nc -w 5 127.0.0.1 {RPC_PORT})
   Check if result.holons[].adversarial_critic matches your AGENT_ID.

   For each plan_id in voting state, assign scores:
   - Normal critic: score fairly (feasibility/parallelism/completeness 0.6-0.9, risk 0.1-0.3)
   - Adversarial critic: be harsh (feasibility/completeness 0.2-0.4, risk 0.6-0.8) with detailed flaw analysis

   echo "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.submit_critique\",\"params\":{
     \"task_id\":\"$TASK_ID\",
     \"round\":2,
     \"plan_scores\":{
       \"$OTHER_PLAN_ID\":{\"feasibility\":0.8,\"parallelism\":0.7,\"completeness\":0.85,\"risk\":0.2},
       \"$PLAN_ID\":{\"feasibility\":0.75,\"parallelism\":0.8,\"completeness\":0.8,\"risk\":0.15}
     },
     \"content\":\"[Your critique rationale — be specific about strengths and weaknesses]\"
   },\"id\":\"crit\",\"signature\":\"\"}" | nc -w 5 127.0.0.1 {RPC_PORT}

Step 4 - SYNTHESIS (when your coordinator task has completed subtasks):
   Poll swarm.get_task for the parent task. When ALL subtask statuses = "Completed":
   - Get each subtask result via swarm.get_task
   - Synthesize them into a coherent unified response
   - Submit as the parent task result with is_synthesis: true

   echo "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.submit_result\",\"params\":{
     \"task_id\":\"$PARENT_TASK_ID\",
     \"artifact\":{
       \"artifact_id\":\"synth-{AGENT_INDEX}-$(date +%s)\",
       \"task_id\":\"$PARENT_TASK_ID\",
       \"producer\":\"$AGENT_ID\",
       \"content_cid\":\"sha256:synthesis\",
       \"merkle_hash\":\"synthesis-hash\",
       \"content_type\":\"text/plain\",
       \"size_bytes\":500,
       \"created_at\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"
     },
     \"content\":\"[SYNTHESIZED RESULT combining all subtask outputs]\",
     \"is_synthesis\":true
   },\"id\":\"synth\",\"signature\":\"\"}" | nc -w 5 127.0.0.1 {RPC_PORT}

== EXECUTOR WORKFLOW ==

When task.assigned_to == your AGENT_ID:
1. Think carefully about the task description and produce substantive output
2. Submit result:

   echo "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.submit_result\",\"params\":{
     \"task_id\":\"$TASK_ID\",
     \"artifact\":{
       \"artifact_id\":\"result-{AGENT_INDEX}-$(date +%s)\",
       \"task_id\":\"$TASK_ID\",
       \"producer\":\"$AGENT_ID\",
       \"content_cid\":\"sha256:$(echo '$RESULT_TEXT' | sha256sum | cut -c1-16)\",
       \"merkle_hash\":\"result-hash\",
       \"content_type\":\"text/plain\",
       \"size_bytes\":$(echo -n '$RESULT_TEXT' | wc -c),
       \"created_at\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"
     },
     \"content\":\"$RESULT_TEXT\"
   },\"id\":\"res\",\"signature\":\"\"}" | nc -w 5 127.0.0.1 {RPC_PORT}

== STOPPING ==

Stop after completing one full cycle per task OR after 30 minutes total.
Report what you did: tasks seen, plans proposed, votes cast, critiques submitted, results submitted.
```

---

### Task 3: Write connector startup script

**Files:**
- Create: `tests/e2e/start_connectors.sh`

This script starts N connectors connected to each other (no agents) and writes a port map.

```bash
#!/usr/bin/env bash
# Start N ASIP connector nodes without agents.
# Usage: bash tests/e2e/start_connectors.sh N
# Output: /tmp/asip-test-nodes.txt  (one line per node: name|pid|rpc_port|files_port)

set -euo pipefail

N="${1:-1}"
BIN="./target/release/openswarm-connector"
SWARM_DIR="/tmp/asip-test"
NODES_FILE="$SWARM_DIR/nodes.txt"

mkdir -p "$SWARM_DIR"
> "$NODES_FILE"

# Kill any leftover connectors
pkill -f 'openswarm-connector' 2>/dev/null || true
sleep 1

BOOTSTRAP_ADDR=""

for i in $(seq 1 "$N"); do
    NAME="asip-node-$i"
    P2P_PORT=$((9000 + i))
    RPC_PORT=$((9368 + i * 2))      # 9370, 9372, 9374 ...
    FILES_PORT=$((9369 + i * 2))    # 9371, 9373, 9375 ...
    LOG="$SWARM_DIR/$NAME.log"

    CMD=("$BIN"
        "--listen" "/ip4/127.0.0.1/tcp/$P2P_PORT"
        "--rpc"    "127.0.0.1:$RPC_PORT"
        "--files-addr" "127.0.0.1:$FILES_PORT"
        "--agent-name" "$NAME"
    )
    if [[ -n "$BOOTSTRAP_ADDR" ]]; then
        CMD+=("--bootstrap" "$BOOTSTRAP_ADDR")
    fi

    "${CMD[@]}" >"$LOG" 2>&1 &
    PID=$!
    echo "$NAME|$PID|$RPC_PORT|$FILES_PORT" >> "$NODES_FILE"
    echo "  Started $NAME (pid=$PID rpc=$RPC_PORT files=$FILES_PORT)"

    sleep 2

    # Explicit connect for non-bootstrap nodes
    if [[ -n "$BOOTSTRAP_ADDR" && $i -gt 1 ]]; then
        echo '{"jsonrpc":"2.0","method":"swarm.connect","params":{"addr":"'"$BOOTSTRAP_ADDR"'"},"id":"c","signature":""}' \
            | nc -w 5 127.0.0.1 "$RPC_PORT" >/dev/null 2>&1 || true
    fi

    # Get bootstrap addr from node 1
    if [[ $i -eq 1 ]]; then
        sleep 1
        PEER_ID=$(echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"s","signature":""}' \
            | nc -w 5 127.0.0.1 "$RPC_PORT" 2>/dev/null \
            | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('result',{}).get('agent_id','').replace('did:swarm:',''))" 2>/dev/null || echo "")
        if [[ -n "$PEER_ID" ]]; then
            BOOTSTRAP_ADDR="/ip4/127.0.0.1/tcp/$P2P_PORT/p2p/$PEER_ID"
            echo "  Bootstrap: $BOOTSTRAP_ADDR"
        fi
    fi
done

echo ""
echo "Nodes written to $NODES_FILE"
cat "$NODES_FILE"
```

---

### Task 4: Phase 1 implementation — 1 node, 1 subagent

**What this phase does:**
1. Build binary, start 1 connector
2. Spawn 1 Claude subagent (background Task)
3. Inject a simple task via RPC
4. Poll HTTP APIs: HolonState, deliberation thread, task status
5. Assert PASS/FAIL

**Steps for the main session to execute:**

```
STEP 1: Build release binary
  ~/.cargo/bin/cargo build --release

STEP 2: Start 1 connector
  bash tests/e2e/start_connectors.sh 1
  # Verify: cat /tmp/asip-test/nodes.txt shows 1 entry

STEP 3: Wait for RPC ready
  # Poll until:
  echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"s","signature":""}' | nc -w 5 127.0.0.1 9370
  # Expected: result.agent_id = "did:swarm:..."

STEP 4: Spawn 1 Claude subagent (Task tool, background=true)
  Agent index: 1
  RPC port: 9370
  Instruction: use Task 2 template with {AGENT_INDEX}=1, {RPC_PORT}=9370

STEP 5: Inject a task from main session
  echo '{"jsonrpc":"2.0","method":"swarm.inject_task","params":{
    "description":"Analyze the trade-offs between consensus algorithms (Raft vs PBFT vs HotStuff) and recommend the best choice for a 100-node permissioned blockchain"
  },"id":"inj","signature":""}' | nc -w 5 127.0.0.1 9370
  # Save TASK_ID from result

STEP 6: Assert checks (poll every 20s, up to 15 min)
  CHECK 1: GET http://127.0.0.1:9371/api/health → {"ok":true}
  CHECK 2: GET http://127.0.0.1:9371/api/holons → list (non-error)
  CHECK 3: GET http://127.0.0.1:9371/api/holons/$TASK_ID → has "task_id" field
  CHECK 4: GET http://127.0.0.1:9371/api/tasks → task $TASK_ID status != "Pending"
  CHECK 5: GET http://127.0.0.1:9371/api/tasks/$TASK_ID/deliberation → >=1 message
  CHECK 6: swarm.get_board_status → holons[0].adversarial_critic is non-null
  BONUS:   Task status reaches "Completed"

STEP 7: Report Phase 1 results
```

---

### Task 5: Phase 2 implementation — 5 nodes, 5 subagents

**Steps for main session:**

```
STEP 1: Stop phase 1 connectors
  pkill -f openswarm-connector || true; sleep 2

STEP 2: Start 5 connectors
  bash tests/e2e/start_connectors.sh 5
  # Ports: rpc=9370,9372,9374,9376,9378 files=9371,9373,9375,9377,9379

STEP 3: Wait 15s for peer discovery, then verify >=2 agents visible
  echo '{"jsonrpc":"2.0","method":"swarm.get_network_stats","params":{},"id":"ns","signature":""}' | nc -w 5 127.0.0.1 9370
  # Check result.total_agents >= 2

STEP 4: Open web console
  open http://127.0.0.1:9371/
  # Manual UI checks listed below

STEP 5: Spawn 5 Claude subagents in PARALLEL (single Task tool message, all background)
  Agent 1 → RPC port 9370
  Agent 2 → RPC port 9372
  Agent 3 → RPC port 9374
  Agent 4 → RPC port 9376
  Agent 5 → RPC port 9378
  All use Task 2 template with their index and port.

STEP 6: Inject a rich task
  echo '{"jsonrpc":"2.0","method":"swarm.inject_task","params":{
    "description":"Design a REST API for a multi-tenant SaaS task management platform with OAuth2 authentication, team workspaces, real-time WebSocket updates, role-based access control, and Stripe billing integration",
    "task_type":"software_architecture",
    "horizon":"medium",
    "capabilities_required":["api-design","distributed-systems"]
  },"id":"inj2","signature":""}' | nc -w 5 127.0.0.1 9370

STEP 7: Assert checks (poll every 20s, up to 25 min)
  CHECK 1: GET /api/holons/$TASK_ID → status progresses Forming→Deliberating→Voting→Executing→Done
  CHECK 2: GET /api/tasks/$TASK_ID/deliberation → >=1 ProposalSubmission (round=1)
  CHECK 3: GET /api/tasks/$TASK_ID/deliberation → >=1 CritiqueFeedback (round=2) with non-empty critic_scores
  CHECK 4: GET /api/holons → adversarial_critic non-null on task holon
  CHECK 5: GET /api/tasks/$TASK_ID/irv-rounds → >=1 IrvRound with tallies
  CHECK 6: GET /api/tasks/$TASK_ID/ballots → >=1 BallotRecord
  CHECK 7: GET /api/tasks → task status = "Completed"
  CHECK 8: GET /api/holons/$TASK_ID → status = "Done"

STEP 8: Manual UI verification
  - Holons tab visible
  - HolonTreePanel shows task holon with correct status color
  - Click holon → detail shows chair, members, adversarial_critic
  - Deliberation tab: Round 1 proposals + Round 2 critiques (⚔ on adversarial)
  - VotingPanel: Ballot Breakdown with per-voter rows + feasibility/parallelism bars
  - VotingPanel: IRV Rounds section with elimination history
```

---

### Task 6: Phase 3 implementation — 20 nodes, 20 subagents

**Steps for main session:**

```
STEP 1: Stop phase 2 connectors
  pkill -f openswarm-connector || true; sleep 2

STEP 2: Start 20 connectors
  bash tests/e2e/start_connectors.sh 20
  # Node 1 ports: rpc=9370, files=9371 (same as always)

STEP 3: Wait 30s for mesh to stabilize
  # Poll total_agents until >= 5 agents visible

STEP 4: Spawn 20 Claude subagents in PARALLEL
  Agents 1-20 → RPC ports 9370,9372,...,9408
  Use Task 2 template. All background=true.
  NOTE: Claude API rate limits may slow some agents — that's acceptable.
        The voting timeout (180s) ensures the task proceeds even if some agents are slow.

STEP 5: Inject a long-horizon complex task
  Designed to trigger sub-holon formation (some subtasks should get estimated_complexity > 0.4):
  echo '{"jsonrpc":"2.0","method":"swarm.inject_task","params":{
    "description":"Design and specify a distributed Byzantine-fault-tolerant consensus protocol for 10000 heterogeneous AI agents: formal TLA+ specification, safety proofs, Rust implementation plan, and comprehensive benchmarking methodology",
    "task_type":"research",
    "horizon":"long",
    "capabilities_required":["distributed-systems","consensus","formal-methods","rust"],
    "backtrack_allowed":true,
    "knowledge_domains":["BFT","HotStuff","PBFT","libp2p","TLA+"]
  },"id":"inj3","signature":""}' | nc -w 5 127.0.0.1 9370

STEP 6: Assert checks (poll every 30s, up to 40 min)
  CHECK 1: total_agents >= 5 in get_network_stats
  CHECK 2: >=2 ProposalSubmission in deliberation
  CHECK 3: >=2 CritiqueFeedback in deliberation
  CHECK 4: >=2 IRV rounds (multiple proposals competing)
  CHECK 5: Task reaches "Completed"
  ADVISORY: /api/holons shows depth>0 holon (sub-holon formation)
  ADVISORY: SynthesisResult message in deliberation thread

STEP 7: Print full deliberation breakdown
  Show count per message_type: ProposalSubmission, CritiqueFeedback, SynthesisResult
```

---

## Execution Order

The test is run **live in the Claude Code session** (not as a standalone script):

1. **Task 1**: Add `AGENT_IMPL=none` to swarm-manager.sh
2. **Task 3**: Write `tests/e2e/start_connectors.sh`
3. **Task 4**: Execute Phase 1 (build → start 1 node → spawn 1 subagent → assert)
4. **Task 5**: Execute Phase 2 (start 5 nodes → spawn 5 subagents → assert + UI)
5. **Task 6**: Execute Phase 3 (start 20 nodes → spawn 20 subagents → assert)

Tasks 2 (subagent instruction template) is used inline during tasks 4-6.

---

## Pass Criteria

| Check | Ph1 | Ph2 | Ph3 |
|-------|-----|-----|-----|
| Connector health ok | ✓ | ✓ | ✓ |
| Task injection works | ✓ | ✓ | ✓ |
| HolonState created | ✓ | ✓ | ✓ |
| >=1 ProposalSubmission | ✓ | ✓ | ✓ |
| Task leaves Pending | ✓ | ✓ | ✓ |
| >=1 CritiqueFeedback w/ scores | — | ✓ | ✓ |
| adversarial_critic set | — | ✓ | — |
| >=1 IRV round | — | ✓ | ✓ |
| >=1 BallotRecord | — | ✓ | — |
| Task Completed | — | ✓ | ✓ |
| HolonState Done | — | ✓ | — |
| >=2 proposals competed | — | — | ✓ |
| Sub-holons (advisory) | — | — | adv |
| SynthesisResult (advisory) | — | — | adv |
