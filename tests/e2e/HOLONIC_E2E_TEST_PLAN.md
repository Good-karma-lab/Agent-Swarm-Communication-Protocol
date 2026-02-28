# Holonic Swarm E2E Test Plan

**Stack**: ZeroClaw + OpenRouter (`arcee-ai/trinity-large-preview:free`)
**New functionality under test**: board formation, CritiquePhase, IrvRound history,
BallotRecord persistence, HolonState lifecycle, new HTTP API endpoints, new UI panels.

---

## Prerequisites

```bash
# .env must have:
AGENT_IMPL=zeroclaw
LLM_BACKEND=openrouter
MODEL_NAME=arcee-ai/trinity-large-preview:free
OPENROUTER_API_KEY=<your-key>

# Build release binary
~/.cargo/bin/cargo build --release

# Ensure zeroclaw is installed
which zeroclaw || pip install zeroclaw
```

---

## Phase 1 — Single Node Smoke Test (1 connector + 1 agent)

**Goal**: Verify ZeroClaw connects to the ASIP.Connector, registers as agent,
polls for tasks, and the basic holonic API surface is reachable.

**Expected duration**: ~2–4 minutes

### Steps

```bash
./swarm-manager.sh start-agents 1
sleep 20   # allow connector startup + p2p init
```

**RPC_PORT** = first port in `/tmp/openswarm-swarm/nodes.txt` (column 5, default 9370)
**HTTP_PORT** = RPC_PORT + 1 (default 9371)

#### 1.1 Connector health
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/health
# Expected: {"ok": true, ...}

echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"1","signature":""}' \
  | nc 127.0.0.1 $RPC_PORT
# Expected: result.agent_id starts with "did:swarm:", result.epoch >= 1
```

#### 1.2 Agent registered
```bash
echo '{"jsonrpc":"2.0","method":"swarm.get_network_stats","params":{},"id":"2","signature":""}' \
  | nc 127.0.0.1 $RPC_PORT
# Expected: result.total_agents >= 1
```

#### 1.3 HolonState API reachable (empty at rest)
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/holons
# Expected: [] (empty array, no active holons yet)
```

#### 1.4 Inject minimal task
```bash
echo '{"jsonrpc":"2.0","method":"swarm.inject_task","params":{
  "description":"Write a Python function that returns the nth Fibonacci number"
},"id":"t1","signature":""}' | nc 127.0.0.1 $RPC_PORT
# Expected: result.task_id is a UUID string
# Save: TASK_ID=$(...)
```

#### 1.5 HolonState created on injection
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/holons
# Expected: array with 1 entry; entry.status == "Forming"
# entry.task_id == $TASK_ID, entry.chair is a DID
```

#### 1.6 Agent polls and processes (up to 3 min)
Poll every 15s:
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/tasks | python3 -c "
import json,sys; tasks=json.load(sys.stdin)
t=[t for t in tasks if t['task_id']=='$TASK_ID']
print(t[0]['status'] if t else 'not found')"
# Pass when status reaches: InProgress, Completed, or Failed
```

#### 1.7 Deliberation thread appears
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/tasks/$TASK_ID/deliberation
# Expected: array with >= 1 DeliberationMessage (ProposalSubmission at minimum)
# Each message has: id, task_id, speaker, round, message_type, content, timestamp
```

### Phase 1 Pass Criteria
- [ ] Connector health: `{"ok": true}`
- [ ] Agent DID visible in `swarm.get_status`
- [ ] `GET /api/holons` returns holon with task_id matching injected task
- [ ] `GET /api/tasks/:id/deliberation` returns ≥1 message
- [ ] Task reaches status != Pending within 3 minutes

---

## Phase 2 — 5 Nodes: Full Board Deliberation

**Goal**: Board forms from multiple agents (≥3 members), Round 1 commit-reveal runs,
Round 2 critique fires with populated CriticScore, IRV completes, result submitted.
All new API endpoints return real data.

**Expected duration**: ~8–15 minutes

### Steps

```bash
./swarm-manager.sh stop
./swarm-manager.sh start-agents 5
sleep 45   # allow p2p peer discovery across all 5 nodes
```

**RPC_PORT** = port of node 1 from `/tmp/openswarm-swarm/nodes.txt`

#### 2.1 Swarm has 5 peers
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/topology
# Expected: nodes array has >= 4 entries (peers of node 1)

echo '{"jsonrpc":"2.0","method":"swarm.get_network_stats","params":{},"id":"1","signature":""}' \
  | nc 127.0.0.1 $RPC_PORT
# Expected: result.total_agents >= 5
```

