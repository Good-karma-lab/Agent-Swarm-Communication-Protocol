#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$ROOT_DIR"

echo "[e2e] Running deterministic connector scenarios"
bash "$SCRIPT_DIR/connector_scenarios.sh"

echo "[e2e] Running operator console network propagation test"
cargo test -p openswarm-connector console_inject_task_publishes_to_swarm >/dev/null

if [[ "${E2E_LIVE_LLM:-0}" == "1" ]]; then
    echo "[e2e] Running live ZeroClaw + OpenRouter E2E"
    bash "$SCRIPT_DIR/zeroclaw_openrouter_live.sh"
else
    echo "[e2e] Skipping live LLM test (set E2E_LIVE_LLM=1 to enable)"
fi

if [[ "${E2E_FAULT:-0}" == "1" ]]; then
    echo "[e2e] Running fault-injection E2E"
    bash "$SCRIPT_DIR/fault_injection.sh"
else
    echo "[e2e] Skipping fault-injection test (set E2E_FAULT=1 to enable)"
fi

if [[ "${E2E_SOAK:-0}" == "1" ]]; then
    echo "[e2e] Running soak E2E"
    bash "$SCRIPT_DIR/soak.sh"
else
    echo "[e2e] Skipping soak test (set E2E_SOAK=1 to enable)"
fi

echo "[e2e] All selected tests passed"
