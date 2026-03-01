#!/usr/bin/env bash
# WWS performance benchmarks
# Measures: PN-Counter merge speed, name registry PoW timing, full suite baseline.
set -euo pipefail

echo "=== WWS Performance Benchmarks ==="
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"

echo ""
echo "--- PN-Counter CRDT merge (all pn_counter tests) ---"
time (cd "$ROOT" && "$CARGO" test -p wws-state pn_counter --release --quiet 2>&1 | tail -3)

echo ""
echo "--- Name registry algorithms (PoW, Levenshtein, TTL) ---"
time (cd "$ROOT" && "$CARGO" test -p wws-network name_registry --release --quiet 2>&1 | tail -3)

echo ""
echo "--- Replay window burst handling ---"
time (cd "$ROOT" && "$CARGO" test -p wws-protocol replay --release --quiet 2>&1 | tail -3)

echo ""
echo "--- Key rotation cryptographic ops ---"
time (cd "$ROOT" && "$CARGO" test -p wws-protocol key_rotation --release --quiet 2>&1 | tail -3)

echo ""
echo "--- Full test suite (baseline) ---"
time (cd "$ROOT" && "$CARGO" test --workspace --release --quiet 2>&1 | tail -3)

echo ""
echo "Benchmarks complete."
