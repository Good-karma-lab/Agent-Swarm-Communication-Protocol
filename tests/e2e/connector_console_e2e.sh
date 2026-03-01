#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[console-e2e] Running console task propagation test"
cargo test -p wws-connector console_inject_task_publishes_to_swarm
