#!/bin/bash
# Zeroclaw Agent Launcher for OpenSwarm
# Runs a Zeroclaw agent connected to OpenSwarm connector

set -e

# Parse arguments
AGENT_NAME=""
RPC_PORT=""
FILES_PORT=""
LLM_BACKEND="openrouter"  # Default: OpenRouter free model
MODEL_PATH=""
API_KEY=""
MODEL_NAME=""
API_BASE_URL=""
LLM_ARGS=""

usage() {
    cat << EOF
Zeroclaw Agent Launcher for OpenSwarm

Usage: $0 [OPTIONS]

Options:
    --agent-name NAME       Agent identifier
    --rpc-port PORT         RPC server port (default: 9370)
    --files-port PORT       File server port (default: 9371)
    --llm-backend BACKEND   LLM backend: anthropic|openai|openrouter|local|ollama (default: openrouter)
    --model-path PATH       Path to local model file (for local backend)
    --api-key KEY           API key for cloud providers
    --model-name NAME       Model name (e.g., claude-opus-4, gpt-4, llama3:70b)
    --api-base-url URL      Optional API base URL (OpenAI-compatible backends)
    -h, --help              Show this help

Examples:
    # Use Claude API (default)
    $0 --agent-name alice --rpc-port 9370 --llm-backend anthropic

    # Use local model
    $0 --agent-name alice --rpc-port 9370 --llm-backend local --model-path ./models/gpt-oss-20b.gguf

    # Use Ollama with gpt-oss:20b (recommended)
    $0 --agent-name alice --rpc-port 9370 --llm-backend ollama --model-name gpt-oss:20b

    # Use OpenRouter with MiniMax M2.5
    $0 --agent-name alice --rpc-port 9370 --llm-backend openrouter --model-name minimax/minimax-m2.5

EOF
    exit 0
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --agent-name)
            AGENT_NAME="$2"
            shift 2
            ;;
        --rpc-port)
            RPC_PORT="$2"
            shift 2
            ;;
        --files-port)
            FILES_PORT="$2"
            shift 2
            ;;
        --llm-backend)
            LLM_BACKEND="$2"
            shift 2
            ;;
        --model-path)
            MODEL_PATH="$2"
            shift 2
            ;;
        --api-key)
            API_KEY="$2"
            shift 2
            ;;
        --model-name)
            MODEL_NAME="$2"
            shift 2
            ;;
        --api-base-url)
            API_BASE_URL="$2"
            shift 2
            ;;
        -h|--help)
            usage
            ;;
        *)
            echo "Unknown option: $1"
            usage
            ;;
    esac
done

# Validate required arguments
if [ -z "$AGENT_NAME" ]; then
    echo "Error: --agent-name is required"
    exit 1
fi

if [ -z "$RPC_PORT" ]; then
    RPC_PORT="9370"
fi

if [ -z "$FILES_PORT" ]; then
    FILES_PORT="9371"
fi

# Check if zeroclaw is installed
if ! command -v zeroclaw &> /dev/null; then
    echo "Error: zeroclaw command not found"
    echo ""
    echo "Install zeroclaw from source:"
    echo "  git clone https://github.com/zeroclaw-labs/zeroclaw"
    echo "  cd zeroclaw && pip install -r requirements.txt && cd .."
    echo ""
    echo "Then add to PATH or use absolute path in this script."
    exit 1
fi

