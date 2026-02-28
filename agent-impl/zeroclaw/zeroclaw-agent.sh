#!/bin/bash
# Zeroclaw Agent Launcher for OpenSwarm
# Runs a Zeroclaw agent connected to OpenSwarm connector

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

if [ -f "$ROOT_DIR/scripts/load-env.sh" ]; then
    # shellcheck disable=SC1091
    source "$ROOT_DIR/scripts/load-env.sh"
fi

# Parse arguments
AGENT_NAME=""
RPC_PORT=""
FILES_PORT=""
LLM_BACKEND="${LLM_BACKEND:-openrouter}"  # Default: OpenRouter free model
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

    # Use local llama.cpp server
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
        # Use zeroclaw's first-class llama.cpp provider to avoid custom-provider
        # API key requirements and extra onboarding state.
        local_model_id="$(basename "$MODEL_PATH")"
        if [ -z "$MODEL_NAME" ] || [ "$MODEL_NAME" = "${local_model_id%.gguf}" ]; then
            MODEL_NAME="$local_model_id"
        fi
        LLM_ARGS="-p llamacpp --model $MODEL_NAME"
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
ASIP.Connector worker: run one cycle and exit.

Agent: $AGENT_NAME
RPC: 127.0.0.1:$RPC_PORT

Hard rules:
- Use shell + nc for all RPC calls.
- Never use notification tools (including pushover).
- Do not ask for confirmation.
- Do not run your own infinite loop.
- All JSON must be valid; escape strings carefully.

RPC reference:
- status:        echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"s","signature":""}' | nc -w 10 127.0.0.1 $RPC_PORT
- receive:       echo '{"jsonrpc":"2.0","method":"swarm.receive_task","params":{},"id":"r","signature":""}' | nc -w 10 127.0.0.1 $RPC_PORT
- get_task:      echo '{"jsonrpc":"2.0","method":"swarm.get_task","params":{"task_id":"TASK_ID"},"id":"t","signature":""}' | nc -w 10 127.0.0.1 $RPC_PORT
- get_vote_state:echo '{"jsonrpc":"2.0","method":"swarm.get_voting_state","params":{"task_id":"TASK_ID"},"id":"v","signature":""}' | nc -w 10 127.0.0.1 $RPC_PORT
- get_board:     echo '{"jsonrpc":"2.0","method":"swarm.get_board_status","params":{},"id":"b","signature":""}' | nc -w 10 127.0.0.1 $RPC_PORT

Determine your role for each pending task:
- COORDINATOR: task has no assigned_to (or assigned_to is not you) and task is Pending/Deliberating
- EXECUTOR: task.assigned_to matches your agent_id and task status is InProgress

=== COORDINATOR WORKFLOW ===

Step A - PROPOSE: Submit a decomposition plan via swarm.propose_plan.
Include 3-6 subtasks, each with estimated_complexity (0.0=trivial, 1.0=very complex).
Set estimated_complexity > 0.4 for subtasks that themselves require multi-agent deliberation.
Example:
  echo '{"jsonrpc":"2.0","method":"swarm.propose_plan","params":{"task_id":"TASK_ID","plan":{"plan_id":"plan-AGENT_NAME-1","proposer":"AGENT_ID","rationale":"...","subtasks":[{"index":0,"description":"...","required_capabilities":[],"estimated_complexity":0.3}],"estimated_parallelism":2,"epoch":1}},"id":"p","signature":""}' | nc -w 10 127.0.0.1 $RPC_PORT

Step B - VOTE: Get all proposals and rank them via swarm.submit_vote.
First call swarm.get_voting_state to get plan_ids, then vote:
  echo '{"jsonrpc":"2.0","method":"swarm.submit_vote","params":{"task_id":"TASK_ID","rankings":["plan-id-1","plan-id-2"],"epoch":1},"id":"vt","signature":""}' | nc -w 10 127.0.0.1 $RPC_PORT

Step C - CRITIQUE: Score each proposal on quality dimensions.
1. Call swarm.get_voting_state to list all plan_ids.
2. Call swarm.get_board_status to check if you are the adversarial critic:
   - Parse the JSON to find the holon for your task_id
   - Check if adversarial_critic field matches your agent_id
3. For EACH plan_id, score on: feasibility, parallelism, completeness, risk (all 0.0-1.0)
   - If you ARE the adversarial critic: focus on weaknesses; lower feasibility/completeness for flawed plans
   - If you are NOT adversarial: score fairly and objectively
4. Submit via swarm.submit_critique:
  echo '{"jsonrpc":"2.0","method":"swarm.submit_critique","params":{"task_id":"TASK_ID","round":2,"plan_scores":{"PLAN_ID":{"feasibility":0.8,"parallelism":0.7,"completeness":0.9,"risk":0.2}},"content":"Brief explanation of your scoring rationale"},"id":"cr","signature":""}' | nc -w 10 127.0.0.1 $RPC_PORT

