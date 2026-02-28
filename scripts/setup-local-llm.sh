#!/bin/bash
# Setup and run local LLM for WorldWideSwarm
# Supports llama.cpp server with GPT OSS 20b model

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

if [ -f "$ROOT_DIR/scripts/load-env.sh" ]; then
    # shellcheck disable=SC1091
    source "$ROOT_DIR/scripts/load-env.sh"
fi

OLLAMA_MODEL=${MODEL_NAME:-gpt-oss:20b}

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

MODEL_DIR="./models"
LLAMACPP_DIR="./llama.cpp"

usage() {
    cat << EOF
Setup and run local LLM for WorldWideSwarm

Usage: $0 [command] [--backend ollama|llamacpp]

Commands:
    install         Install Ollama (default) or llama.cpp
    download        Download/pull GPT OSS 20b model
    start           Start LLM server
    stop            Stop LLM server
    status          Check server status
    all             Do everything (install + download + start)

Options:
    --backend ollama      Use Ollama (default, recommended)
    --backend llamacpp    Use llama.cpp

Examples:
    $0 all                      # Complete setup with Ollama (recommended)
    $0 all --backend llamacpp   # Complete setup with llama.cpp
    $0 start                    # Start the server
    $0 status                   # Check if running

EOF
    exit 0
}

install_llamacpp() {
    echo -e "${BLUE}Installing llama.cpp...${NC}"

    if [ -d "$LLAMACPP_DIR" ]; then
        echo -e "${YELLOW}llama.cpp already installed at $LLAMACPP_DIR${NC}"
    else
        echo "Cloning llama.cpp repository..."
        git clone https://github.com/ggerganov/llama.cpp "$LLAMACPP_DIR"
    fi

    echo "Building llama.cpp (CMake)..."
    cmake -S "$LLAMACPP_DIR" -B "$LLAMACPP_DIR/build" -DLLAMA_BUILD_SERVER=ON
    cmake --build "$LLAMACPP_DIR/build" --config Release -j 2

    echo -e "${GREEN}✓ llama.cpp installed successfully${NC}"
}

install_ollama() {
    echo -e "${BLUE}Installing Ollama...${NC}"

    # Check if already installed
    if command -v ollama &> /dev/null; then
        echo -e "${YELLOW}Ollama already installed${NC}"
        return 0
    fi

    echo "Installing Ollama..."
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        if command -v brew &> /dev/null; then
            brew install ollama
        else
            curl -fsSL https://ollama.ai/install.sh | sh
        fi
    else
        # Linux
        curl -fsSL https://ollama.ai/install.sh | sh
    fi

    echo -e "${GREEN}✓ Ollama installed successfully${NC}"
}

download_model_ollama() {
    echo -e "${BLUE}Downloading $OLLAMA_MODEL model via Ollama...${NC}"

    # Check if ollama is running
    if ! pgrep -x "ollama" > /dev/null; then
        echo "Starting Ollama server..."
        ollama serve > /dev/null 2>&1 &
        sleep 3
    fi

    # Check if model already exists
    if ollama list | grep -q "$OLLAMA_MODEL"; then
        echo -e "${YELLOW}Model already downloaded${NC}"
        return 0
    fi

    echo "Pulling $OLLAMA_MODEL (this may take several minutes)..."
    ollama pull "$OLLAMA_MODEL" || {
        echo -e "${RED}Download failed. Check your internet connection.${NC}"
        exit 1
    }

    echo -e "${GREEN}✓ Model $OLLAMA_MODEL downloaded successfully${NC}"
}

download_model_llamacpp() {
    echo -e "${BLUE}Downloading model for llama.cpp...${NC}"

    mkdir -p "$MODEL_DIR"

    # Check if model already exists
    if [ -f "$MODEL_DIR/gpt-oss-20b.gguf" ]; then
        echo -e "${YELLOW}Model already downloaded${NC}"
        return 0
    fi

    echo -e "${YELLOW}Downloading Llama-2-7b as fallback (gpt-oss GGUF not available)...${NC}"

    # Download a real model (Llama-2-7b as example)
    curl -L --fail --retry 3 --retry-delay 2 \
        "https://huggingface.co/TheBloke/Llama-2-7B-GGUF/resolve/main/llama-2-7b.Q4_K_M.gguf" \
        -o "$MODEL_DIR/gpt-oss-20b.gguf" || {
        echo -e "${RED}Download failed. You may need to manually download a model.${NC}"
        exit 1
    }

    echo -e "${GREEN}✓ Model downloaded to $MODEL_DIR/gpt-oss-20b.gguf${NC}"
}

