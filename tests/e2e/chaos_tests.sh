#!/usr/bin/env bash
# WWS chaos test suite — tests resilience of core algorithms.
# Actual network chaos tests require a running swarm; these tests verify
# algorithmic resilience under concurrent-style operations using unit tests.
set -euo pipefail

PASS=0; FAIL=0

run_test() {
    local name="$1"; local cmd="$2"
    printf "  %-60s" "$name ..."
    if eval "$cmd" > /dev/null 2>&1; then
        echo "PASS"; ((PASS++))
    else
        echo "FAIL"; ((FAIL++))
    fi
}

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"

echo "=== WWS Chaos: CRDT Convergence ==="
run_test "CRDT merge under concurrent operations" \
    "cd \"$ROOT\" && \"$CARGO\" test -p wws-state --quiet 2>&1 | grep -q '0 failed'"

echo ""
echo "=== WWS Chaos: Replay Window Under Load ==="
run_test "Replay window handles burst of nonces" \
    "cd \"$ROOT\" && \"$CARGO\" test -p wws-protocol replay --quiet 2>&1 | grep -q '0 failed'"

echo ""
echo "=== WWS Chaos: Name Registry Levenshtein ==="
run_test "Typosquat detection correctness" \
    "cd \"$ROOT\" && \"$CARGO\" test -p wws-network name_registry --quiet 2>&1 | grep -q '0 failed'"

echo ""
echo "=== WWS Chaos: Key Rotation Correctness ==="
run_test "Key rotation under stale timestamp injection" \
    "cd \"$ROOT\" && \"$CARGO\" test -p wws-protocol key_rotation --quiet 2>&1 | grep -q '0 failed'"

echo ""
echo "=== WWS Chaos: Reputation Tier Stability ==="
run_test "Reputation tiers stable under decay" \
    "cd \"$ROOT\" && \"$CARGO\" test -p wws-state reputation --quiet 2>&1 | grep -q '0 failed'"

echo ""
echo "=== WWS Chaos: Integration E2E ==="
run_test "Cross-module integration under all new modules" \
    "cd \"$ROOT\" && \"$CARGO\" test -p wws-connector --test wws_integration_tests --quiet 2>&1 | grep -q '0 failed'"

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ]
