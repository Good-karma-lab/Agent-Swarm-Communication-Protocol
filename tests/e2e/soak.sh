#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if [ -f "$ROOT_DIR/scripts/load-env.sh" ]; then
    # shellcheck disable=SC1091
    source "$ROOT_DIR/scripts/load-env.sh"
fi

cd "$ROOT_DIR"

SOAK_SECS=${SOAK_SECS:-180}
INJECT_EVERY=${INJECT_EVERY:-20}
MAX_RSS_KB_GROWTH=${MAX_RSS_KB_GROWTH:-300000}

cleanup() {
    ./swarm-manager.sh stop >/dev/null 2>&1 || true
}
trap cleanup EXIT

./swarm-manager.sh stop >/dev/null 2>&1 || true
./swarm-manager.sh start 5 >/dev/null
sleep 20

RPC=$(awk -F'|' 'NR==1 {print $4}' /tmp/openswarm-swarm/nodes.txt)
PID=$(awk -F'|' 'NR==1 {print $2}' /tmp/openswarm-swarm/nodes.txt)

rss_kb() {
    ps -o rss= -p "$1" | tr -d ' '
}

RSS_START=$(rss_kb "$PID")
END_TS=$(( $(date +%s) + SOAK_SECS ))
COUNT=0

while [ "$(date +%s)" -lt "$END_TS" ]; do
    COUNT=$((COUNT + 1))
    echo "{\"jsonrpc\":\"2.0\",\"method\":\"swarm.inject_task\",\"params\":{\"description\":\"soak task #$COUNT\"},\"id\":\"inj-$COUNT\",\"signature\":\"\"}" | nc 127.0.0.1 "$RPC" >/dev/null || true
    sleep "$INJECT_EVERY"
done

RSS_END=$(rss_kb "$PID")
GROWTH=$((RSS_END - RSS_START))

STATUS=$(echo '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"status","signature":""}' | nc 127.0.0.1 "$RPC")
TASKS=$(python3 - <<'PY' "$STATUS"
import json,sys
d=json.loads(sys.argv[1])
print(d.get('result',{}).get('active_tasks',0))
PY
)

if [ "$GROWTH" -gt "$MAX_RSS_KB_GROWTH" ]; then
    echo "Soak FAILED: RSS growth too high (${GROWTH}KB)"
    exit 1
fi

if [ "$TASKS" -gt 500 ]; then
    echo "Soak FAILED: active tasks too high ($TASKS)"
    exit 1
fi

echo "Soak E2E PASSED (rss_growth_kb=$GROWTH, active_tasks=$TASKS)"