# Configure LLM backend
case $LLM_BACKEND in
    anthropic)
        if [ -z "$API_KEY" ] && [ -z "$ANTHROPIC_API_KEY" ]; then
            echo "Error: API key required for Anthropic backend"
            echo "Set ANTHROPIC_API_KEY environment variable or use --api-key"
            exit 1
        fi
        export ANTHROPIC_API_KEY="${API_KEY:-$ANTHROPIC_API_KEY}"
        MODEL_NAME="${MODEL_NAME:-claude-opus-4}"
        LLM_ARGS="-p anthropic --model $MODEL_NAME"
        ;;
    openai)
        if [ -z "$API_KEY" ] && [ -z "$OPENAI_API_KEY" ]; then
            echo "Error: API key required for OpenAI backend"
            echo "Set OPENAI_API_KEY environment variable or use --api-key"
            exit 1
        fi
        export OPENAI_API_KEY="${API_KEY:-$OPENAI_API_KEY}"
        if [ -n "$API_BASE_URL" ]; then
            export OPENAI_BASE_URL="$API_BASE_URL"
        fi
        MODEL_NAME="${MODEL_NAME:-gpt-4}"
        LLM_ARGS="-p openai --model $MODEL_NAME"
        ;;
    openrouter)
        if [ -z "$API_KEY" ] && [ -z "$OPENROUTER_API_KEY" ]; then
            echo "Error: API key required for OpenRouter backend"
            echo "Set OPENROUTER_API_KEY environment variable or use --api-key"
            exit 1
        fi

        export OPENAI_API_KEY="${API_KEY:-$OPENROUTER_API_KEY}"
        export OPENAI_BASE_URL="${API_BASE_URL:-https://openrouter.ai/api/v1}"
        MODEL_NAME="${MODEL_NAME:-arcee-ai/trinity-large-preview:free}"
        LLM_ARGS="-p openrouter --model $MODEL_NAME"
        ;;
    local)
        if [ -z "$MODEL_PATH" ]; then
            echo "Error: --model-path required for local backend"
            exit 1
        fi
        if [ ! -f "$MODEL_PATH" ]; then
            echo "Error: Model file not found: $MODEL_PATH"
            echo ""
            echo "Download a model first, for example:"
            echo "  wget https://huggingface.co/TheBloke/Llama-2-70B-GGUF/resolve/main/llama-2-70b.Q4_K_M.gguf -O models/llama-2-70b.gguf"
            exit 1
        fi
        MODEL_NAME="${MODEL_NAME:-local-model}"
        LLM_ARGS="-p custom:http://127.0.0.1:8080/v1 --model $MODEL_NAME"
        ;;
    ollama)
        MODEL_NAME="${MODEL_NAME:-gpt-oss:20b}"
        # Check if Ollama is running
        if ! curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
            echo "Error: Ollama server not running"
            echo "Start Ollama: ollama serve"
            exit 1
        fi
        # Check if model is available
        if ! ollama list 2>/dev/null | grep -q "$MODEL_NAME"; then
            echo "Warning: Model $MODEL_NAME not found locally"
            echo "Pulling model (this may take a few minutes)..."
            ollama pull "$MODEL_NAME" || {
                echo "Error: Failed to pull model $MODEL_NAME"
                echo "Available models:"
                ollama list
                exit 1
            }
        fi
        LLM_ARGS="-p ollama --model $MODEL_NAME"
        ;;
    *)
        echo "Error: Unknown LLM backend: $LLM_BACKEND"
        echo "Supported backends: anthropic, openai, openrouter, local, ollama"
        exit 1
        ;;
esac

# Create instructions file
INSTRUCTIONS_FILE="/tmp/zeroclaw-instructions-${AGENT_NAME}.txt"
cat > "$INSTRUCTIONS_FILE" << EOF
You are an autonomous OpenSwarm agent running in Zeroclaw.

CRITICAL: Run in an INFINITE LOOP until interrupted.

Your Agent ID: $AGENT_NAME
RPC Endpoint: tcp://127.0.0.1:$RPC_PORT
Skill Documentation: http://127.0.0.1:$FILES_PORT/SKILL.md

INITIALIZATION (run once):
1. Fetch skill documentation:
   curl http://127.0.0.1:$FILES_PORT/SKILL.md

2. Get your status to learn your canonical DID and tier:
   echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"status","signature":""}' | nc 127.0.0.1 $RPC_PORT

3. Register with swarm using your canonical DID from status.agent_id (not alias names):
   echo '{"jsonrpc":"2.0","method":"swarm.register_agent","params":{"agent_id":"<status.agent_id>"},"id":"init","signature":""}' | nc 127.0.0.1 $RPC_PORT

4. Parse the response to extract your tier: "tier": "Tier1" / "Tier2" / ... / "Executor"

5. Store your tier in memory for task processing

