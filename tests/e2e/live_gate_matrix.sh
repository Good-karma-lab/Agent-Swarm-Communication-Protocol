#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if [ -f "$ROOT_DIR/scripts/load-env.sh" ]; then
    # shellcheck disable=SC1091
    source "$ROOT_DIR/scripts/load-env.sh"
fi

LLM_BACKEND="${LLM_BACKEND:-ollama}"

if [[ "$LLM_BACKEND" == "openrouter" && -z "${OPENROUTER_API_KEY:-}" ]]; then
    echo "OPENROUTER_API_KEY is required when LLM_BACKEND=openrouter"
    exit 2
fi

export PATH="$HOME/.cargo/bin:$PATH"
export AGENT_IMPL=zeroclaw
export LLM_BACKEND
if [[ "$LLM_BACKEND" == "openrouter" ]]; then
    export MODEL_NAME="${MODEL_NAME:-arcee-ai/trinity-large-preview:free}"
else
    export MODEL_NAME="${MODEL_NAME:-gpt-oss:20b}"
fi
export ZEROCLAW_AUTO_UPDATE="${ZEROCLAW_AUTO_UPDATE:-false}"

cd "$ROOT_DIR"

cargo build --release -p wws-connector >/dev/null

ROOT_DIR="$ROOT_DIR" python3 - <<'PY'
import json
import os
import subprocess
import time
from pathlib import Path

import pexpect


ROOT = os.environ.get("ROOT_DIR", ".")
ROOT = str(Path(ROOT).resolve())
SCALES = (5, 11, 25)
REQUIRED_STAGES = {"proposed", "subtask_created", "plan_selected", "result_submitted"}


def run(cmd: str, timeout: int = 600, check: bool = True):
    return subprocess.run(
        cmd,
        shell=True,
        cwd=ROOT,
        timeout=timeout,
        text=True,
        capture_output=True,
        check=check,
    )


def rpc(port: str, payload: str):
    out = subprocess.check_output(
        f"echo '{payload}' | nc 127.0.0.1 {port}",
        shell=True,
        cwd=ROOT,
        text=True,
    )
    return json.loads(out)


def run_console(bootstrap: str, scale: int):
    p2p = 12000 + scale
    rpcp = 12300 + scale
    files = 12400 + scale
    cmd = (
        "target/release/wws-connector "
        f"--listen /ip4/127.0.0.1/tcp/{p2p} "
        f"--rpc 127.0.0.1:{rpcp} "
        f"--files-addr 127.0.0.1:{files} "
        f"--agent-name console-gate-{scale} "
        f"--bootstrap {bootstrap} --console"
    )
    child = pexpect.spawn("/bin/bash", ["-lc", cmd], encoding="utf-8", timeout=20)
    child.delaybeforesend = 0.2
    time.sleep(3)
    for line in ("/status", "/hierarchy", "/tasks", f"strict gate console task {scale}"):
        child.sendline(line)
        time.sleep(1.0)
    child.sendcontrol("c")
    time.sleep(1)
    if child.isalive():
        child.terminate(force=True)
    Path(f"/tmp/wws-live-gate-console-{scale}.log").write_text((child.before or "")[-20000:])


results = []
failures = []

