# WWS Real End-to-End Test — Design Document

**Date:** 2026-03-01
**Status:** Approved

---

## Goal

Run a real end-to-end test of the entire WWS stack: 8 Claude AI subagents connect through real `wws-connector` binaries deployed across 3 Docker subnets, collaborate on a complex research task using the full holonic protocol, exhibit genuine social behavior between phases, and pass an anti-bot verification challenge on join. Produce a markdown log with real agent text (not IDs).

---

## Architecture

### Docker Topology

3 Docker bridge networks, multi-homed bootstrap:

```
Host machine (Claude subagents connect via mapped ports)
│
├── wws-bootstrap-net (172.20.0.0/24)
│   └── bootstrap-node (172.20.0.10)
│       --bootstrap-mode (relay server, no agent)
│       --listen /ip4/0.0.0.0/tcp/9000
│       --files-addr 0.0.0.0:9371  (HTTP → host:9371)
│       Attached to: ALL 3 networks (multi-homed relay)
│
├── wws-tier1-net (172.21.0.0/24)
│   ├── connector-alpha (172.21.0.10)
│   │   --rpc 0.0.0.0:9370  (→ host:9370)
│   │   --files-addr 0.0.0.0:9381 (→ host:9381)
│   │   --bootstrap /ip4/172.20.0.10/tcp/9000/p2p/<BOOTSTRAP_PEER>
│   └── connector-beta (172.21.0.11)
│       --rpc 0.0.0.0:9372  (→ host:9372)
│       --files-addr 0.0.0.0:9383 (→ host:9383)
│       --bootstrap /ip4/172.20.0.10/tcp/9000/p2p/<BOOTSTRAP_PEER>
│
└── wws-tier2-net (172.22.0.0/24)
    ├── connector-raft      (172.22.0.10) RPC→host:9374, HTTP→host:9385
    ├── connector-pbft      (172.22.0.11) RPC→host:9376, HTTP→host:9387
    ├── connector-paxos     (172.22.0.12) RPC→host:9378, HTTP→host:9389
    ├── connector-tendermint(172.22.0.13) RPC→host:9380, HTTP→host:9391
    ├── connector-hashgraph (172.22.0.14) RPC→host:9382, HTTP→host:9393
    └── connector-synth     (172.22.0.15) RPC→host:9384, HTTP→host:9395
```

Tier1↔Tier2 communication routes through the bootstrap relay (true cross-subnet NAT traversal test).

---

## Agent Roles

| Agent | wws:name | Subnet | RPC Port | Role |
|-------|----------|--------|----------|------|
| coordinator-alpha | wws:alpha | Tier1 | 9370 | RFP coordinator, task decomposer, synthesis lead |
| coordinator-beta | wws:beta | Tier1 | 9372 | Competing coordinator, plan critic |
| researcher-raft | wws:raft | Tier2 | 9374 | Research Raft consensus |
| researcher-pbft | wws:pbft | Tier2 | 9376 | Research PBFT |
| researcher-paxos | wws:paxos | Tier2 | 9378 | Research Paxos |
| researcher-tendermint | wws:tendermint | Tier2 | 9380 | Research Tendermint |
| researcher-hashgraph | wws:hashgraph | Tier2 | 9382 | Research Hashgraph/DAG |
| synthesizer | wws:synth | Tier2 | 9384 | Synthesize all research findings |

---

## Anti-Bot Verification Challenge

When an agent first calls `swarm.register_agent`, the connector returns a challenge instead of accepting immediately. The challenge uses obfuscated mixed-caps/symbol encoding that requires LLM reasoning to decode:

```json
{
  "result": null,
  "challenge": {
    "code": "wws_verify_<random_hex>",
    "text": "T hE} FiRsT^ ClOuD{ hAs 2 3} LaYeRs aNd| ThE sEcOnD{ HaS H aLf^ aS MaNy|, HoW mAnY LaYeRs In ToTaL?"
  }
}
```

The agent decodes the garbled text, solves the math question, and calls `swarm.verify_agent { code, answer }`. Only then is registration accepted. A bot/script with no LLM reasoning will fail every new random challenge.

**Implementation:** New `VerificationChallenge` struct in connector state. Challenge generated on first `register_agent`, stored per-agent, cleared on successful verify. Question types: arithmetic, simple logic (always integer answer to avoid float comparison issues).

---

## WWS Features Tested

### 1. Identity & Key Management
- Each connector starts with `--identity-path /data/{name}.key`
- After all phases, bootstrap node restarts with same identity-path
- Test verifies DID is identical before and after restart

### 2. Name Registry
- Each agent registers `wws:{name}` via HTTP POST `/api/names` on startup
- Agents resolve each other's names via RPC `swarm.resolve_name` before direct messaging
- Cross-subnet resolution: Tier2 agent resolves Tier1 name through bootstrap relay

### 3. P2P Direct Messaging
- Agents use gossip topic `/wws/1.0.0/s/public/messages/{recipient_did}` for direct messages
- RPC method `swarm.send_message { recipient_did, content }` (to be verified/added)
- All message texts captured in log

### 4. Social Communication (non-task)
- **Greeting phase:** Each agent broadcasts a greeting with personality
- **Curiosity phase:** Agents ask each other about their domain expertise
- **Mid-task chatter:** While researchers work, they comment on each other's progress
- **Post-task social:** Agents celebrate completion, share opinions on consensus algorithms
- These are genuine Claude responses, not scripted

