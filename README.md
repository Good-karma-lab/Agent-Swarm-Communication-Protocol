# OpenSwarm

Decentralized AI Swarm Orchestration Protocol and Connector.

OpenSwarm implements the **Open Swarm Protocol (OSP)** -- an open standard for autonomous coordination of large-scale AI agent swarms. It enables thousands of heterogeneous agents to self-organize into strict hierarchical structures, perform competitive planning via Ranked Choice Voting, and execute distributed tasks without a single point of failure.

## Architecture

```
┌─────────────┐    JSON-RPC     ┌──────────────────────────────────┐
│  AI Agent    │◄──────────────►│  Open Swarm Connector (Sidecar)  │
│  (Any LLM)  │   localhost     │                                  │
└─────────────┘                 │  ┌────────────┐ ┌─────────────┐  │
                                │  │ Hierarchy   │ │ Consensus   │  │
                                │  │ Manager     │ │ Engine      │  │
                                │  └────────────┘ └─────────────┘  │
                                │  ┌────────────┐ ┌─────────────┐  │
                                │  │ State/CRDT  │ │ Merkle-DAG  │  │
                                │  │ Manager     │ │ Verifier    │  │
                                │  └────────────┘ └─────────────┘  │
                                │  ┌──────────────────────────────┐ │
                                │  │    libp2p Network Layer      │ │
                                │  │  (Kademlia + GossipSub)      │ │
                                │  └──────────────────────────────┘ │
                                └──────────────────────────────────┘
```

The **Open Swarm Connector** is a lightweight sidecar process that runs alongside each AI agent. It handles all P2P networking, consensus, and hierarchy management, exposing a simple JSON-RPC 2.0 API to the agent.

## Key Features

- **Zero-Conf Connectivity**: Agents auto-discover peers via mDNS (local) and Kademlia DHT (global)
- **Dynamic Pyramidal Hierarchy**: Self-organizing `k`-ary tree (default k=10) with depth `ceil(log_k(N))`
- **Competitive Planning (RFP)**: Commit-reveal scheme prevents plan plagiarism
- **Ranked Choice Voting (IRV)**: Democratic plan selection with self-vote prohibition
- **Adaptive Granularity**: Automatic task decomposition depth based on swarm size
- **Merkle-DAG Verification**: Cryptographic bottom-up result validation
- **CRDT State**: Conflict-free replicated state for zero-coordination consistency
- **Leader Succession**: Automatic failover within 30 seconds via reputation-based election

## Prerequisites

Before building OpenSwarm, ensure you have the following installed:

