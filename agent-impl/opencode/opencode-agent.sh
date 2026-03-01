#!/usr/bin/env bash
# WorldWideSwarm agent using opencode CLI for LLM reasoning.
# The shell loop handles polling/RPC; opencode run handles one task at a time.
#
# Usage: opencode-agent.sh <agent-name> <rpc-port> <files-port>

AGENT_NAME="${1:?agent-name required}"
RPC_PORT="${2:?rpc-port required}"
FILES_PORT="${3:?files-port required}"

OPENCODE_BIN="${OPENCODE_BIN:-/opt/homebrew/bin/opencode}"
OPENCODE_MODEL="${OPENCODE_MODEL:-openai/gpt-5.2-codex}"
POLL_INTERVAL="${POLL_INTERVAL:-30}"

# File to track processed task IDs (works on bash 3 / macOS)
PROCESSED_FILE=$(mktemp /tmp/wws-processed-XXXX)
trap 'rm -f "$PROCESSED_FILE"' EXIT

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

rpc() {
    echo "$1" | /usr/bin/nc 127.0.0.1 "$RPC_PORT" 2>/dev/null || true
}

# Extract a JSON field from a string: json_field <json> <dotted.path>
json_field() {
    printf '%s' "$1" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    parts = '$2'.split('.')
    v = data
    for p in parts:
        if isinstance(v, list):
            v = v[int(p)]
        else:
            v = v[p]
    print(v if v is not None else '')
except Exception:
    print('')
" 2>/dev/null || true
}

# Extract first element of a JSON array field
json_array_first() {
    printf '%s' "$1" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    parts = '$2'.split('.')
    v = data
    for p in parts:
        v = v[p]
    if isinstance(v, list) and len(v) > 0:
        print(v[0])
    else:
        print('')
except Exception:
    print('')
" 2>/dev/null || true
}

already_processed() {
    grep -qxF "$1" "$PROCESSED_FILE" 2>/dev/null
}

mark_processed() {
    echo "$1" >> "$PROCESSED_FILE"
}

log() { echo "[$(date '+%H:%M:%S')] [$AGENT_NAME] $*"; }

# ---------------------------------------------------------------------------
# Register + learn tier
# ---------------------------------------------------------------------------

log "Registering with connector on port $RPC_PORT..."
REGISTER_RESP=$(rpc "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.register_agent\",\"params\":{\"agent_id\":\"$AGENT_NAME\"},\"id\":\"register\",\"signature\":\"\"}")
log "Register: $REGISTER_RESP"

STATUS_RESP=$(rpc "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.get_status\",\"params\":{},\"id\":\"status\",\"signature\":\"\"}")
log "Status: $STATUS_RESP"

TIER=$(json_field "$STATUS_RESP" "result.tier")
AGENT_DID=$(json_field "$STATUS_RESP" "result.agent_id")
log "Tier: $TIER  DID: $AGENT_DID"

# ---------------------------------------------------------------------------
# Main polling loop
# receive_task returns: {"result": {"pending_tasks": ["id1","id2"], "tier": "...", "agent_id": "..."}}
# For each pending task ID we call swarm.get_task to get full details.
# ---------------------------------------------------------------------------

log "Starting poll loop (interval: ${POLL_INTERVAL}s, model: $OPENCODE_MODEL, initial tier: $TIER)..."