Step D - SYNTHESIS (only when all subtasks complete): When your coordinator task has subtasks
and all of them have status=Completed, synthesize the results:
1. Call swarm.get_task for each subtask_id to retrieve results.
2. Synthesize all results into a unified, coherent answer using the LLM.
3. Submit the synthesis for the PARENT task via swarm.submit_result with is_synthesis=true:
  echo '{"jsonrpc":"2.0","method":"swarm.submit_result","params":{"task_id":"PARENT_TASK_ID","artifact":{"artifact_id":"synth-AGENT_NAME","task_id":"PARENT_TASK_ID","producer":"AGENT_ID","content_cid":"sha256:...","merkle_hash":"...","content_type":"text/plain","size_bytes":100,"created_at":"2024-01-01T00:00:00Z"},"content":"SYNTHESIZED RESULT TEXT","is_synthesis":true},"id":"rs","signature":""}' | nc -w 10 127.0.0.1 $RPC_PORT

=== EXECUTOR WORKFLOW ===

When task.assigned_to matches your agent_id:
1. Read the full task description via swarm.get_task.
2. Execute the task: produce real, substantive text output.
3. Submit the result via swarm.submit_result:
  echo '{"jsonrpc":"2.0","method":"swarm.submit_result","params":{"task_id":"TASK_ID","artifact":{"artifact_id":"result-AGENT_NAME-1","task_id":"TASK_ID","producer":"AGENT_ID","content_cid":"sha256:result","merkle_hash":"abc123","content_type":"text/plain","size_bytes":100,"created_at":"2024-01-01T00:00:00Z"},"content":"YOUR RESULT TEXT HERE"},"id":"sr","signature":""}' | nc -w 10 127.0.0.1 $RPC_PORT

Complete one full cycle (all applicable steps) then stop.
EOF

echo "Starting Zeroclaw agent: $AGENT_NAME"
echo "LLM Backend: $LLM_BACKEND"
echo "RPC Port: $RPC_PORT"
echo "Files Port: $FILES_PORT"
echo ""

# Run Zeroclaw in repeated single-shot mode for current CLI compatibility.
BASE_INSTRUCTIONS_TEXT="$(<"$INSTRUCTIONS_FILE")"

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
auto_approve = ["*"]

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
sub(r'^auto_approve\s*=\s*\[[^\]]*\]', 'auto_approve = ["*"]')

if 'auto_approve =' not in text:
    text = text.replace(
        'block_high_risk_commands = false',
        'block_high_risk_commands = false\nauto_approve = ["*"]'
    )

with open(path, 'w', encoding='utf-8') as f:
    f.write(text)
PY

rpc_call() {
    local payload="$1"
    local out=""
    for _ in 1 2 3; do
        out="$(printf '%s\n' "$payload" | nc -w 10 127.0.0.1 "$RPC_PORT" 2>/dev/null || true)"
        if [ -n "$out" ]; then
            printf '%s\n' "$out"
            return 0
        fi
        sleep 1
    done
    return 0
}

STATUS_JSON="$(rpc_call '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"status","signature":""}')"
AGENT_DID="$(python3 - "$STATUS_JSON" <<'PY'
import json
import sys

raw = sys.argv[1]
try:
    data = json.loads(raw)
    print(data.get("result", {}).get("agent_id", ""))
except Exception:
    print("")
PY
)"

if [ -n "$AGENT_DID" ]; then
    rpc_call "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.register_agent\",\"params\":{\"agent_id\":\"$AGENT_DID\"},\"id\":\"register\",\"signature\":\"\"}" >/dev/null
fi

JITTER="$(( $(printf '%s' "$AGENT_NAME" | cksum | awk '{print $1}') % 120 ))"
sleep "$JITTER"

EMPTY_BACKOFF=10

while true; do
    RECEIVE_JSON="$(rpc_call '{"jsonrpc":"2.0","method":"swarm.receive_task","params":{},"id":"receive","signature":""}')"
    PENDING_TASKS="$(python3 - "$RECEIVE_JSON" <<'PY'
import json
import sys

raw = sys.argv[1]
try:
    data = json.loads(raw)
    tasks = data.get("result", {}).get("pending_tasks", [])
    if isinstance(tasks, list):
        print("\n".join(str(t) for t in tasks))
except Exception:
    pass
PY
)"

    PENDING_COUNT="$(printf '%s\n' "$PENDING_TASKS" | awk 'NF { c++ } END { print c+0 }')"
    echo "$(date -u +%FT%TZ) poll pending_tasks=$PENDING_COUNT"

    if [ -z "$PENDING_TASKS" ]; then
        sleep "$EMPTY_BACKOFF"
        EMPTY_BACKOFF=$(( EMPTY_BACKOFF * 2 ))
        if [ "$EMPTY_BACKOFF" -gt 40 ]; then
            EMPTY_BACKOFF=40
        fi
        continue
    fi

    EMPTY_BACKOFF=10

    INSTRUCTIONS_TEXT="$BASE_INSTRUCTIONS_TEXT

Current status snapshot:
- agent_did: ${AGENT_DID:-unknown}
- rpc_port: $RPC_PORT
- pending_task_ids:
$PENDING_TASKS

Execute exactly one cycle now for these pending tasks only."

    # Feed one-shot "Always" approval to avoid interactive stalls on tool execution.
    # shellcheck disable=SC2086
    printf 'A\n' | zeroclaw --config-dir "$CONFIG_DIR" agent $LLM_ARGS --message "$INSTRUCTIONS_TEXT" || true
    # Cooldown to avoid provider free-tier burst limits.
    sleep 120
done