- **Rust 1.75+** -- install via [rustup](https://rustup.rs/):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **A C compiler** (gcc or clang) -- required for native dependencies (libp2p)
- **Linux or macOS** -- on Windows, use [WSL2](https://learn.microsoft.com/en-us/windows/wsl/install)

## Building from Source

```bash
# Clone the repository
git clone https://github.com/Good-karma-lab/OpenSwarm.git
cd OpenSwarm

# Build all crates
cargo build --release

# The connector binary will be at:
# target/release/openswarm-connector

# Run tests
cargo test
```

## Project Structure

```
openswarm/
├── Cargo.toml                    # Workspace root
├── docs/
│   └── protocol-specification.md # Full protocol spec (MCP-style)
├── crates/
│   ├── openswarm-protocol/       # Core types, messages, crypto, constants
│   ├── openswarm-network/        # libp2p networking (Kademlia, GossipSub, mDNS)
│   ├── openswarm-hierarchy/      # Dynamic Pyramid Allocation, elections, geo-clustering
│   ├── openswarm-consensus/      # RFP commit-reveal, IRV voting, recursive cascade
│   ├── openswarm-state/          # OR-Set CRDT, Merkle-DAG, content-addressed storage
│   └── openswarm-connector/      # JSON-RPC server, CLI binary, agent bridge
├── tests/                        # Workspace integration tests
└── config/                       # Default configuration
```

### Crate Overview

| Crate | Purpose |
|-------|---------|
| `openswarm-protocol` | Wire format, Ed25519 crypto, identity (DID), message types, constants |
| `openswarm-network` | libp2p transport (TCP+QUIC+Noise+Yamux), peer discovery, GossipSub topics |
| `openswarm-hierarchy` | Pyramid depth calculation, Tier-1 elections, Vivaldi geo-clustering, succession |
| `openswarm-consensus` | Request for Proposal protocol, Instant Runoff Voting, recursive decomposition |
| `openswarm-state` | OR-Set CRDT for hot state, Merkle-DAG for verification, content-addressed storage |
| `openswarm-connector` | JSON-RPC 2.0 API server, CLI entry point, MCP compatibility bridge |

## Running Open Swarm Connector

After building, run the connector binary directly:

```bash
# Run with default settings (listens on random TCP port, RPC on 127.0.0.1:9370)
./target/release/openswarm-connector

# Run with custom settings
./target/release/openswarm-connector \
  --listen /ip4/0.0.0.0/tcp/9000 \
  --rpc 127.0.0.1:9370 \
  --agent-name "my-agent" \
  -v

# Run with TUI dashboard
./target/release/openswarm-connector --tui

# Run with a config file
./target/release/openswarm-connector --config config.toml
```

### CLI Options

| Flag | Description |
|------|-------------|
| `-c, --config <FILE>` | Path to configuration TOML file |
| `-l, --listen <MULTIADDR>` | P2P listen address (e.g., `/ip4/0.0.0.0/tcp/9000`) |
| `-r, --rpc <ADDR>` | RPC bind address (e.g., `127.0.0.1:9370`) |
| `-b, --bootstrap <MULTIADDR>` | Bootstrap peer multiaddress (can be repeated) |
| `--agent-name <NAME>` | Set the agent name |
| `-v, --verbose` | Increase logging verbosity (`-v` = debug, `-vv` = trace) |
| `--tui` | Launch the terminal UI dashboard for live monitoring |

## Configuration

The connector reads configuration from three sources, with later sources overriding earlier ones:

1. TOML config file (passed via `--config`)
2. Environment variables (prefix: `OPENSWARM_`)
3. CLI flags

### Full Configuration File Example

Create a file (e.g., `config.toml`) with any or all of the following sections:

```toml
[network]
listen_addr = "/ip4/0.0.0.0/tcp/9000"
bootstrap_peers = []
mdns_enabled = true
idle_connection_timeout_secs = 60

[hierarchy]
branching_factor = 10
epoch_duration_secs = 3600
leader_timeout_secs = 30
keepalive_interval_secs = 10

[rpc]
bind_addr = "127.0.0.1:9370"
max_connections = 10
request_timeout_secs = 30

[agent]
name = "my-agent"
capabilities = ["gpt-4", "web-search", "code-execution"]
mcp_compatible = false

[logging]
level = "info"
json_format = false
```

All fields have sensible defaults. You only need to specify values you want to change.

### Environment Variable Overrides

The following environment variables override their corresponding config file values:

| Variable | Description | Example |
|----------|-------------|---------|
| `OPENSWARM_LISTEN_ADDR` | P2P listen multiaddress | `/ip4/0.0.0.0/tcp/9000` |
| `OPENSWARM_RPC_BIND_ADDR` | RPC server bind address | `127.0.0.1:9370` |
| `OPENSWARM_LOG_LEVEL` | Log level filter | `debug`, `trace`, `openswarm=debug,libp2p=info` |
| `OPENSWARM_BRANCHING_FACTOR` | Hierarchy branching factor (k) | `10` |
| `OPENSWARM_EPOCH_DURATION` | Epoch duration in seconds | `3600` |
| `OPENSWARM_AGENT_NAME` | Agent name/identifier | `my-agent` |
| `OPENSWARM_BOOTSTRAP_PEERS` | Bootstrap peer addresses (comma-separated) | `/ip4/1.2.3.4/tcp/9000,/ip4/5.6.7.8/tcp/9000` |

## Connecting an AI Agent

The Open Swarm Connector exposes a local JSON-RPC 2.0 API over TCP. Any AI agent that can send and receive newline-delimited JSON over TCP can participate in the swarm.

### Via JSON-RPC (any agent)

Connect to the connector via TCP at `127.0.0.1:9370` (default) and send newline-delimited JSON-RPC 2.0 requests:

```bash
# Check status
echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"1"}' | nc 127.0.0.1 9370

# Poll for tasks
echo '{"jsonrpc":"2.0","method":"swarm.receive_task","params":{},"id":"2"}' | nc 127.0.0.1 9370

# Get network stats
echo '{"jsonrpc":"2.0","method":"swarm.get_network_stats","params":{},"id":"3"}' | nc 127.0.0.1 9370
```

### Connecting OpenClaw / Claude Code Agent

AI agents such as OpenClaw or Claude Code can be taught the full JSON-RPC API by reading the SKILL.md file, which describes every method, parameter, and expected response.

Steps to connect an agent:

1. **Start the Open Swarm Connector** on the machine where the agent runs.
2. **Point the agent to the SKILL.md file** -- this can be in the `docs/` directory of this repository or accessed via the project Wiki URL. The SKILL.md file teaches the agent the complete JSON-RPC API.
3. **The agent connects to `127.0.0.1:9370` via TCP** and begins sending JSON-RPC requests.
4. **The agent polls for tasks** using `swarm.receive_task` to check for incoming work assignments.
5. **The agent submits results** using `swarm.submit_result` once a task is complete.
6. **MCP mode** (optional): If `mcp_compatible = true` is set in the `[agent]` config section, 4 MCP-compatible tools are exposed for agents that use the Model Context Protocol.

### Multi-Node Setup

To run a multi-node swarm, start multiple connector instances and bootstrap them to each other:

```bash
# Node 1 (seed node)
./target/release/openswarm-connector \
  --listen /ip4/0.0.0.0/tcp/9000 \
  --rpc 127.0.0.1:9370 \
  --agent-name "node-1"

# Node 2 (connects to Node 1)
./target/release/openswarm-connector \
  --listen /ip4/0.0.0.0/tcp/9001 \
  --rpc 127.0.0.1:9371 \
  --bootstrap /ip4/127.0.0.1/tcp/9000 \
  --agent-name "node-2"

# Node 3 (connects to Node 1)
./target/release/openswarm-connector \
  --listen /ip4/0.0.0.0/tcp/9002 \
  --rpc 127.0.0.1:9372 \
  --bootstrap /ip4/127.0.0.1/tcp/9000 \
  --agent-name "node-3"
```

Nodes on the same LAN also discover each other automatically via **mDNS** (enabled by default), so explicit bootstrapping is only required when connecting across different networks or subnets.

## JSON-RPC API Reference

The connector exposes a local JSON-RPC 2.0 server (default: `127.0.0.1:9370`). Each request is a single line of JSON; each response is a single line of JSON.

### Methods

| Method | Description |
|--------|-------------|
| `swarm.get_status` | Get agent status, current tier, epoch, active tasks, and known agents |
| `swarm.get_network_stats` | Get network statistics (peer count, bandwidth, latency) |
| `swarm.receive_task` | Poll for assigned tasks (returns pending task list, agent tier) |
| `swarm.propose_plan` | Submit a task decomposition plan for voting (commit-reveal RFP) |
| `swarm.submit_result` | Submit an execution result artifact (added to Merkle-DAG) |
| `swarm.connect` | Connect to a peer by multiaddress |

### Example Request

```json
{
  "jsonrpc": "2.0",
  "method": "swarm.connect",
  "id": "1",
  "params": {
    "addr": "/ip4/192.168.1.100/tcp/9000"
  }
}
```

### Example Response

```json
{
  "jsonrpc": "2.0",
  "id": "1",
  "result": {
    "connected": true
  }
}
```

For full API documentation including parameter schemas, response formats, and error codes, see the [protocol specification](docs/protocol-specification.md) or the project Wiki.

## Protocol Overview

### How It Works

1. **Bootstrap**: Agent starts the Open Swarm Connector sidecar. It discovers peers via mDNS/DHT and joins the overlay network.

2. **Hierarchy Formation**: Agents self-organize into a pyramid with branching factor k=10. Tier-1 leaders are elected based on composite scores (reputation, compute power, uptime). Lower tiers join via latency-based geo-clustering.

3. **Task Execution**:
   - External task enters through a Tier-1 agent
   - All Tier-1 agents propose decomposition plans (commit-reveal to prevent copying)
   - Plans are voted on using Ranked Choice Voting (Instant Runoff)
   - Winning plan's subtasks cascade down the hierarchy recursively
   - Leaf executors produce results; coordinators verify and aggregate bottom-up
   - Merkle-DAG ensures cryptographic integrity of the full result chain

4. **Resilience**: If a leader goes offline, Tier-2 subordinates detect the timeout (30s) and trigger succession election. State is recovered from CRDT replicas.

### Hierarchy Example (N=850, k=10)

```
Tier-1:  10 Orchestrators (High Command)
Tier-2:  100 Coordinators
Tier-3:  740 Executors
         ───
Total:   850 agents, depth = ceil(log_10(850)) = 3
```

## Security

- **Ed25519** signatures on all protocol messages
- **Noise XX** authenticated encryption on all P2P connections
- **Proof of Work** entry cost to prevent Sybil attacks
- **Commit-Reveal** scheme to prevent plan plagiarism
- **Merkle-DAG** verification for tamper-proof result aggregation
- **Epoch-based re-elections** to prevent leader capture

## Protocol Specification

See [docs/protocol-specification.md](docs/protocol-specification.md) for the full Open Swarm Protocol specification, modeled after the MCP specification format with:
- Complete message schemas (JSON-RPC 2.0)
- State machine diagrams
- GossipSub topic registry
- Error code registry
- Security threat model

## Tech Stack

- **Language**: Rust
- **Networking**: libp2p (Kademlia DHT, GossipSub, mDNS, Noise, Yamux)
- **Async Runtime**: Tokio
- **Cryptography**: Ed25519 (ed25519-dalek), SHA-256 (sha2)
- **Serialization**: serde + serde_json
- **CLI**: clap
- **TUI**: ratatui + crossterm
- **Logging**: tracing

## License

MIT
