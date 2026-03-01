#!/usr/bin/env bash
# WWS adversarial test suite
# Tests replay protection, PoW difficulty enforcement, and name squatting prevention
# via unit-test runners (full network adversarial tests require a live swarm).
set -euo pipefail

PASS=0; FAIL=0

run_test() {
    local name="$1"; local result="$2"
    printf "  %-60s" "$name ..."
    if [ "$result" = "pass" ]; then
        echo "PASS"; ((PASS++))
    else
        echo "FAIL: $result"; ((FAIL++))
    fi
}

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"

echo "=== Adversarial: Replay Window ==="
cargo_result=$(cd "$ROOT" && \
    "$CARGO" test -p wws-protocol replay --quiet 2>&1 | tail -3)
if echo "$cargo_result" | grep -q "0 failed"; then
    run_test "Replay protection unit tests" "pass"
else
    run_test "Replay protection unit tests" "$cargo_result"
fi

echo ""
echo "=== Adversarial: Name Registry PoW ==="
cargo_result=$(cd "$ROOT" && \
    "$CARGO" test -p wws-network name_registry --quiet 2>&1 | tail -3)
if echo "$cargo_result" | grep -q "0 failed"; then
    run_test "Name registry PoW difficulty tests" "pass"
else
    run_test "Name registry PoW difficulty tests" "$cargo_result"
fi

echo ""
echo "=== Adversarial: Typosquat Detection ==="
cargo_result=$(cd "$ROOT" && \
    "$CARGO" test -p wws-network test_typosquat_detection --quiet 2>&1 | tail -3)
if echo "$cargo_result" | grep -q "0 failed"; then
    run_test "Typosquat detection (Levenshtein guard)" "pass"
else
    run_test "Typosquat detection (Levenshtein guard)" "$cargo_result"
fi

echo ""
echo "=== Adversarial: Guardian Recovery Threshold ==="
cargo_result=$(cd "$ROOT" && \
    "$CARGO" test -p wws-protocol guardian --quiet 2>&1 | tail -3)
if echo "$cargo_result" | grep -q "0 failed"; then
    run_test "Guardian threshold enforcement tests" "pass"
else
    run_test "Guardian threshold enforcement tests" "$cargo_result"
fi

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ]