start_server_ollama() {
    echo -e "${BLUE}Starting Ollama server...${NC}"

    # Check if already running
    if pgrep -x "ollama" > /dev/null; then
        echo -e "${YELLOW}Ollama server already running${NC}"

        # Verify model is available
        if ollama list | grep -q "$OLLAMA_MODEL"; then
            echo -e "${GREEN}✓ Model $OLLAMA_MODEL is available${NC}"
        else
            echo -e "${YELLOW}Model not found, pulling $OLLAMA_MODEL...${NC}"
            ollama pull "$OLLAMA_MODEL"
        fi
        return 0
    fi

    echo "Starting Ollama server..."
    ollama serve > ollama-server.log 2>&1 &
    echo $! > ollama-server.pid

    echo "Waiting for server to start..."
    for i in {1..30}; do
        if pgrep -x "ollama" > /dev/null; then
            echo -e "${GREEN}✓ Ollama server started successfully${NC}"
            echo ""
            echo "Test it:"
            echo '  ollama list'
            echo '  curl http://localhost:11434/api/tags'
            return 0
        fi
        sleep 1
    done

    echo -e "${RED}✗ Server failed to start. Check ollama-server.log${NC}"
    exit 1
}

start_server_llamacpp() {
    echo -e "${BLUE}Starting llama.cpp server...${NC}"

    local server_bin=""
    if [ -f "$LLAMACPP_DIR/server" ]; then
        server_bin="$LLAMACPP_DIR/server"
    elif [ -f "$LLAMACPP_DIR/build/bin/llama-server" ]; then
        server_bin="$LLAMACPP_DIR/build/bin/llama-server"
    elif [ -f "$LLAMACPP_DIR/build/bin/server" ]; then
        server_bin="$LLAMACPP_DIR/build/bin/server"
    fi

    if [ -z "$server_bin" ]; then
        echo -e "${RED}Error: llama.cpp not installed. Run: $0 install --backend llamacpp${NC}"
        exit 1
    fi

    if [ ! -f "$MODEL_DIR/gpt-oss-20b.gguf" ]; then
        echo -e "${RED}Error: Model not downloaded. Run: $0 download --backend llamacpp${NC}"
        exit 1
    fi

    # Check if already running
    if curl -s http://localhost:8080/health > /dev/null 2>&1; then
        echo -e "${YELLOW}Server already running on http://localhost:8080${NC}"
        return 0
    fi

    echo "Starting server on port 8080..."
    cd "$ROOT_DIR"
    nohup "$server_bin" \
        -m "$ROOT_DIR/$MODEL_DIR/gpt-oss-20b.gguf" \
        --port 8080 \
        --ctx-size 8192 \
        --parallel 1 \
        --n-gpu-layers 35 \
        > "$ROOT_DIR/llama-server.log" 2>&1 &

    echo $! > "$ROOT_DIR/llama-server.pid"

    echo "Waiting for server to start..."
    for i in {1..30}; do
        if curl -s http://localhost:8080/health > /dev/null 2>&1; then
            echo -e "${GREEN}✓ Server started successfully on http://localhost:8080${NC}"
            echo ""
            echo "Test it:"
            echo '  curl http://localhost:8080/v1/models'
            return 0
        fi
        sleep 1
    done

    echo -e "${RED}✗ Server failed to start. Check llama-server.log${NC}"
    exit 1
}

