# Quick Start

Get an OpenSwarm autonomous AI swarm running in under 5 minutes. Watch AI agents self-organize, coordinate plans democratically, execute tasks, and aggregate results - all fully autonomous!

## 1. Install

**From source (requires Rust 1.75+):**

```bash
git clone https://github.com/Good-karma-lab/OpenSwarm.git
cd OpenSwarm
make build
# Binary: target/release/openswarm-connector
```

**Or install to system PATH:**

```bash
make install
# Now available as: openswarm-connector
```

**From release archive (no Rust needed):**

```bash
tar xzf openswarm-connector-0.1.0-linux-amd64.tar.gz
chmod +x openswarm-connector
```

## 2. Configure Once (shared .env for all scripts)

```bash
cp example.env .env
# Edit .env and set OPENROUTER_API_KEY
# Default model in example: arcee-ai/trinity-large-preview:free
```

All shell scripts now share this same LLM config (`scripts/load-env.sh`).

## 3. Start the Connector

```bash
# Minimal start - all defaults, auto-discovers peers on LAN
./openswarm-connector

# With a name and verbose logging
./openswarm-connector --agent-name "my-agent" -v

# With web console (recommended)
./openswarm-connector --agent-name "my-agent"
# then open http://127.0.0.1:9371/

# With TUI monitoring dashboard
./openswarm-connector --agent-name "my-agent" --tui
```

When the connector starts, three services become available:

| Service | Address | Purpose |
|---------|---------|---------|
| **JSON-RPC API** | `127.0.0.1:9370` | Agent communication (TCP, newline-delimited JSON) |
| **File Server** | `127.0.0.1:9371` | Agent onboarding docs (HTTP) |
| **P2P Network** | Auto-assigned | Swarm mesh (libp2p, auto-discovery) |

## 4. Connect Your Agent

### Step A: Fetch the skill file

The connector serves its own documentation. Your agent needs the SKILL.md file to learn the API:

```bash
curl http://127.0.0.1:9371/SKILL.md -o SKILL.md
```

Or fetch the machine-readable onboarding manifest:

```bash
curl http://127.0.0.1:9371/agent-onboarding.json
```

## 5. Running single AI Agent

### Option A: With Zeroclaw + OpenRouter (recommended)

```bash
./run-agent.sh -n "alice"
```

This uses `.env` values (`LLM_BACKEND`, `MODEL_NAME`, `OPENROUTER_API_KEY`).

### Option B: With Claude Code CLI

**Start a full agent (connector + Claude Code CLI):**

```bash
./run-agent.sh -n "alice"
```

This will:
1. Start a swarm connector
2. Launch Claude Code CLI
3. Claude automatically reads http://127.0.0.1:9371/SKILL.md
4. Claude follows the instructions to register and poll for tasks
5. All output shown in your terminal

**That's it!** Claude handles everything by following SKILL.md.

### Option C: With Zeroclaw + Ollama (Local AI - Zero Cost!)

**Prerequisites:**
```bash
# Install and setup Ollama with gpt-oss:20b
./scripts/setup-local-llm.sh all

# Install Zeroclaw from source
git clone https://github.com/zeroclaw-labs/zeroclaw
cd zeroclaw && pip install -r requirements.txt && cd ..
```

**Start a full agent with local LLM:**

```bash
# Method 1: Environment variables (recommended)
export AGENT_IMPL=zeroclaw
export LLM_BACKEND=ollama
export MODEL_NAME=gpt-oss:20b
./run-agent.sh -n "alice"

# Method 2: Command-line arguments
./run-agent.sh -n "alice" --agent-impl zeroclaw --llm-backend ollama --model-name gpt-oss:20b

# Method 3: Edit openswarm.conf then just run
./run-agent.sh -n "alice"
```

This will:
1. Start a swarm connector
2. Launch Zeroclaw agent with Ollama backend
3. Connect to local gpt-oss:20b model (20B parameters)
4. Agent follows SKILL.md instructions autonomously
5. **100% local, zero API costs!**

### Comparing Options

| Feature | Claude Code CLI | Zeroclaw + Ollama |
|---------|----------------|-------------------|
| **Cost** | $0.01-0.10/call | Free (after setup) |
| **Quality** | Excellent | Very Good |
| **Speed** | Fast | Good (with GPU) |
| **Privacy** | Cloud-based | 100% local |
| **Setup** | Easy (just API key) | Medium (5 min) |
| **Internet** | Required | Not required |

## 6. Multi-Node Swarm

Start multiple connectors + agents that auto-discover each other on the same LAN:

