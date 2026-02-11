# OpenSwarm Quick Start Guide

## Overview

OpenSwarm is now ready to use! I've created helper scripts to make testing easy.

## What's Available

### 1. `run-node.sh` - Single Node Launcher

Start individual connector instances with automatic port selection.

**Basic usage:**
```bash
./run-node.sh -n "my-agent"
```

**Features:**
- ✅ TUI dashboard enabled by default
- ✅ Automatic port selection (no conflicts)
- ✅ Displays connection info and peer ID
- ✅ Saves multiaddress for easy sharing
- ✅ Supports bootstrap connections
- ✅ Verbose logging options

**Examples:**
```bash
# Start a standalone node (with TUI by default)
./run-node.sh -n "alice"

# Connect to existing node
./run-node.sh -n "bob" -b "/ip4/127.0.0.1/tcp/9000/p2p/12D3Koo..."

# Start without TUI dashboard
./run-node.sh -n "charlie" --no-tui

# Start with verbose logging
./run-node.sh -n "dave" -vv

# Join a private swarm
./run-node.sh -n "eve" -s "my-private-swarm"
```

### 2. `swarm-manager.sh` - Multi-Node Manager

Manage multiple nodes at once for testing swarm behavior.

**Basic usage:**
```bash
# Start 3 nodes
./swarm-manager.sh start 3

# Check status
./swarm-manager.sh status

# Test all nodes
./swarm-manager.sh test

# Stop all nodes
./swarm-manager.sh stop

# Clean up everything
./swarm-manager.sh clean
```

**Features:**
- ✅ Start N nodes automatically
- ✅ Automatic bootstrap chain setup
- ✅ Status monitoring for all nodes
- ✅ API testing for all nodes
- ✅ Easy cleanup

## Quick Test - Single Node

```bash
# Start a node
./run-node.sh -n "test-node"

# In another terminal, test the API
echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"1","signature":""}' | nc 127.0.0.1 9370
```

Expected output:
```json
{
  "jsonrpc": "2.0",
  "id": "1",
  "result": {
    "agent_id": "did:swarm:12D3Koo...",
    "status": "Running",
    "tier": "Executor",
    "epoch": 1,
    "active_tasks": 0,
    "known_agents": 0,
    "content_items": 0
  }
}
```

## Quick Test - Multi-Node Swarm

```bash
# Terminal 1: Start 5 nodes
./swarm-manager.sh start 5

# Terminal 2: Check status
./swarm-manager.sh status
```

You should see output like:
```
NODE NAME            PID      P2P PORT   RPC PORT   STATUS     PEERS
────────────────────────────────────────────────────────────────────
swarm-node-1         12345    9000       9370       RUNNING    5
swarm-node-2         12346    9001       9371       RUNNING    5
swarm-node-3         12347    9002       9372       RUNNING    5
swarm-node-4         12348    9003       9373       RUNNING    5
swarm-node-5         12349    9004       9374       RUNNING    5
```

## Testing the API

### Available Methods

| Method | Description |
|--------|-------------|
| `swarm.get_status` | Get agent identity, tier, and status |
| `swarm.get_network_stats` | Get swarm topology and statistics |
| `swarm.receive_task` | Poll for assigned tasks |
| `swarm.propose_plan` | Submit task decomposition plan |
| `swarm.submit_result` | Submit task execution result |
| `swarm.connect` | Connect to a specific peer |

### Example API Calls

**Get status:**
```bash
echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"1","signature":""}' | nc 127.0.0.1 9370
```

**Get network stats:**
```bash
echo '{"jsonrpc":"2.0","method":"swarm.get_network_stats","params":{},"id":"2","signature":""}' | nc 127.0.0.1 9370
```

**Poll for tasks:**
```bash
echo '{"jsonrpc":"2.0","method":"swarm.receive_task","params":{},"id":"3","signature":""}' | nc 127.0.0.1 9370
```

**Connect to peer:**
```bash
echo '{"jsonrpc":"2.0","method":"swarm.connect","params":{"addr":"/ip4/127.0.0.1/tcp/9001/p2p/12D3Koo..."},"id":"4","signature":""}' | nc 127.0.0.1 9370
```

## Python Client Example

Create a file `test_client.py`:

