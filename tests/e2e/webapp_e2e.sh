#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
if [ -f "$ROOT_DIR/scripts/load-env.sh" ]; then
    # shellcheck disable=SC1091
    source "$ROOT_DIR/scripts/load-env.sh"
fi

cd "$ROOT_DIR"

TMP_DIR="$(mktemp -d /tmp/wws-web-e2e.XXXXXX)"
PID=""

cleanup() {
    if [ -n "$PID" ]; then
        kill "$PID" >/dev/null 2>&1 || true
        wait "$PID" 2>/dev/null || true
    fi
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

cargo build --release -p wws-connector >/dev/null

./target/release/wws-connector \
  --listen /ip4/127.0.0.1/tcp/22100 \
  --rpc 127.0.0.1:22370 \
  --files-addr 127.0.0.1:22371 \
  --agent-name web-e2e \
  >"$TMP_DIR/node.log" 2>&1 &
PID=$!

for _ in $(seq 1 30); do
    if curl -sf "http://127.0.0.1:22371/api/health" >/dev/null; then
        break
    fi
    sleep 1
done

curl -sf "http://127.0.0.1:22371/" >/dev/null
curl -sf "http://127.0.0.1:22371/api/hierarchy" >/dev/null
curl -sf "http://127.0.0.1:22371/api/voting" >/dev/null
curl -sf "http://127.0.0.1:22371/api/messages" >/dev/null
curl -sf "http://127.0.0.1:22371/api/audit" >/dev/null
curl -sf "http://127.0.0.1:22371/api/topology" >/dev/null
curl -sf "http://127.0.0.1:22371/api/flow" >/dev/null

RESP=$(curl -sf -X POST "http://127.0.0.1:22371/api/tasks" -H "Content-Type: application/json" -d '{"description":"webapp e2e task"}')
TASK_ID=$(python3 - <<'PY' "$RESP"
import json,sys
doc=json.loads(sys.argv[1])
print(doc.get('task_id',''))
PY
)

if [ -z "$TASK_ID" ]; then
    echo "webapp e2e FAILED: task id missing"
    exit 1
fi

curl -sf "http://127.0.0.1:22371/api/tasks/${TASK_ID}/timeline" >/dev/null

echo "Webapp E2E PASSED"