while true; do
    # Re-query tier each cycle so we pick up TierAssignment messages from the network
    CURRENT_STATUS=$(rpc "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.get_status\",\"params\":{},\"id\":\"status2\",\"signature\":\"\"}")
    NEW_TIER=$(json_field "$CURRENT_STATUS" "result.tier")
    if [ -n "$NEW_TIER" ] && [ "$NEW_TIER" != "$TIER" ]; then
        log "Tier updated: $TIER -> $NEW_TIER"
        TIER="$NEW_TIER"
    fi

    RECV_RESP=$(rpc "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.receive_task\",\"params\":{},\"id\":\"recv\",\"signature\":\"\"}")

    # Get the first pending task ID from the list
    TASK_ID=$(json_array_first "$RECV_RESP" "result.pending_tasks")

    if [ -z "$TASK_ID" ] || already_processed "$TASK_ID"; then
        sleep "$POLL_INTERVAL"
        continue
    fi

    # Fetch full task details (get_task returns {"result": {"task": {...}, "is_pending": ...}})
    TASK_RESP=$(rpc "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.get_task\",\"params\":{\"task_id\":\"$TASK_ID\"},\"id\":\"get\",\"signature\":\"\"}")
    TASK_DESC=$(json_field "$TASK_RESP" "result.task.description")
    TASK_EPOCH=$(json_field "$TASK_RESP" "result.task.epoch")
    TASK_EPOCH="${TASK_EPOCH:-1}"

    log "New task $TASK_ID (epoch $TASK_EPOCH, tier: $TIER): $TASK_DESC"
    mark_processed "$TASK_ID"

    if [ "$TIER" != "Executor" ]; then
        # ------------------------------------------------------------------
        # Coordinator: use opencode to generate a decomposition plan
        # ------------------------------------------------------------------
        log "Acting as coordinator — generating decomposition plan..."

        PLAN_ID=$(python3 -c "import uuid; print(str(uuid.uuid4()))")
        CREATED_AT=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

        # Determine subtask count = available executor members (known_agents / 2, min 1)
        KNOWN_AGENTS=$(json_field "$CURRENT_STATUS" "result.known_agents")
        SUBTASK_COUNT=$(python3 -c "n=int('${KNOWN_AGENTS:-5}' or 5); print(max(1, n//2))" 2>/dev/null || echo "2")
        log "Known agents: $KNOWN_AGENTS — generating $SUBTASK_COUNT subtasks"

        # Build the subtask template lines dynamically based on SUBTASK_COUNT
        SUBTASK_TEMPLATE=$(python3 -c "
import json
n = int('$SUBTASK_COUNT')
items = []
for i in range(1, n+1):
    items.append(json.dumps({'index': i, 'description': f'specific subtask {i}', 'required_capabilities': [], 'estimated_complexity': round(1.0/n, 2)}))
print(',\n    '.join(items))
" 2>/dev/null || echo '{"index": 1, "description": "specific subtask 1", "required_capabilities": [], "estimated_complexity": 1.0}')

        COORD_PROMPT="You are a swarm coordinator agent. Decompose the following task into exactly $SUBTASK_COUNT concrete subtasks that can be executed in parallel (one per available executor agent). Do NOT ask questions or use the question tool — output only the JSON response below.

Task ID: $TASK_ID
Task Description: $TASK_DESC

Output ONLY valid JSON (no markdown fences, no explanation), just the raw JSON object:
{
  \"plan_id\": \"$PLAN_ID\",
  \"task_id\": \"$TASK_ID\",
  \"proposer\": \"$AGENT_NAME\",
  \"epoch\": $TASK_EPOCH,
  \"created_at\": \"$CREATED_AT\",
  \"subtasks\": [
    $SUBTASK_TEMPLATE
  ],
  \"rationale\": \"brief reason for this decomposition\",
  \"estimated_parallelism\": $SUBTASK_COUNT.0
}"

        PLAN_JSON=$("$OPENCODE_BIN" run "$COORD_PROMPT" -m "$OPENCODE_MODEL" 2>/dev/null | \
            python3 -c "
import sys, json, re, uuid
from datetime import datetime, timezone
text = sys.stdin.read()
# Strip markdown fences if present
text = re.sub(r'\`\`\`json\s*', '', text)
text = re.sub(r'\`\`\`\s*', '', text)
# Try to find and parse the first JSON object in the output
m = re.search(r'\{.*\}', text, re.DOTALL)
if m:
    try:
        obj = json.loads(m.group())
        # Ensure required fields are present
        if 'plan_id' not in obj:
            obj['plan_id'] = str(uuid.uuid4())
        if 'created_at' not in obj:
            obj['created_at'] = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
        if 'estimated_parallelism' not in obj:
            obj['estimated_parallelism'] = float(len(obj.get('subtasks', [])))
        for st in obj.get('subtasks', []):
            if 'required_capabilities' not in st:
                st['required_capabilities'] = []
        print(json.dumps(obj))
        sys.exit(0)
    except Exception:
        pass
print('')
" 2>/dev/null || true)

        if [ -n "$PLAN_JSON" ]; then
            log "Submitting plan: $(printf '%s' "$PLAN_JSON" | head -c 200)..."
            PROPOSE_RESP=$(rpc "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.propose_plan\",\"params\":$PLAN_JSON,\"id\":\"propose\",\"signature\":\"\"}")
            log "Propose response: $PROPOSE_RESP"

            MY_PLAN_ID=$(printf '%s' "$PROPOSE_RESP" | python3 -c "
import sys, json
try:
    d = json.loads(sys.stdin.read())
    print(d.get('result', {}).get('plan_id', ''))
except Exception:
    print('')
" 2>/dev/null || true)

            if [ -n "$MY_PLAN_ID" ]; then
                # Wait for other proposals to propagate via P2P (poll get_voting_state up to 25s)
                WAIT=0
                ALL_PLAN_IDS="[]"
                while [ $WAIT -lt 25 ]; do
                    sleep 5
                    WAIT=$((WAIT + 5))
                    # Use swarm.get_voting_state (returns plan_ids from local rfp_coordinators)
                    VS_RESP=$(rpc "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.get_voting_state\",\"params\":{\"task_id\":\"$TASK_ID\"},\"id\":\"vs\",\"signature\":\"\"}")
                    ALL_PLAN_IDS=$(printf '%s' "$VS_RESP" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    for entry in data.get('result', {}).get('rfp_coordinators', []):
        if entry.get('task_id') == '$TASK_ID':
            print(json.dumps(entry.get('plan_ids', [])))
            sys.exit(0)
    print('[]')
except Exception:
    print('[]')
" 2>/dev/null || echo "[]")
                    # Stop waiting if we see at least 2 plans (ours + at least 1 other)
                    COUNT=$(printf '%s' "$ALL_PLAN_IDS" | python3 -c "import sys,json; print(len(json.load(sys.stdin)))" 2>/dev/null || echo "0")
                    if [ "$COUNT" -ge 2 ]; then
                        break
                    fi
                done

                # Build rankings: EXCLUDE own plan (self-voting not permitted by protocol)
                RANKINGS=$(python3 -c "
import sys, json
my = '$MY_PLAN_ID'
try:
    all_ids = json.loads('''$ALL_PLAN_IDS''')
except Exception:
    all_ids = []
others = [p for p in all_ids if p != my]
print(json.dumps(others))
" 2>/dev/null || echo "[]")

                if [ "$RANKINGS" = "[]" ]; then
                    log "No other proposals known yet — skipping vote (system will use timeout fallback)"
                else
                    log "Submitting vote (excluding own plan, ranking others): $RANKINGS"
                    VOTE_RESP=$(rpc "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.submit_vote\",\"params\":{\"task_id\":\"$TASK_ID\",\"rankings\":$RANKINGS,\"epoch\":$TASK_EPOCH},\"id\":\"vote\",\"signature\":\"\"}")
                    log "Vote response: $VOTE_RESP"
                fi
            else
                log "WARNING: could not extract plan_id from propose response — skipping vote"
            fi
        else
            log "WARNING: could not parse plan JSON from opencode output — skipping"
        fi

    else
        # ------------------------------------------------------------------
        # Executor: use opencode to do the actual work
        # ------------------------------------------------------------------
        log "Acting as executor — performing task work..."

        EXEC_PROMPT="You are a swarm executor agent. Complete the following task thoroughly and produce a high-quality result.

Task: $TASK_DESC

IMPORTANT: Do NOT ask clarifying questions or use the question tool. Work directly with the information provided and produce a complete response. Do not request user input.

Provide a detailed, actionable response. Use your knowledge and reasoning to produce the best possible output."

        RESULT_TEXT=$("$OPENCODE_BIN" run "$EXEC_PROMPT" -m "$OPENCODE_MODEL" 2>/dev/null || echo "Task completed: $TASK_DESC")
        [ -z "$RESULT_TEXT" ] && RESULT_TEXT="Task completed: $TASK_DESC"

        RESULT_SIZE=${#RESULT_TEXT}
        CONTENT_HASH=$(printf '%s' "$RESULT_TEXT" | shasum -a 256 | awk '{print $1}')
        CREATED_AT=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

        RESULT_TEXT_JSON=$(python3 -c "import sys,json; print(json.dumps(sys.stdin.read()))" <<< "$RESULT_TEXT" 2>/dev/null || echo "\"$TASK_ID completed\"")

        RESULT_JSON="{\"task_id\":\"$TASK_ID\",\"agent_id\":\"$AGENT_NAME\",\"result_text\":$RESULT_TEXT_JSON,\"artifact\":{\"artifact_id\":\"$TASK_ID-result\",\"task_id\":\"$TASK_ID\",\"producer\":\"$AGENT_NAME\",\"content_cid\":\"$CONTENT_HASH\",\"merkle_hash\":\"$CONTENT_HASH\",\"content_type\":\"text/plain\",\"size_bytes\":$RESULT_SIZE,\"created_at\":\"$CREATED_AT\"},\"merkle_proof\":[]}"

        log "Submitting result (${RESULT_SIZE} bytes)..."
        SUBMIT_RESP=$(rpc "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.submit_result\",\"params\":$RESULT_JSON,\"id\":\"result\",\"signature\":\"\"}")
        log "Submit response: $SUBMIT_RESP"
    fi

    sleep "$POLL_INTERVAL"
done