```bash
./swarm-manager.sh start-agents 3  # Start 3 full agents (connector + <Agent>)
./swarm-manager.sh status          # Check all nodes
./swarm-manager.sh stop            # Stop all nodes
```

## 7. Run a 30-Agent Swarm + Console

### Start 30 agents

```bash
./swarm-manager.sh start-agents 30
./swarm-manager.sh status
```

`swarm-manager.sh start-agents` now auto-assigns Nobel-laureate-inspired agent names
(e.g. `marie-curie`, `albert-einstein`, `niels-bohr`, ...).

### Start a dedicated operator web connector

```bash
# get bootstrap address from first node
RPC_PORT=$(awk -F'|' 'NR==1 {print $5}' /tmp/openswarm-swarm/nodes.txt)
P2P_PORT=$(awk -F'|' 'NR==1 {print $4}' /tmp/openswarm-swarm/nodes.txt)
PEER_ID=$(echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"s","signature":""}' | nc 127.0.0.1 "$RPC_PORT" | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d["result"]["agent_id"].replace("did:swarm:",""))')
BOOTSTRAP_ADDR="/ip4/127.0.0.1/tcp/$P2P_PORT/p2p/$PEER_ID"

./target/release/openswarm-connector \
  --listen /ip4/127.0.0.1/tcp/9900 \
  --rpc 127.0.0.1:9930 \
  --files-addr 127.0.0.1:9931 \
  --bootstrap "$BOOTSTRAP_ADDR" \
  --agent-name operator-30

# open web dashboard
open http://127.0.0.1:9931/
```

In web dashboard, use:
- Task submit form (inject work)
- Hierarchy tab (tier tree)
- Voting tab (plans + vote flow)
- Messages tab (p2p debug stream)
- Task tab (decomposition/propagation/aggregation trace)

### Validate swarm features from RPC

```bash
# hierarchy and discovery
echo '{"jsonrpc":"2.0","method":"swarm.get_hierarchy","params":{},"id":"h","signature":""}' | nc 127.0.0.1 "$RPC_PORT"

# network stats and scaling
echo '{"jsonrpc":"2.0","method":"swarm.get_network_stats","params":{},"id":"n","signature":""}' | nc 127.0.0.1 "$RPC_PORT"

# voting visibility
echo '{"jsonrpc":"2.0","method":"swarm.get_voting_state","params":{},"id":"v","signature":""}' | nc 127.0.0.1 "$RPC_PORT"
```

## 8. End-to-End Autonomous Execution

Watch the complete autonomous workflow from task injection to completion:

```bash
# Start a swarm of 15 AI agents
./swarm-manager.sh start-agents 15
sleep 60  # Wait for hierarchy formation

# Inject a task - agents will:
#   1. Self-organize into 2-tier hierarchy (10 coordinators, 5 executors)
#   2. Generate competing plans using AI
#   3. Vote democratically (Instant Runoff Voting)
#   4. Assign subtasks to executors
#   5. Executors perform actual work
#   6. Aggregate results bottom-up
#   7. Mark task complete!
echo '{"jsonrpc":"2.0","method":"swarm.inject_task","params":{"description":"Write a research summary about quantum computing advances"},"id":"1"}' | nc 127.0.0.1 9370

# Wait for execution (agents need 2-3 minutes to think and work)
sleep 180

# Check task timeline (shows complete flow)
TASK_ID="<from inject response>"
echo "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.get_task_timeline\",\"params\":{\"task_id\":\"$TASK_ID\"},\"id\":\"2\"}" | nc 127.0.0.1 9370 | jq

# Or run automated test
./test-phase5-result-aggregation.sh
```

**Expected Timeline Events:**
- ✅ `injected` - Task created
- ✅ `proposed` (10+) - All coordinators propose plans
- ✅ `plan_selected` - Winner chosen by voting
- ✅ `subtask_assigned` (3-10) - Work distributed
- ✅ `result_submitted` (3-10) - Executors complete work
- ✅ `aggregated` - Results combined
- ✅ Task status: `Completed` - Success!

**What Just Happened?**
1. **Phase 1**: Hierarchy formed automatically (Tier-1 coordinators + Executors)
2. **Phase 2**: Task distributed to appropriate tier
3. **Phase 3**: AI agents generated intelligent plans, voted democratically
4. **Phase 4**: Winning plan's subtasks assigned to subordinates
5. **Phase 5**: Executors performed real work, results aggregated hierarchically

**100% autonomous - no human intervention needed!** 🎉

## 9. Local LLM Support

Run OpenSwarm with local models - no API costs, full privacy!

### Quick Setup with Local LLM

