#!/usr/bin/env bash

set -euo pipefail

LOAD_ENV_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOAD_ENV_ROOT_DIR="$(cd "$LOAD_ENV_SCRIPT_DIR/.." && pwd)"

if [ -f "$LOAD_ENV_ROOT_DIR/.env" ]; then
    set -a
    # shellcheck disable=SC1091
    source "$LOAD_ENV_ROOT_DIR/.env"
    set +a
fi

if [ -f "$LOAD_ENV_ROOT_DIR/openswarm.conf" ]; then
    # shellcheck disable=SC1091
    source "$LOAD_ENV_ROOT_DIR/openswarm.conf"
fi