MAIN LOOP (run forever):
While true:
  1. Poll for tasks (every 60 seconds):
     echo '{"jsonrpc":"2.0","method":"swarm.receive_task","params":{},"id":"poll","signature":""}' | nc 127.0.0.1 $RPC_PORT

  2. Track which task IDs you've already processed (keep a list in memory)

  3. For each NEW task (not already processed):

      A. Get task details:
         echo '{"jsonrpc":"2.0","method":"swarm.get_task","params":{"task_id":"TASK_ID"},"id":"task","signature":""}' | nc 127.0.0.1 $RPC_PORT

     B. IF YOUR TIER IS A COORDINATOR (Tier1, Tier2, ..., TierN - anything except Executor):
        - You decompose tasks into subtasks
        - Analyze the task using your AI capabilities
        - Generate a decomposition plan (3-10 subtasks)
        - Create plan JSON:
          {
            "task_id": "TASK_ID",
            "proposer": "$AGENT_NAME",
            "epoch": EPOCH_FROM_TASK,
            "subtasks": [
              {"index": 1, "description": "Detailed subtask 1", "estimated_complexity": 0.2},
              {"index": 2, "description": "Detailed subtask 2", "estimated_complexity": 0.3},
              ...
            ],
            "rationale": "Why this decomposition is optimal",
            "estimated_parallelism": NUMBER_OF_SUBTASKS
          }
        - Submit plan:
          echo '{"jsonrpc":"2.0","method":"swarm.propose_plan","params":PLAN_JSON,"id":"propose","signature":""}' | nc 127.0.0.1 $RPC_PORT
        - Mark task as processed

     C. IF YOUR TIER IS 'Executor' (Leaf Worker):
        - You execute tasks, not decompose them
        - Perform the actual work using your AI capabilities:
          * Research the topic
          * Write code
          * Analyze data
          * Generate content
          * Whatever the task requires
        - Create result JSON:
          {
            "task_id": "TASK_ID",
            "agent_id": "$AGENT_NAME",
            "artifact": {
              "artifact_id": "TASK_ID-result",
              "task_id": "TASK_ID",
              "producer": "$AGENT_NAME",
              "content_cid": "HASH_OF_YOUR_RESULT",
              "merkle_hash": "HASH_OF_YOUR_RESULT",
              "content_type": "text/plain",
              "size_bytes": SIZE_OF_RESULT,
              "created_at": "CURRENT_TIMESTAMP"
            },
            "merkle_proof": []
          }
        - Submit result:
          echo '{"jsonrpc":"2.0","method":"swarm.submit_result","params":RESULT_JSON,"id":"result","signature":""}' | nc 127.0.0.1 $RPC_PORT
        - Mark task as processed

  4. Sleep 60 seconds and repeat

IMPORTANT:
- NEVER process the same task twice
- Use your bash execution tools for all RPC calls
- Log all actions for debugging
- Only stop when interrupted
- Coordinators propose PLANS, Executors perform WORK
- Provide high-quality, thoughtful work

Run autonomously until interrupted.
EOF

echo "Starting Zeroclaw agent: $AGENT_NAME"
echo "LLM Backend: $LLM_BACKEND"
echo "RPC Port: $RPC_PORT"
echo "Files Port: $FILES_PORT"
echo ""

# Run Zeroclaw in repeated single-shot mode for current CLI compatibility.
INSTRUCTIONS_TEXT="$(<"$INSTRUCTIONS_FILE")"

CONFIG_DIR="/tmp/zeroclaw-config-${AGENT_NAME}"
CONFIG_FILE="$CONFIG_DIR/config.toml"
mkdir -p "$CONFIG_DIR"

if [ -f "$HOME/.zeroclaw/config.toml" ]; then
    cp "$HOME/.zeroclaw/config.toml" "$CONFIG_FILE"
else
    cat > "$CONFIG_FILE" << 'EOF'
default_provider = "openrouter"
default_model = "minimax/minimax-m2.5"

[autonomy]
level = "full"
workspace_only = false
allowed_commands = ["*"]
forbidden_paths = ["~/.ssh", "~/.gnupg", "~/.aws"]
max_actions_per_hour = 10000
max_cost_per_day_cents = 100000
require_approval_for_medium_risk = false
block_high_risk_commands = false
auto_approve = ["shell", "file_write", "file_read", "list_dir"]

[runtime]
kind = "native"

[agent]
parallel_tools = true
EOF
fi

python3 - "$CONFIG_FILE" << 'PY'
import re
import sys

path = sys.argv[1]
with open(path, 'r', encoding='utf-8') as f:
    text = f.read()

def sub(pattern, repl):
    global text
    text = re.sub(pattern, repl, text, flags=re.MULTILINE)

sub(r'^level\s*=\s*"[^"]+"', 'level = "full"')
sub(r'^workspace_only\s*=\s*(true|false)', 'workspace_only = false')
sub(r'^allowed_commands\s*=\s*\[[^\]]*\]', 'allowed_commands = ["*"]')
sub(r'^max_actions_per_hour\s*=\s*\d+', 'max_actions_per_hour = 10000')
sub(r'^max_cost_per_day_cents\s*=\s*\d+', 'max_cost_per_day_cents = 100000')
sub(r'^require_approval_for_medium_risk\s*=\s*(true|false)', 'require_approval_for_medium_risk = false')
sub(r'^block_high_risk_commands\s*=\s*(true|false)', 'block_high_risk_commands = false')

if 'auto_approve =' not in text:
    text = text.replace(
        'block_high_risk_commands = false',
        'block_high_risk_commands = false\nauto_approve = ["shell", "file_write", "file_read", "list_dir"]'
    )

with open(path, 'w', encoding='utf-8') as f:
    f.write(text)
PY

while true; do
    # shellcheck disable=SC2086
    zeroclaw --config-dir "$CONFIG_DIR" agent $LLM_ARGS --message "$INSTRUCTIONS_TEXT" || true
    sleep 30
done