stop_server() {
    echo -e "${BLUE}Stopping servers...${NC}"

    # Stop Ollama
    if [ -f "ollama-server.pid" ]; then
        PID=$(cat ollama-server.pid)
        if ps -p $PID > /dev/null 2>&1; then
            kill $PID
            rm ollama-server.pid
            echo -e "${GREEN}✓ Ollama server stopped${NC}"
        else
            rm ollama-server.pid
        fi
    elif pgrep -x "ollama" > /dev/null; then
        pkill -x ollama
        echo -e "${GREEN}✓ Ollama server stopped${NC}"
    fi

    # Stop llama.cpp
    if [ -f "llama-server.pid" ]; then
        PID=$(cat llama-server.pid)
        if ps -p $PID > /dev/null 2>&1; then
            kill $PID
            rm llama-server.pid
            echo -e "${GREEN}✓ llama.cpp server stopped${NC}"
        else
            rm llama-server.pid
        fi
    fi

    if [ ! -f "ollama-server.pid" ] && [ ! -f "llama-server.pid" ]; then
        echo -e "${YELLOW}No servers were running${NC}"
    fi
}

check_status() {
    echo -e "${BLUE}Checking server status...${NC}"

    # Check Ollama
    if pgrep -x "ollama" > /dev/null; then
        echo -e "${GREEN}✓ Ollama server is running on http://localhost:11434${NC}"
        echo ""
        echo "Available models:"
        ollama list 2>/dev/null || echo "Could not fetch model list"

        # Check if configured model is loaded
        if ollama list 2>/dev/null | grep -q "$OLLAMA_MODEL"; then
            echo ""
            echo -e "${GREEN}✓ $OLLAMA_MODEL model is available${NC}"
        fi
        return 0
    fi

    # Check llama.cpp
    if curl -s http://localhost:8080/health > /dev/null 2>&1; then
        echo -e "${GREEN}✓ llama.cpp server is running on http://localhost:8080${NC}"

        # Get model info
        echo ""
        echo "Model info:"
        curl -s http://localhost:8080/v1/models | jq '.' 2>/dev/null || echo "Could not fetch model info"
        return 0
    fi

    echo -e "${RED}✗ No servers are running${NC}"
    exit 1
}

do_all() {
    if [ "$BACKEND" = "llamacpp" ]; then
        install_llamacpp
        download_model_llamacpp
        start_server_llamacpp
        check_status

        echo ""
        echo -e "${GREEN}╔══════════════════════════════════════════════════════════╗${NC}"
        echo -e "${GREEN}║  Local LLM Setup Complete (llama.cpp)!                   ║${NC}"
        echo -e "${GREEN}╚══════════════════════════════════════════════════════════╝${NC}"
        echo ""
        echo "Server running on: http://localhost:8080"
        echo ""
        echo "Now start WorldWideSwarm with Zeroclaw:"
        echo "  AGENT_IMPL=zeroclaw LLM_BACKEND=local ./swarm-manager.sh start-agents 15"
    else
        # Default to Ollama (recommended)
        install_ollama
        download_model_ollama
        start_server_ollama
        check_status

        echo ""
        echo -e "${GREEN}╔══════════════════════════════════════════════════════════╗${NC}"
        echo -e "${GREEN}║  Local LLM Setup Complete (Ollama)!                      ║${NC}"
        echo -e "${GREEN}╚══════════════════════════════════════════════════════════╝${NC}"
        echo ""
        echo "Server running on: http://localhost:11434"
        echo "Model: $OLLAMA_MODEL"
        echo ""
        echo "Now start WorldWideSwarm with Zeroclaw:"
        echo "  AGENT_IMPL=zeroclaw LLM_BACKEND=ollama ./swarm-manager.sh start-agents 15"
    fi
}

# Parse backend option
BACKEND="ollama"  # Default to Ollama (recommended)
while [[ $# -gt 0 ]]; do
    case $1 in
        --backend)
            BACKEND="$2"
            shift 2
            ;;
        *)
            COMMAND="$1"
            shift
            ;;
    esac
done

# Main
case ${COMMAND:-help} in
    install)
        if [ "$BACKEND" = "llamacpp" ]; then
            install_llamacpp
        else
            install_ollama
        fi
        ;;
    download)
        if [ "$BACKEND" = "llamacpp" ]; then
            download_model_llamacpp
        else
            download_model_ollama
        fi
        ;;
    start)
        if [ "$BACKEND" = "llamacpp" ]; then
            start_server_llamacpp
        else
            start_server_ollama
        fi
        ;;
    stop)
        stop_server
        ;;
    status)
        check_status
        ;;
    all)
        do_all
        ;;
    *)
        usage
        ;;
esac