#### 2.2 Inject complex task with holonic fields
```bash
echo '{"jsonrpc":"2.0","method":"swarm.inject_task","params":{
  "description": "Design a REST API for a task management system with authentication, CRUD endpoints, rate limiting, and webhook support",
  "task_type": "software_architecture",
  "horizon": "medium",
  "capabilities_required": ["api-design", "distributed-systems"],
  "backtrack_allowed": true
},"id":"t2","signature":""}' | nc 127.0.0.1 $RPC_PORT
# Save TASK_ID
```

#### 2.3 Board forms with ≥3 members
Poll `GET /api/holons/$TASK_ID` every 10s for up to 2 min:
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/holons/$TASK_ID
# Pass when:
#   status == "Deliberating" AND members array has >= 3 entries
#   adversarial_critic field is non-null
```

#### 2.4 Round 1 proposals appear in deliberation thread
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/tasks/$TASK_ID/deliberation
# Pass when: array contains ≥1 message with message_type == "ProposalSubmission"
#            and round == 1
```

#### 2.5 Round 2 critique messages appear with real CriticScore
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/tasks/$TASK_ID/deliberation | python3 -c "
import json, sys
msgs = json.load(sys.stdin)
critiques = [m for m in msgs if m['message_type'] == 'CritiqueFeedback']
print(f'{len(critiques)} critique messages found')
if critiques:
    scores = critiques[0].get('critic_scores', {})
    print(f'  plan IDs scored: {list(scores.keys())}')
    for plan_id, s in scores.items():
        print(f'  {plan_id[:8]}... feasibility={s[\"feasibility\"]} risk={s[\"risk\"]}')
"
# Pass when: >= 1 CritiqueFeedback with non-empty critic_scores dict
#            critic_scores values have feasibility, parallelism, completeness, risk != 0
```

#### 2.6 IRV voting completes with round history
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/tasks/$TASK_ID/irv-rounds | python3 -c "
import json, sys
rounds = json.load(sys.stdin)
print(f'{len(rounds)} IRV rounds recorded')
for r in rounds:
    print(f'  Round {r[\"round_number\"]}: tallies={r[\"tallies\"]} eliminated={r[\"eliminated\"]}')
"
# Pass when: >= 1 IrvRound with non-empty tallies
```

#### 2.7 Per-voter ballot records with critic scores
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/tasks/$TASK_ID/ballots | python3 -c "
import json, sys
ballots = json.load(sys.stdin)
print(f'{len(ballots)} ballot records')
for b in ballots:
    print(f'  voter={b[\"voter\"][:20]}... rankings={b[\"rankings\"]} critic_plans={list(b[\"critic_scores\"].keys())[:2]}')
"
# Pass when: >= 2 BallotRecord entries, each with rankings and critic_scores populated
```

#### 2.8 Task reaches Completed with result
Poll `GET /api/tasks` every 20s for up to 10 min:
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/tasks | python3 -c "
import json, sys
tasks = json.load(sys.stdin)
t = next((t for t in tasks if t['task_id']=='$TASK_ID'), None)
print(t['status'] if t else 'not found')
"
# Pass when: status == "Completed"
```

#### 2.9 HolonState reaches Done after completion
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/holons/$TASK_ID
# Expected: status == "Done"
```

#### 2.10 Holon tree via /api/holons
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/holons | python3 -c "
import json, sys
holons = json.load(sys.stdin)
print(f'{len(holons)} holons total')
for h in holons:
    print(f'  task={h[\"task_id\"][:8]}... depth={h[\"depth\"]} status={h[\"status\"]} members={len(h[\"members\"])}')
"
```

