#!/bin/bash

# OpenSwarm Multi-Node Manager
# Start, stop, and manage multiple connector instances

set -e

# Add cargo to PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SWARM_DIR="/tmp/openswarm-swarm"
NODES_FILE="$SWARM_DIR/nodes.txt"

usage() {
    cat << EOF
${GREEN}OpenSwarm Multi-Node Manager${NC}

Usage: $0 <command> [options]

Commands:
    start <N>           Start N nodes (default: 3)
    stop                Stop all running nodes
    status              Show status of all nodes
    test                Run a quick API test on all nodes
    clean               Clean up all temporary files
    help                Show this help message

Examples:
    # Start 3 nodes
    $0 start 3

    # Check status
    $0 status

    # Test all nodes
    $0 test

    # Stop all nodes
    $0 stop

EOF
    exit 0
}

# Initialize swarm directory
init_swarm_dir() {
    mkdir -p "$SWARM_DIR"
    if [ ! -f "$NODES_FILE" ]; then
        touch "$NODES_FILE"
    fi
}

# Start N nodes
start_nodes() {
    local num_nodes=${1:-3}

    echo -e "${GREEN}Starting $num_nodes OpenSwarm nodes...${NC}"
    echo ""

    init_swarm_dir

    # Clear existing nodes file
    > "$NODES_FILE"

    local bootstrap_addr=""

    for i in $(seq 1 $num_nodes); do
        local node_name="swarm-node-$i"
        local log_file="$SWARM_DIR/$node_name.log"

        echo -e "${BLUE}Starting node $i/$num_nodes: $node_name${NC}"

        # Find available ports
        local p2p_port=$(find_available_port $((9000 + i - 1)))
        local rpc_port=$(find_available_port $((9370 + i - 1)))

        # Build command
        local cmd="./target/release/openswarm-connector"
        cmd="$cmd --listen /ip4/0.0.0.0/tcp/$p2p_port"
        cmd="$cmd --rpc 127.0.0.1:$rpc_port"
        cmd="$cmd --agent-name $node_name"

        # Add bootstrap peer if not the first node
        if [ -n "$bootstrap_addr" ]; then
            cmd="$cmd --bootstrap $bootstrap_addr"
        fi

        # Start the node in background
        eval "$cmd > $log_file 2>&1 &"
        local pid=$!

        # Save node info
        echo "$node_name|$pid|$p2p_port|$rpc_port" >> "$NODES_FILE"

        # Wait for node to start
        sleep 2

        # Get peer ID for bootstrap
        if [ $i -eq 1 ]; then
            local peer_id=$(get_peer_id $rpc_port)
            if [ -n "$peer_id" ]; then
                bootstrap_addr="/ip4/127.0.0.1/tcp/$p2p_port/p2p/$peer_id"
                echo -e "  ${GREEN}✓${NC} Bootstrap node ready: $bootstrap_addr"
            fi
        else
            echo -e "  ${GREEN}✓${NC} Node started (PID: $pid, RPC: $rpc_port)"
        fi
    done

    echo ""
    echo -e "${GREEN}All $num_nodes nodes started successfully!${NC}"
    echo ""
    echo -e "${YELLOW}Node information saved to: $NODES_FILE${NC}"
    echo -e "${YELLOW}View status with: $0 status${NC}"
    echo ""
}

# Find available port
find_available_port() {
    local start_port=$1
    local port=$start_port
    while true; do
        if ! lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
            echo $port
            return
        fi
        port=$((port + 1))
    done
}

# Get peer ID from RPC
get_peer_id() {
    local rpc_port=$1
    echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"1","signature":""}' | \
        nc 127.0.0.1 $rpc_port 2>/dev/null | \
        grep -o 'did:swarm:[^"]*' | \
        sed 's/did:swarm://' | \
        head -1
}

# Stop all nodes
stop_nodes() {
    init_swarm_dir

    if [ ! -s "$NODES_FILE" ]; then
        echo -e "${YELLOW}No nodes are currently running.${NC}"
        return
    fi

    echo -e "${YELLOW}Stopping all OpenSwarm nodes...${NC}"
    echo ""

    while IFS='|' read -r name pid p2p_port rpc_port; do
        if ps -p $pid > /dev/null 2>&1; then
            echo -e "  Stopping $name (PID: $pid)..."
            kill $pid 2>/dev/null || true
        fi
    done < "$NODES_FILE"

    # Wait for processes to terminate
    sleep 1

    echo ""
    echo -e "${GREEN}All nodes stopped.${NC}"

    # Clear nodes file
    > "$NODES_FILE"
}