```python
#!/usr/bin/env python3
import socket
import json
import sys

def call_rpc(method, params={}, rpc_port=9370):
    """Make a JSON-RPC call to the OpenSwarm connector"""
    request = {
        "jsonrpc": "2.0",
        "id": "1",
        "method": method,
        "params": params,
        "signature": ""
    }

    try:
        sock = socket.create_connection(("127.0.0.1", rpc_port), timeout=5)
        sock.sendall((json.dumps(request) + "\n").encode())
        response = sock.makefile().readline()
        sock.close()
        return json.loads(response)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        return None

# Example usage
if __name__ == "__main__":
    print("=== Swarm Status ===")
    status = call_rpc("swarm.get_status")
    if status:
        result = status.get("result", {})
        print(f"Agent ID: {result.get('agent_id')}")
        print(f"Status: {result.get('status')}")
        print(f"Tier: {result.get('tier')}")
        print(f"Epoch: {result.get('epoch')}")
        print(f"Active Tasks: {result.get('active_tasks')}")

    print("\n=== Network Stats ===")
    stats = call_rpc("swarm.get_network_stats")
    if stats:
        result = stats.get("result", {})
        print(f"Total Agents: {result.get('total_agents')}")
        print(f"Hierarchy Depth: {result.get('hierarchy_depth')}")
        print(f"Branching Factor: {result.get('branching_factor')}")
        print(f"Current Epoch: {result.get('current_epoch')}")
        print(f"My Tier: {result.get('my_tier')}")
```

Run it:
```bash
chmod +x test_client.py
./test_client.py
```

## What You Should See

### Successful Startup

When a node starts successfully, you'll see:
```
╔════════════════════════════════════════════════════════════╗
║         OpenSwarm Connector Instance Starting...          ║
╚════════════════════════════════════════════════════════════╝

Agent Name:     my-agent
Swarm ID:       public
P2P Port:       9000
RPC Port:       9370

Connection Information:
  JSON-RPC API:  tcp://127.0.0.1:9370

✓ Connector started successfully!

Your Peer ID: 12D3KooWABC123...

Full Multiaddress:
  /ip4/192.168.1.46/tcp/9000/p2p/12D3KooWABC123...
  /ip4/127.0.0.1/tcp/9000/p2p/12D3KooWABC123...
```

### Multi-Node Discovery

When nodes discover each other via mDNS:
```
[INFO] mDNS discovered new peer peer=12D3Koo... addr=/ip4/192.168.1.46/tcp/9001/...
[INFO] Connection established peer=12D3Koo...
[INFO] Kademlia bootstrap complete
```

## Troubleshooting

### Port already in use

The scripts automatically find available ports, but if you see this error when running manually:
```
Error: Address already in use
```

Solution: Use different ports or let the script choose automatically:
```bash
./run-node.sh -n "my-node"  # Auto-selects ports
```

### Cannot connect to RPC

Check if the node is running:
```bash
ps aux | grep openswarm-connector
```

Check what ports are in use:
```bash
lsof -i :9370-9380
```

### Nodes not discovering each other

Make sure:
1. Nodes are on the same network
2. mDNS is enabled (default)
3. If using bootstrap, the multiaddress includes the peer ID
4. Firewall allows the P2P ports

## Next Steps

1. ✅ **You can now test the connector** - Use the scripts above
2. 📖 **Read the full API docs** - See [docs/SKILL.md](docs/SKILL.md)
3. 🤖 **Connect an AI agent** - Implement a client that uses the JSON-RPC API
4. 🧪 **Test consensus** - Try submitting tasks and plans
5. 📚 **Learn the protocol** - Read [docs/Protocol-Specification.md](docs/Protocol-Specification.md)

## Files Reference

| File | Purpose |
|------|---------|
| `run-node.sh` | Start single connector instances |
| `swarm-manager.sh` | Manage multiple nodes |
| `TESTING.md` | Comprehensive testing guide |
| `QUICKSTART.md` | This file |
| `README.md` | Full project documentation |
| `docs/SKILL.md` | Complete JSON-RPC API reference |
| `docs/HEARTBEAT.md` | Agent polling loop guide |
| `docs/Protocol-Specification.md` | Full protocol spec |

## Support

- **Documentation**: See the `docs/` directory
- **Examples**: See `TESTING.md` for more examples
- **Issues**: Report bugs on GitHub

---

**Happy Testing! 🎉**

The OpenSwarm connector is ready to orchestrate decentralized AI agent swarms.
