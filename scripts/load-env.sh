#!/usr/bin/env bash

set -euo pipefail

LOAD_ENV_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOAD_ENV_ROOT_DIR="$(cd "$LOAD_ENV_SCRIPT_DIR/.." && pwd)"

if [ -f "$LOAD_ENV_ROOT_DIR/.env" ]; then
    _pre_env_snapshot="$(mktemp)"
    env > "$_pre_env_snapshot"
    set -a
    # shellcheck disable=SC1091
    source "$LOAD_ENV_ROOT_DIR/.env"
    set +a

    # Preserve explicitly provided environment overrides from the caller.
    while IFS='=' read -r _k _v; do
        [ -n "$_k" ] || continue
        export "$_k=$_v"
    done < "$_pre_env_snapshot"
    rm -f "$_pre_env_snapshot"
fi

if [ -f "$LOAD_ENV_ROOT_DIR/openswarm.conf" ]; then
    # shellcheck disable=SC1091
    source "$LOAD_ENV_ROOT_DIR/openswarm.conf"
fi
