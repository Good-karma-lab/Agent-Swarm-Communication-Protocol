#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
BIN="$ROOT_DIR/target/release/openswarm-connector"

log() {
    printf "[%s] %s\n" "$(date +%H:%M:%S)" "$*"
}

rpc_call() {
    local port="$1"
    local payload="$2"
    printf '%s\n' "$payload" | nc 127.0.0.1 "$port"
}

json_get() {
    local json="$1"
    local expr="$2"
    python3 - <<'PY' "$json" "$expr"
import json, sys
doc = json.loads(sys.argv[1])
expr = sys.argv[2].strip('.')
cur = doc
for part in expr.split('.'):
    if not part:
        continue
    if isinstance(cur, dict):
        cur = cur.get(part)
    else:
        cur = None
        break
if isinstance(cur, (dict, list)):
    print(json.dumps(cur))
elif cur is None:
    print("")
else:
    print(cur)
PY
}

wait_for_rpc() {
    local port="$1"
    local tries="${2:-30}"
    local i
    for i in $(seq 1 "$tries"); do
        if rpc_call "$port" '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"status","signature":""}' >/dev/null 2>&1; then
            return 0
        fi
        sleep 1
    done
    return 1
}

start_connector() {
    local name="$1"
    local p2p_port="$2"
    local rpc_port="$3"
    local files_port="$4"
    local log_file="$5"
    local bootstrap="${6:-}"
    local extra_args="${7:-}"

    local cmd=("$BIN" "--listen" "/ip4/127.0.0.1/tcp/$p2p_port" "--rpc" "127.0.0.1:$rpc_port" "--files-addr" "127.0.0.1:$files_port" "--agent-name" "$name")

    if [[ -n "$bootstrap" ]]; then
        cmd+=("--bootstrap" "$bootstrap")
    fi

    if [[ -n "$extra_args" ]]; then
        # shellcheck disable=SC2206
        local extra=( $extra_args )
        cmd+=("${extra[@]}")
    fi

    "${cmd[@]}" >"$log_file" 2>&1 &
    local pid=$!
    echo "$pid"
}