# Show status of all nodes
show_status() {
    init_swarm_dir

    if [ ! -s "$NODES_FILE" ]; then
        echo -e "${YELLOW}No nodes are currently running.${NC}"
        return
    fi

    echo -e "${GREEN}╔════════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                    OpenSwarm Nodes Status                              ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    printf "%-20s %-8s %-10s %-10s %-10s %-15s\n" "NODE NAME" "PID" "P2P PORT" "RPC PORT" "STATUS" "PEERS"
    printf "%-20s %-8s %-10s %-10s %-10s %-15s\n" "────────────────────" "────────" "──────────" "──────────" "──────────" "───────────────"

    while IFS='|' read -r name pid p2p_port rpc_port; do
        local status="STOPPED"
        local peers="N/A"

        if ps -p $pid > /dev/null 2>&1; then
            status="${GREEN}RUNNING${NC}"

            # Get network stats
            local stats=$(echo '{"jsonrpc":"2.0","method":"swarm.get_network_stats","params":{},"id":"1","signature":""}' | nc 127.0.0.1 $rpc_port 2>/dev/null || echo "")

            if [ -n "$stats" ]; then
                peers=$(echo "$stats" | grep -o '"total_agents":[0-9]*' | cut -d':' -f2)
            fi
        fi

        printf "%-20s %-8s %-10s %-10s %-10b %-15s\n" "$name" "$pid" "$p2p_port" "$rpc_port" "$status" "$peers"
    done < "$NODES_FILE"

    echo ""
}

# Test all nodes
test_nodes() {
    init_swarm_dir

    if [ ! -s "$NODES_FILE" ]; then
        echo -e "${YELLOW}No nodes are currently running.${NC}"
        return
    fi

    echo -e "${GREEN}Testing all OpenSwarm nodes...${NC}"
    echo ""

    while IFS='|' read -r name pid p2p_port rpc_port; do
        echo -e "${BLUE}Testing $name (RPC: $rpc_port)${NC}"

        if ! ps -p $pid > /dev/null 2>&1; then
            echo -e "  ${RED}✗ Node is not running${NC}"
            echo ""
            continue
        fi

        # Test get_status
        local response=$(echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"1","signature":""}' | nc 127.0.0.1 $rpc_port 2>/dev/null)

        if echo "$response" | grep -q "result"; then
            local agent_id=$(echo "$response" | grep -o 'did:swarm:[^"]*' | head -1)
            local tier=$(echo "$response" | grep -o '"tier":"[^"]*"' | cut -d':' -f2 | tr -d '"')
            local epoch=$(echo "$response" | grep -o '"epoch":[0-9]*' | cut -d':' -f2)

            echo -e "  ${GREEN}✓ API responding${NC}"
            echo -e "    Agent ID: $agent_id"
            echo -e "    Tier: $tier"
            echo -e "    Epoch: $epoch"
        else
            echo -e "  ${RED}✗ API not responding${NC}"
        fi

        echo ""
    done
}

# Clean up
clean_up() {
    echo -e "${YELLOW}Cleaning up OpenSwarm temporary files...${NC}"

    # Stop nodes first
    stop_nodes

    # Remove swarm directory
    if [ -d "$SWARM_DIR" ]; then
        rm -rf "$SWARM_DIR"
        echo -e "${GREEN}Temporary files cleaned.${NC}"
    fi

    # Remove other temp files
    rm -f /tmp/openswarm-*.pid
    rm -f /tmp/openswarm-*-info.txt
    rm -f /tmp/node*.log

    echo -e "${GREEN}Cleanup complete.${NC}"
}

# Main command dispatcher
case "${1:-help}" in
    start)
        start_nodes ${2:-3}
        ;;
    stop)
        stop_nodes
        ;;
    status)
        show_status
        ;;
    test)
        test_nodes
        ;;
    clean)
        clean_up
        ;;
    help|--help|-h)
        usage
        ;;
    *)
        echo -e "${RED}Unknown command: $1${NC}"
        echo ""
        usage
        ;;
esac
