#!/usr/bin/env bash
set -euo pipefail
LISTEN="${1:-/ip4/0.0.0.0/tcp/9000}"
exec ./target/release/wws-connector \
  --bootstrap-mode \
  --listen "$LISTEN" \
  --agent-name "wws-bootstrap" \
  -v