#### 2.11 Web console UI verification
```bash
open http://127.0.0.1:$HTTP_PORT/
```
Manual checks:
- [ ] "Holons" tab is visible in sidebar
- [ ] "Deliberation" tab is visible in sidebar
- [ ] HolonTreePanel shows the task holon with correct status color
- [ ] Click holon node → detail panel shows chair, members, adversarial critic
- [ ] DeliberationPanel shows proposal messages (round 1) and critique messages (round 2, ⚔️ for adversarial critic)
- [ ] VotingPanel "Ballot Breakdown" table shows per-voter rows with feasibility/parallelism bars
- [ ] VotingPanel "IRV Rounds" section shows round-by-round elimination

### Phase 2 Pass Criteria
- [ ] Board formed: ≥3 members, adversarial_critic non-null
- [ ] Deliberation: ≥1 ProposalSubmission (round 1) + ≥1 CritiqueFeedback (round 2)
- [ ] CriticScore values non-zero in critique messages
- [ ] IRV rounds: ≥1 IrvRound with populated tallies
- [ ] Ballots: ≥2 BallotRecord with per-voter rankings + critic_scores
- [ ] Task status reaches Completed
- [ ] HolonState.status reaches Done
- [ ] UI: all 3 new panels render data correctly

---

## Phase 3 — 20 Nodes: Scale + Recursive Holons

**Goal**: At 20 agents, verify swarm-scale peer discovery, larger boards, multiple
deliberation participants, and that a complex task drives real work through the chain.

**Expected duration**: ~20–35 minutes

### Steps

```bash
./swarm-manager.sh stop
./swarm-manager.sh start-agents 20
sleep 90   # longer stabilization for 20-node p2p mesh
```

#### 3.1 Swarm topology stable
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/topology | python3 -c "
import json, sys; d=json.load(sys.stdin); print(len(d.get('nodes',[])), 'nodes in topology')"
# Expected: >= 15 nodes visible (some may not yet have propagated)

echo '{"jsonrpc":"2.0","method":"swarm.get_network_stats","params":{},"id":"1","signature":""}' \
  | nc 127.0.0.1 $RPC_PORT | python3 -c "
import json,sys; d=json.loads(sys.stdin.read())
print('agents:', d['result']['total_agents'], '| hierarchy_depth:', d['result']['hierarchy_depth'])"
# Expected: total_agents >= 15, hierarchy_depth >= 2
```

#### 3.2 Inject long-horizon scientific task
```bash
echo '{"jsonrpc":"2.0","method":"swarm.inject_task","params":{
  "description": "Design a distributed consensus protocol for a network of 10000 AI agents that is Byzantine-fault tolerant, achieves sub-second latency, and scales linearly with node count",
  "task_type": "research",
  "horizon": "long",
  "capabilities_required": ["distributed-systems", "consensus", "fault-tolerance"],
  "backtrack_allowed": true,
  "knowledge_domains": ["BFT", "Raft", "PBFT", "HotStuff", "libp2p"],
  "tools_available": ["literature_search", "formal_verification"]
},"id":"t3","signature":""}' | nc 127.0.0.1 $RPC_PORT
# Save TASK_ID
```

#### 3.3 Large board forms (5–10 members expected)
Poll `GET /api/holons/$TASK_ID` for up to 3 min:
```bash
# Pass when: members array has >= 5 agents
# Verify: adversarial_critic is set
```

#### 3.4 Multiple critique rounds with diverse opinions
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/tasks/$TASK_ID/deliberation | python3 -c "
import json, sys
msgs = json.load(sys.stdin)
by_type = {}
for m in msgs:
    by_type.setdefault(m['message_type'], []).append(m)
for t, ms in by_type.items():
    print(f'{t}: {len(ms)} messages')
"
# Expected breakdown roughly:
#   ProposalSubmission: 5-10
#   CritiqueFeedback:   5-10
#   SynthesisResult:    1 (if synthesis step fires)
```