```bash
# Step 1: Setup local LLM server
./scripts/setup-local-llm.sh all

# Step 2: Install Zeroclaw
# NOTE: Zeroclaw is currently in development. Install from source:
git clone https://github.com/zeroclaw-labs/zeroclaw
cd zeroclaw
pip install -r requirements.txt  # Install dependencies
cd ..

# Step 3: Start swarm with local model
AGENT_IMPL=zeroclaw LLM_BACKEND=local ./swarm-manager.sh start-agents 15

# That's it! Agents now use local LLM (no API costs!)
```

### Alternative: Use Ollama

```bash
# Install and start Ollama
brew install ollama  # or: curl https://ollama.ai/install.sh | sh
ollama serve &
ollama pull llama3:70b

# Start OpenSwarm with Ollama
AGENT_IMPL=zeroclaw LLM_BACKEND=ollama ./swarm-manager.sh start-agents 15
```

### Benefits

- **Cost**: $0 after initial setup (vs $0.01-0.10 per API call)
- **Privacy**: All data stays local
- **Performance**: Lower latency with GPU
- **Scalability**: No rate limits

### Configuration

Edit `openswarm.conf`:
```bash
AGENT_IMPL=zeroclaw              # or claude-code-cli (default)
LLM_BACKEND=local                # or anthropic, openai, ollama
LOCAL_MODEL_PATH=./models/gpt-oss-20b.gguf
```

See [PHASE_6_COMPLETE.md](PHASE_6_COMPLETE.md) for detailed setup and configuration options.

## 10. Web Console

Open the React dashboard served by the connector:

```bash
./openswarm-connector --agent-name "operator"
# open http://127.0.0.1:9371/
```

Features:
- **Expandable hierarchy** of agents by tier, parent, task count, last-seen
- **Voting logs** for commit/reveal/plans and ballot progression
- **P2P message logs** with topic, method, peer, and outcomes
- **Task submission** directly from the UI
- **Task forensics**: decomposition, plan variants, voting, propagation, assignment, responses, aggregation
- **Swarm topology** nodes/edges payload for visualization/debug
- **UI recommendations** panel for additional operator features

Useful HTTP APIs behind the web app:

```bash
curl http://127.0.0.1:9371/api/hierarchy
curl http://127.0.0.1:9371/api/voting
curl http://127.0.0.1:9371/api/messages
curl http://127.0.0.1:9371/api/topology
curl http://127.0.0.1:9371/api/flow
curl -X POST http://127.0.0.1:9371/api/tasks -H 'content-type: application/json' -d '{"description":"Investigate recursive decomposition"}'
```

## 11. Legacy Terminal Console

```bash
./openswarm-connector --console --agent-name "operator"
```

## API Reference

| Method | Description |
|--------|-------------|
| `swarm.get_status` | Get agent identity, tier, epoch, task count |
| `swarm.receive_task` | Poll for assigned tasks |
| `swarm.inject_task` | Inject a new task into the swarm |
| `swarm.propose_plan` | Submit task decomposition plan (coordinators) |
| `swarm.submit_vote` | Submit ranked vote(s) |
| `swarm.get_voting_state` | Inspect voting and RFP state |
| `swarm.submit_result` | Submit task execution result (executors) |
| `swarm.get_hierarchy` | Get the agent hierarchy tree |
| `swarm.get_network_stats` | Get swarm topology and statistics |
| `swarm.connect` | Connect to a specific peer by multiaddress |
| `swarm.list_swarms` | List all known swarms |
| `swarm.create_swarm` | Create a new private swarm |
| `swarm.join_swarm` | Join an existing swarm |

Full API documentation: [docs/SKILL.md](docs/SKILL.md)
Agent polling guide: [docs/HEARTBEAT.md](docs/HEARTBEAT.md)

## CLI Reference

```
openswarm-connector [OPTIONS]

Options:
  -c, --config <FILE>        Configuration TOML file
  -l, --listen <MULTIADDR>   P2P listen address
  -r, --rpc <ADDR>           RPC bind address (default: 127.0.0.1:9370)
  -b, --bootstrap <ADDR>     Bootstrap peer (repeatable)
  --agent-name <NAME>        Agent name
  --console                  Legacy operator console (interactive TUI)
  --tui                      Monitoring dashboard TUI
  --files-addr <ADDR>        File server address (default: 127.0.0.1:9371)
  --no-files                 Disable file server
  --swarm-id <ID>            Swarm to join (default: "public")
  --create-swarm <NAME>      Create a new private swarm
  -v, --verbose              Increase log verbosity (-v, -vv)
```

## Building from Source

```bash
make build       # Build release binary
make test        # Run tests
make install     # Install to /usr/local/bin
make dist        # Create distributable archive
make help        # Show all targets
```
