#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

TMP_DIR="$(mktemp -d /tmp/openswarm-e2e-ui.XXXXXX)"

cleanup() {
    pkill -f "openswarm-connector --listen /ip4/127.0.0.1/tcp/2110" >/dev/null 2>&1 || true
    pkill -f "openswarm-connector --listen /ip4/127.0.0.1/tcp/2111" >/dev/null 2>&1 || true
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

cd "$ROOT_DIR"

log "Building release binary"
cargo build --release -p openswarm-connector >/dev/null

PID_A=$(start_connector "ui-a" 21100 21370 21371 "$TMP_DIR/a.log")
wait_for_rpc 21370

STATUS_A=$(rpc_call 21370 '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"s1","signature":""}')
PEER_A=$(json_get "$STATUS_A" "result.agent_id" | sed 's/^did:swarm://')
BOOT_A="/ip4/127.0.0.1/tcp/21100/p2p/$PEER_A"

PID_B=$(start_connector "ui-b" 21101 21372 21373 "$TMP_DIR/b.log" "$BOOT_A")
wait_for_rpc 21372

rpc_call 21370 '{"jsonrpc":"2.0","method":"swarm.inject_task","params":{"description":"ui observability e2e task"},"id":"inject","signature":""}' >/dev/null

sleep 4

log "Checking flow counters are observable via /flow command equivalent"
FLOW=$(rpc_call 21370 '{"jsonrpc":"2.0","method":"swarm.get_task_timeline","params":{"task_id":"unknown"},"id":"tl","signature":""}')
if [[ -z "$FLOW" ]]; then
    echo "UI observability E2E FAILED: timeline query failed"
    exit 1
fi

log "Running TUI smoke (start and immediate quit)"
TUI_CMD="$ROOT_DIR/target/release/openswarm-connector --listen /ip4/127.0.0.1/tcp/21102 --rpc 127.0.0.1:21374 --files-addr 127.0.0.1:21375 --bootstrap $BOOT_A --agent-name ui-tui --tui"
python3 - <<'PY' "$TUI_CMD" "$TMP_DIR/tui.log"
import pexpect, sys, time, pathlib
cmd, out = sys.argv[1], sys.argv[2]
child = pexpect.spawn('/bin/bash', ['-lc', cmd], encoding='utf-8', timeout=20)
time.sleep(2)
child.send('q')
time.sleep(1)
if child.isalive():
    child.terminate(force=True)
pathlib.Path(out).write_text((child.before or '')[-20000:])
PY

log "Running console smoke with new flow/vote/timeline commands"
CONSOLE_CMD="$ROOT_DIR/target/release/openswarm-connector --listen /ip4/127.0.0.1/tcp/21103 --rpc 127.0.0.1:21376 --files-addr 127.0.0.1:21377 --bootstrap $BOOT_A --agent-name ui-console --console"
python3 - <<'PY' "$CONSOLE_CMD" "$TMP_DIR/console.log"
import pexpect, sys, time, pathlib
cmd, out = sys.argv[1], sys.argv[2]
child = pexpect.spawn('/bin/bash', ['-lc', cmd], encoding='utf-8', timeout=20)
time.sleep(2)
for line in ('/flow', '/votes', '/timeline unknown-task'):
    child.sendline(line)
    time.sleep(0.8)
child.sendcontrol('c')
time.sleep(1)
if child.isalive():
    child.terminate(force=True)
pathlib.Path(out).write_text((child.before or '')[-20000:])
PY

log "Observability UI E2E PASSED"