#### 3.5 IRV with multiple elimination rounds
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/tasks/$TASK_ID/irv-rounds | python3 -c "
import json, sys
rounds = json.load(sys.stdin)
print(f'{len(rounds)} IRV rounds (expect >1 when >3 proposals)')
for r in rounds:
    elim = r['eliminated'] or 'WINNER'
    print(f'  Round {r[\"round_number\"]}: {len(r[\"tallies\"])} candidates, eliminated={elim[:8] if elim != \"WINNER\" else \"WINNER\"}')"
# Pass when: >= 2 rounds (meaning >2 proposals competed)
```

#### 3.6 All 20 ballot records present
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/tasks/$TASK_ID/ballots | python3 -c "
import json, sys
b = json.load(sys.stdin)
print(f'{len(b)} ballot records (senate subset of 20 agents)')
scored = [r for r in b if r['critic_scores']]
print(f'{len(scored)} ballots have critic scores populated')"
# Pass when: >= 3 ballots, majority have critic_scores populated
```

#### 3.7 /api/holons returns full tree structure
```bash
curl -s http://127.0.0.1:$HTTP_PORT/api/holons | python3 -c "
import json, sys
holons = json.load(sys.stdin)
print(f'{len(holons)} holons total')
max_depth = max(h['depth'] for h in holons) if holons else 0
print(f'max depth: {max_depth}')
for h in holons:
    indent = '  ' * h['depth']
    print(f'{indent}depth={h[\"depth\"]} status={h[\"status\"]} members={len(h[\"members\"])}')"
# Pass when: >= 1 holon with depth 0 and members >= 5
# Bonus: depth > 0 entries would indicate recursive sub-holon formation
```

#### 3.8 Task completes end-to-end
Poll task status every 30s for up to 25 min:
```bash
# Pass when status == "Completed"
# Result artifact should have non-empty content field
```

#### 3.9 Web console at scale
```bash
open http://127.0.0.1:$HTTP_PORT/
```
Manual checks:
- [ ] HolonTreePanel renders all holons without crash (20-agent scale)
- [ ] Clicking any holon shows correct member count and status
- [ ] DeliberationPanel loads full message thread (may be 20+ messages)
- [ ] VotingPanel ballot table shows all senate members
- [ ] IRV rounds animation shows multi-round elimination

### Phase 3 Pass Criteria
- [ ] ≥15 agents visible in topology
- [ ] Board formed with ≥5 members
- [ ] Deliberation: ≥5 ProposalSubmission + ≥5 CritiqueFeedback
- [ ] IRV: ≥2 elimination rounds
- [ ] Ballots: ≥5 records with critic_scores
- [ ] Task reaches Completed within 25 minutes
- [ ] `/api/holons` returns correct nested structure

---

## Known Limitations / Expected Gaps

| Gap | Reason | Impact |
|-----|---------|--------|
| `CritiqueFeedback` messages may be sparse | ZeroClaw agent doesn't yet send `discussion.critique` P2P messages — critique is tracked via connector's `ConsensusVote` handler, not via explicit critique round | Critique scores may be 0/empty in Phase 2–3; deliberation thread may only have ProposalSubmission + SynthesisResult |
| `adversarial_critic` always non-null | Chair randomly assigns one member on `board.ready` (connector-side) | No LLM behavior change — adversarial prompt not yet wired into zeroclaw agent |
| Sub-holons at depth > 0 | Recursive sub-holon formation in agent script is Task #5 (not yet implemented) | `/api/holons` will only show depth-0 holons |
| SynthesisResult messages | LLM synthesis step is Task #5 | DeliberationPanel won't show SynthesisResult entries |

---

## Implementation: New Test Script

Will be implemented as: `tests/e2e/holonic_swarm_e2e.sh`

Script structure:
- `phase1()` — 1 node smoke test (2–4 min)
- `phase2()` — 5 node full deliberation test (8–15 min)
- `phase3()` — 20 node scale test (20–35 min)
- `validate_holonic_apis()` — shared validation function for all new endpoints
- `--phase N` flag to run individual phases

Invocation:
```bash
# Run all phases
bash tests/e2e/holonic_swarm_e2e.sh

# Run single phase
bash tests/e2e/holonic_swarm_e2e.sh --phase 2
```