### 5. Holonic Task Execution
- **Task:** "Research and compare 5 consensus algorithms for use in agent swarms: analyze Raft, PBFT, Paxos, Tendermint, and Hashgraph DAG. For each: describe mechanism, fault tolerance, latency, and suitability for agent swarms. Produce a final synthesis and recommendation."
- **RFP flow:** Tier1 election → coordinator-alpha wins → proposes 6-subtask plan → coordinator-beta critiques → IRV vote → plan executes
- **Holon formation:** board.invite/accept/ready for all 8 agents
- **Parallel execution:** 5 researchers work simultaneously (5 Claude subagents run in parallel)
- **Synthesis:** synthesizer receives all 5 results, produces final recommendation

### 6. Deliberation (Critique Phase)
- coordinator-beta produces a real critique of coordinator-alpha's decomposition plan
- Scores each subtask on feasibility, parallelism, completeness, risk
- Critique text is substantive (2-3 paragraphs of real reasoning)

### 7. IRV Voting
- All 6 Tier2 agents vote on proposals
- IRV elimination rounds captured in log

### 8. Reputation & Tier
- Initial: score=10, tier=newcomer for all
- After task: each agent that executed a subtask should see score increase
- Verify via `/api/reputation` on each node

### 9. Browser UI (Playwright)
Runs against `http://127.0.0.1:9371` (bootstrap node HTTP):
- Graph view: all 9 nodes visible with connections
- Directory: all 8 agents listed with names, tiers, DIDs
- Activity view: tasks, messages, holons
- Task Detail panel: full deliberation thread with real text
- Holon Detail panel: tree of 8 members
- Agent Detail panel: click on an agent, see their info
- Submit Task modal: functional
- Name Registry panel: list registered names
- Key Management panel: shows Ed25519 key info
- Reputation panel: shows score and tier progress bar
- Audit panel: shows event timeline

---

## Log File Format

**Output:** `docs/e2e-results/YYYY-MM-DD-wws-real-e2e-log.md`

Structure:
```markdown
# WWS Real E2E Test Log — {date}

## Infrastructure
[table: agent, DID, subnet, port]

## Phase 0: Anti-Bot Verification
[per agent: challenge text → agent's decoded answer → PASS/FAIL]

## Phase 1: Agent Registration & Name Claims
[per agent: wws:name registered, HTTP POST result]

## Phase 2: Identity Persistence Test
[before/after restart DID comparison]

## Phase 3: Greeting & Social Communication
[timestamped messages with full text]

## Phase 4: Name Resolution Cross-Subnet
[resolve attempts and results]

## Phase 5: Task Submission
[task text, submitter]

## Phase 6: RFP — Coordinator Election & Proposals
[coordinator-alpha plan: full rationale + subtasks]
[coordinator-beta plan: full rationale + subtasks]

## Phase 7: Deliberation — Critiques
[coordinator-beta critique of alpha's plan: full text + scores]
[coordinator-alpha critique of beta's plan: full text + scores]

## Phase 8: IRV Voting
[rankings per agent, IRV rounds, winner]

## Phase 9: Holon Formation
[board.invite/accept events, final holon roster]

## Phase 10: Subtask Execution (Parallel)
[per researcher: full research text (500-800 words)]

## Phase 11: Synthesis
[synthesizer: full recommendation text (300-500 words)]

## Phase 12: Post-Task Social Communication
[agents celebrate, discuss findings, chat freely]

## Phase 13: Reputation Scores After Task
[before/after table per agent]

## Phase 14: Browser UI Verification
[per panel: PASS/FAIL + screenshot description]

## Overall Result
PASS ✓ / FAIL ✗
[summary of what passed/failed]
```

---

## Implementation Approach

**Option A (chosen):** Claude subagents directly operate WWS RPC protocol as their agent loops. No wrapper scripts. Each subagent:
1. Calls `swarm.register_agent` RPC → handles verification challenge (Claude decodes naturally)
2. Reads its assigned role and task from the log context
3. Performs its function (coordinator: proposes plans; researcher: does research; synthesizer: synthesizes)
4. Also sends social messages between phases
5. Writes all actions + full text to the shared log file

**New connector code needed:**
- `swarm.register_agent` returns verification challenge on first call (new)
- `swarm.verify_agent` to complete registration (new)
- Verification challenge generation: arithmetic/logic questions in garbled mixed-caps encoding
- `swarm.send_message { recipient_did, content }` for direct P2P messaging (verify if already exists)

**Orchestration script:** `tests/e2e/wws_real_e2e.sh` — launches Docker containers, waits for bootstrap, then the main orchestrator (a Claude subagent in its own right) coordinates everything.

---

## Test Files to Create

1. **`docker/wws-real-e2e/docker-compose.yml`** — 9-container setup (1 bootstrap + 8 connectors)
2. **`crates/wws-connector/src/connector.rs`** — add verification challenge to `register_agent`, add `verify_agent`, add `send_message`
3. **`tests/e2e/wws_real_e2e.sh`** — Docker launch + orchestration
4. **`tests/e2e/wws_claude_agent.sh`** — reusable Claude subagent wrapper that runs an agent loop
5. **`tests/playwright/wws-real-e2e.spec.js`** — full UI panel spec (extends wws-features-e2e.spec.js)
6. **`docs/e2e-results/`** — directory for log output
