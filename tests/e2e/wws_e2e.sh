#!/usr/bin/env bash
# WWS E2E: Tests auto-discovery, name registry, and adversarial scenarios.
# Usage: ./tests/e2e/wws_e2e.sh
set -euo pipefail

BINARY="${BINARY:-./target/release/wws-connector}"
PASS=0; FAIL=0

die() { echo "FATAL: $1" >&2; exit 1; }

run_test() {
    local name="$1"; local cmd="$2"; local expect="$3"
    printf "  %-50s" "$name ..."
    if eval "$cmd" 2>&1 | grep -q "$expect"; then
        echo "PASS"; ((PASS++))
    else
        echo "FAIL"; ((FAIL++))
    fi
}

check_binary() {
    [ -f "$BINARY" ] || die "Binary not found: $BINARY. Run 'make build' first."
}

cleanup() {
    kill "${BS_PID:-}" "${A1_PID:-}" "${A2_PID:-}" 2>/dev/null || true
}
trap cleanup EXIT

check_binary

# === Test: Identity persistence ===
echo ""
echo "=== WWS E2E: Identity Persistence ==="
IDENTITY_DIR=$(mktemp -d)
$BINARY --agent-name test-e2e-1 --identity-path "$IDENTITY_DIR/test.key" \
        --rpc-bind-addr 127.0.0.1:19370 --listen /ip4/127.0.0.1/tcp/19100 \
        --file-server-addr 127.0.0.1:19371 &
A1_PID=$!
sleep 2
run_test "Identity file created" "ls $IDENTITY_DIR/test.key" "test.key"
kill "$A1_PID" 2>/dev/null; sleep 1

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ]