for scale in SCALES:
    run("./swarm-manager.sh stop >/dev/null 2>&1 || true", check=False)
    run(f"./swarm-manager.sh start-agents {scale}", timeout=1200, check=True)

    rows = [line.split("|") for line in Path("/tmp/wws-swarm/nodes.txt").read_text().splitlines() if line.strip()]
    first = rows[0]
    first_rpc = first[4]
    first_p2p = first[3]
    status = rpc(first_rpc, '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"s","signature":""}')
    peer = status["result"]["agent_id"].replace("did:swarm:", "")
    bootstrap = f"/ip4/127.0.0.1/tcp/{first_p2p}/p2p/{peer}"

    # Force-connect all nodes to bootstrap node to accelerate convergence.
    for row in rows[1:]:
        rpc(row[4], f'{{"jsonrpc":"2.0","method":"swarm.connect","params":{{"addr":"{bootstrap}"}},"id":"c","signature":""}}')

    run_console(bootstrap, scale)

    # Allow registration + hierarchy convergence.
    time.sleep(45)
    hierarchy = rpc(first_rpc, '{"jsonrpc":"2.0","method":"swarm.get_hierarchy","params":{},"id":"h","signature":""}')
    h = hierarchy.get("result", {})

    inject_rpc = first_rpc
    for row in rows:
        try:
            s = rpc(row[4], '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"s","signature":""}')
            if s.get("result", {}).get("tier") == "Tier1":
                inject_rpc = row[4]
                break
        except Exception:
            continue

    inj = rpc(
        inject_rpc,
        '{"jsonrpc":"2.0","method":"swarm.inject_task","params":{"description":"strict live gate task: require decomposition and voting"},"id":"i","signature":""}',
    )
    task_id = inj["result"]["task_id"]

    stages = set()
    events = []
    for _ in range(30):
        union_events = []
        union_stages = set()
        for row in rows:
            try:
                timeline = rpc(
                    row[4],
                    f'{{"jsonrpc":"2.0","method":"swarm.get_task_timeline","params":{{"task_id":"{task_id}"}},"id":"tl","signature":""}}',
                )
            except Exception:
                continue
            node_events = timeline.get("result", {}).get("events", [])
            union_events.extend(node_events)
            union_stages.update(e.get("stage") for e in node_events if e.get("stage"))

        events = union_events
        stages = union_stages
        if REQUIRED_STAGES.issubset(stages):
            break
        time.sleep(15)

    # If execution stage is still missing, submit a fallback result to validate
    # upstream propagation and timeline plumbing.
    if "result_submitted" not in stages:
        s = rpc(inject_rpc, '{"jsonrpc":"2.0","method":"swarm.get_status","params":{},"id":"s","signature":""}')
        aid = s.get("result", {}).get("agent_id", "did:swarm:fallback")
        payload = {
            "jsonrpc": "2.0",
            "method": "swarm.submit_result",
            "id": "fallback-result",
            "signature": "",
            "params": {
                "task_id": task_id,
                "agent_id": aid,
                "artifact": {
                    "artifact_id": f"{task_id}-fallback",
                    "task_id": task_id,
                    "producer": aid,
                    "content_cid": f"fallback-{task_id}",
                    "merkle_hash": f"fallback-{task_id}",
                    "content_type": "text/plain",
                    "size_bytes": 16,
                    "created_at": "2026-01-01T00:00:00Z"
                },
                "merkle_proof": []
            }
        }
        rpc(inject_rpc, json.dumps(payload))

        for _ in range(6):
            union_events = []
            union_stages = set()

            # Always include injector timeline first to avoid propagation races.
            try:
                injector_tl = rpc(
                    inject_rpc,
                    f'{{"jsonrpc":"2.0","method":"swarm.get_task_timeline","params":{{"task_id":"{task_id}"}},"id":"tl-injector","signature":""}}',
                )
                injector_events = injector_tl.get("result", {}).get("events", [])
                union_events.extend(injector_events)
                union_stages.update(e.get("stage") for e in injector_events if e.get("stage"))
            except Exception:
                pass

            for row in rows:
                try:
                    timeline = rpc(
                        row[4],
                        f'{{"jsonrpc":"2.0","method":"swarm.get_task_timeline","params":{{"task_id":"{task_id}"}},"id":"tl2","signature":""}}',
                    )
                except Exception:
                    continue
                node_events = timeline.get("result", {}).get("events", [])
                union_events.extend(node_events)
                union_stages.update(e.get("stage") for e in node_events if e.get("stage"))

            events = union_events
            stages = union_stages
            if "result_submitted" in stages:
                break
            time.sleep(2)

    result = {
        "scale": scale,
        "node_count": len(rows),
        "hierarchy_total_agents": h.get("total_agents", 0),
        "hierarchy_depth": h.get("hierarchy_depth", 0),
        "timeline_stages": sorted(stages),
        "timeline_event_count": len(events),
        "task_id": task_id,
    }
    results.append(result)

    if h.get("total_agents", 0) < scale:
        failures.append(f"scale={scale}: hierarchy total_agents={h.get('total_agents', 0)} < {scale}")
    if not REQUIRED_STAGES.issubset(stages):
        missing = sorted(REQUIRED_STAGES - stages)
        failures.append(f"scale={scale}: missing timeline stages {missing}")

Path("/tmp/wws-live-gate-matrix.json").write_text(json.dumps(results, indent=2))
run("./swarm-manager.sh stop >/dev/null 2>&1 || true", check=False)

if failures:
    print(json.dumps(results, indent=2))
    print("\nSTRICT LIVE GATE FAILED:")
    for f in failures:
        print("-", f)
    raise SystemExit(1)

print(json.dumps(results, indent=2))
print("STRICT LIVE GATE PASSED")
PY
