#!/usr/bin/env bash

set -euo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:22971}"
SUBMIT_RPC_PORT="${SUBMIT_RPC_PORT:-22970}"

echo "[validate] Base URL: $BASE_URL"
echo "[validate] Submit RPC port: $SUBMIT_RPC_PORT"

BASE_URL="$BASE_URL" SUBMIT_RPC_PORT="$SUBMIT_RPC_PORT" python3 - <<'PY'
import datetime
import hashlib
import json
import os
import pathlib
import subprocess
import sys
import time
import urllib.request
import uuid

BASE_URL = os.environ["BASE_URL"]
SUBMIT_RPC_PORT = os.environ["SUBMIT_RPC_PORT"]

def get(path):
    with urllib.request.urlopen(BASE_URL + path, timeout=20) as r:
        return json.loads(r.read().decode())

def post(path, payload):
    req = urllib.request.Request(
        BASE_URL + path,
        data=json.dumps(payload).encode(),
        headers={"content-type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=20) as r:
        return json.loads(r.read().decode())

def rpc(port, payload):
    cmd = json.dumps(payload)
    out = subprocess.check_output(
        ["bash", "-lc", f"printf '%s' '{cmd}' | nc 127.0.0.1 {port}"], timeout=8
    ).decode()
    return json.loads(out)

health = get("/api/health")
if not health.get("ok"):
    raise SystemExit("[validate] health check failed")

hierarchy = get("/api/hierarchy")
nodes = hierarchy.get("nodes", [])
if not nodes:
    raise SystemExit("[validate] hierarchy nodes are empty")

did_names = [n for n in nodes if str(n.get("agent_name", "")).startswith("did:swarm:")]
if did_names:
    raise SystemExit("[validate] hierarchy still contains DID as agent_name")

topology = get("/api/topology")
if not any(n.get("id") == "zero0" for n in topology.get("nodes", [])):
    raise SystemExit("[validate] topology missing zero0 root node")

root_edges = [e for e in topology.get("edges", []) if e.get("kind") == "root_hierarchy"]
if not root_edges:
    raise SystemExit("[validate] topology missing root_hierarchy edges")

ports = []
for line in pathlib.Path("/tmp/wws-swarm/nodes.txt").read_text().splitlines():
    parts = line.split("|")
    if len(parts) >= 5:
        ports.append(parts[4])

tier1_rpcs = []
for port in ports:
    try:
        status = rpc(
            port,
            {"jsonrpc": "2.0", "method": "swarm.get_status", "params": {}, "id": "s", "signature": ""},
        )
        if status.get("result", {}).get("tier") == "Tier1":
            tier1_rpcs.append(port)
    except Exception:
        continue

tier1_rpcs = sorted(set(tier1_rpcs))
if not tier1_rpcs:
    raise SystemExit("[validate] could not find any Tier1 RPC nodes")

injected = post("/api/tasks", {"description": "comprehensive validation task"})
task_id = injected.get("task_id")
if not task_id:
    raise SystemExit("[validate] task injection did not return task_id")

proposed_plan_ids = []
for idx, tier1_rpc in enumerate(tier1_rpcs, start=1):
    plan = {
        "plan_id": str(uuid.uuid4()),
        "task_id": task_id,
        "proposer": "did:swarm:placeholder",
        "epoch": 1,
        "subtasks": [
            {"index": 1, "description": f"Collect options ({idx})", "required_capabilities": ["research"], "estimated_complexity": 0.34},
            {"index": 2, "description": f"Evaluate options ({idx})", "required_capabilities": ["analysis"], "estimated_complexity": 0.33},
            {"index": 3, "description": f"Write recommendation ({idx})", "required_capabilities": ["writing"], "estimated_complexity": 0.33},
        ],
        "rationale": f"Validate recursive decomposition pipeline proposer-{idx}",
        "estimated_parallelism": 3,
        "created_at": datetime.datetime.now(datetime.timezone.utc).isoformat().replace("+00:00", "Z"),
    }

    proposed = rpc(
        tier1_rpc,
        {"jsonrpc": "2.0", "method": "swarm.propose_plan", "params": plan, "id": f"p{idx}", "signature": ""},
    )
    if proposed.get("error"):
        message = str(proposed["error"].get("message", ""))
        if "Not in commit phase" in message:
            continue
        raise SystemExit(f"[validate] propose_plan failed on {tier1_rpc}: {proposed['error']}")
    proposed_plan_ids.append(plan["plan_id"])

if not proposed_plan_ids:
    raise SystemExit("[validate] no plan proposal was accepted")

rankings = proposed_plan_ids[:]
for idx, tier1_rpc in enumerate(tier1_rpcs, start=1):
    vote_error = None
    for _ in range(20):
        voted = rpc(
            tier1_rpc,
            {
                "jsonrpc": "2.0",
                "method": "swarm.submit_vote",
                "params": {"task_id": task_id, "rankings": rankings, "epoch": 1},
                "id": f"v{idx}",
                "signature": "",
            },
        )
        if not voted.get("error"):
            vote_error = None
            break
        vote_error = voted["error"]
        message = str(vote_error.get("message", ""))
        if "No valid proposals in rankings" in message:
            time.sleep(1)
            continue
        break

    if vote_error is not None:
        raise SystemExit(f"[validate] submit_vote failed on {tier1_rpc}: {vote_error}")

subtasks = []
for _ in range(25):
    tasks = get("/api/tasks").get("tasks", [])
    root = next((t for t in tasks if t.get("task_id") == task_id), None)
    if root:
        subtasks = sorted([s for s in root.get("subtasks", []) if "-st-" in s])
    if len(subtasks) >= 3:
        break
    time.sleep(1)

if len(subtasks) < 3:
    raise SystemExit("[validate] subtasks were not materialized")

for subtask_id in subtasks:
    content = f"done {subtask_id}"
    h = hashlib.sha256(content.encode()).hexdigest()
    submission = {
        "jsonrpc": "2.0",
        "method": "swarm.submit_result",
        "id": "r",
        "signature": "",
        "params": {
            "task_id": subtask_id,
            "agent_id": "did:swarm:placeholder",
            "artifact": {
                "artifact_id": f"{subtask_id}-artifact",
                "task_id": subtask_id,
                "producer": "did:swarm:placeholder",
                "content_cid": h,
                "merkle_hash": h,
                "content_type": "text/plain",
                "size_bytes": len(content),
                "created_at": datetime.datetime.now(datetime.timezone.utc).isoformat().replace("+00:00", "Z"),
            },
            "merkle_proof": [],
        },
    }
    submitted = rpc(SUBMIT_RPC_PORT, submission)
    if submitted.get("error"):
        raise SystemExit(f"[validate] submit_result failed for {subtask_id}: {submitted['error']}")

root_task = None
for _ in range(25):
    tasks = get("/api/tasks").get("tasks", [])
    root_task = next((t for t in tasks if t.get("task_id") == task_id), None)
    if root_task and root_task.get("status") == "Completed":
        break
    time.sleep(1)

if not root_task or root_task.get("status") != "Completed":
    raise SystemExit("[validate] root task did not complete")

if not root_task.get("has_result"):
    raise SystemExit("[validate] root task is completed but has_result is false")

voting = get("/api/voting")
if len(voting.get("rfp", [])) == 0:
    raise SystemExit("[validate] voting.rfp is empty")

audit = get("/api/audit")
if len(audit.get("events", [])) == 0:
    raise SystemExit("[validate] audit events are empty")

flow = get("/api/flow")
required_flow = {"injected", "proposal_commit", "proposal_reveal", "result_submitted"}
if not required_flow.issubset(set(flow.get("counters", {}).keys())):
    raise SystemExit("[validate] flow counters missing expected stages")

timeline = get(f"/api/tasks/{task_id}/timeline")
if len(timeline.get("descendants", [])) == 0:
    raise SystemExit("[validate] descendants are empty")

print("[validate] comprehensive validation PASSED")
print(f"[validate] task_id={task_id}")
PY
