//! HTTP server for onboarding docs, APIs, and web dashboard assets.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, put};
use axum::{Json, Router};
use bytes::Bytes;
use futures_util::stream;
use serde::Deserialize;
use tokio::sync::RwLock;
use tower_http::services::{ServeDir, ServeFile};

use wws_protocol::Tier;

use crate::connector::{ConnectorState, MessageTraceEvent};

const ACTIVE_MEMBER_STALENESS_SECS: u64 = 45;

struct EmbeddedDocs {
    skill_md: &'static str,
    heartbeat_md: &'static str,
    messaging_md: &'static str,
}

static DOCS: EmbeddedDocs = EmbeddedDocs {
    skill_md: include_str!("../../../docs/SKILL.md"),
    heartbeat_md: include_str!("../../../docs/HEARTBEAT.md"),
    messaging_md: include_str!("../../../docs/MESSAGING.md"),
};

#[derive(Clone)]
struct WebState {
    state: Arc<RwLock<ConnectorState>>,
    network_handle: wws_network::SwarmHandle,
}

pub struct FileServer {
    bind_addr: String,
    state: Arc<RwLock<ConnectorState>>,
    network_handle: wws_network::SwarmHandle,
    web_root: PathBuf,
}

impl FileServer {
    pub fn new(
        bind_addr: String,
        state: Arc<RwLock<ConnectorState>>,
        network_handle: wws_network::SwarmHandle,
    ) -> Self {
        Self {
            bind_addr,
            state,
            network_handle,
            web_root: detect_web_root(),
        }
    }

    pub async fn run(self) -> Result<(), anyhow::Error> {
        let web_state = WebState {
            state: self.state,
            network_handle: self.network_handle,
        };

        let web_root = self.web_root.clone();
        let index_file = web_root.join("index.html");
        let static_service = ServeDir::new(web_root)
            .not_found_service(ServeFile::new(index_file));

        let app = Router::new()
            .route("/SKILL.md", get(skill_md))
            .route("/HEARTBEAT.md", get(heartbeat_md))
            .route("/MESSAGING.md", get(messaging_md))
            .route("/agent-onboarding.json", get(onboarding))
            .route("/api/health", get(api_health))
            .route("/api/auth-status", get(api_auth_status))
            .route("/api/identity", get(api_identity))
            .route("/api/reputation", get(api_reputation))
            .route("/api/reputation/events", get(api_reputation_events))
            .route("/api/names", get(api_names).post(api_names_register))
            .route("/api/names/:name/renew", put(api_names_renew))
            .route("/api/names/:name", delete(api_names_release))
            .route("/api/network", get(api_network))
            .route("/api/peers", get(api_peers))
            .route("/api/directory", get(api_directory))
            .route("/api/keys", get(api_keys))
            .route("/api/graph", get(api_graph))
            .route("/api/events", get(api_events))
            .route("/api/hierarchy", get(api_hierarchy))
            .route("/api/voting", get(api_voting))
            .route("/api/voting/:task_id", get(api_voting_task))
            .route("/api/messages", get(api_messages))
            .route("/api/messages/:task_id", get(api_messages_task))
            .route("/api/tasks", get(api_tasks).post(api_submit_task))
            .route("/api/tasks/:task_id/timeline", get(api_task_timeline))
            .route("/api/tasks/:task_id/deliberation", get(api_task_deliberation))
            .route("/api/tasks/:task_id/ballots", get(api_task_ballots))
            .route("/api/tasks/:task_id/irv-rounds", get(api_task_irv_rounds))
            .route("/api/holons", get(api_holons))
            .route("/api/holons/:task_id", get(api_holon_detail))
            .route("/api/agents", get(api_agents))
            .route("/api/topology", get(api_topology))
            .route("/api/flow", get(api_flow))
            .route("/api/audit", get(api_audit))
            .route("/api/ui-recommendations", get(api_ui_recommendations))
            .route("/api/stream", get(api_stream))
            .fallback_service(static_service)
            .with_state(web_state);

        let listener = tokio::net::TcpListener::bind(&self.bind_addr).await?;
        tracing::info!(
            addr = %self.bind_addr,
            web_root = %self.web_root.display(),
            "HTTP web dashboard listening"
        );
        axum::serve(listener, app).await?;
        Ok(())
    }
}

fn detect_web_root() -> PathBuf {
    if let Ok(path) = std::env::var("WWS_WEBAPP_DIR") {
        let p = PathBuf::from(path);
        if p.join("index.html").exists() {
            return p;
        }
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let candidates = [
        cwd.join("webapp/dist"),
        cwd.join("dist"),
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../webapp/dist")
            .to_path_buf(),
    ];

    for c in candidates {
        if c.join("index.html").exists() {
            return c;
        }
    }

    cwd
}

async fn skill_md() -> impl IntoResponse {
    ([("content-type", "text/markdown; charset=utf-8")], DOCS.skill_md)
}

async fn heartbeat_md() -> impl IntoResponse {
    (
        [("content-type", "text/markdown; charset=utf-8")],
        DOCS.heartbeat_md,
    )
}

async fn messaging_md() -> impl IntoResponse {
    (
        [("content-type", "text/markdown; charset=utf-8")],
        DOCS.messaging_md,
    )
}

async fn onboarding() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "WWS.Connector",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": "JSON-RPC 2.0",
        "rpc_default_port": 9370,
        "files_default_port": 9371,
        "dashboard": "/",
        "methods": [
            "swarm.get_status",
            "swarm.receive_task",
            "swarm.get_task",
            "swarm.get_task_timeline",
            "swarm.register_agent",
            "swarm.propose_plan",
            "swarm.submit_vote",
            "swarm.get_voting_state",
            "swarm.submit_result",
            "swarm.connect",
            "swarm.get_network_stats",
            "swarm.inject_task",
            "swarm.get_hierarchy",
            "swarm.list_swarms",
            "swarm.create_swarm",
            "swarm.join_swarm"
        ]
    }))
}

async fn api_health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok": true, "service": "wws-web"}))
}

async fn api_auth_status() -> Json<serde_json::Value> {
    let token_required = std::env::var("WWS_WEB_TOKEN")
        .ok()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    Json(serde_json::json!({"token_required": token_required}))
}

async fn api_hierarchy(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let active = collect_known_members(&s);

    let mut nodes = Vec::new();
    for agent_id in active {
        let tier = s
            .agent_tiers
            .get(&agent_id)
            .cloned()
            .unwrap_or(if agent_id == s.agent_id.to_string() {
                s.my_tier
            } else {
                Tier::Executor
            });
        let parent_id = s.agent_parents.get(&agent_id).cloned();
        let task_count = s
            .task_details
            .values()
            .filter(|t| t.assigned_to.as_ref().map(|a| a.to_string()) == Some(agent_id.clone()))
            .count();

        let last_seen_secs = s.member_last_seen.get(&agent_id).map(|ts| {
            chrono::Utc::now()
                .signed_duration_since(*ts)
                .num_seconds()
                .max(0)
        });

        nodes.push(serde_json::json!({
            "agent_id": agent_id,
            "agent_name": s.agent_names.get(&agent_id).cloned().unwrap_or_else(|| short_agent_label(&agent_id)),
            "tier": format!("{:?}", tier),
            "parent_id": parent_id,
            "task_count": task_count,
            "last_seen_secs": last_seen_secs,
            "is_self": agent_id == s.agent_id.to_string(),
        }));
    }

    Json(serde_json::json!({
        "generated_at": chrono::Utc::now(),
        "self_agent": s.agent_id.to_string(),
        "nodes": nodes,
    }))
}

async fn api_voting(State(web): State<WebState>) -> Json<serde_json::Value> {
    Json(voting_payload(&web.state, None).await)
}

async fn api_voting_task(
    State(web): State<WebState>,
    AxumPath(task_id): AxumPath<String>,
) -> Json<serde_json::Value> {
    Json(voting_payload(&web.state, Some(task_id)).await)
}

async fn voting_payload(
    state: &Arc<RwLock<ConnectorState>>,
    task_filter: Option<String>,
) -> serde_json::Value {
    let s = state.read().await;

    let voting = s
        .voting_engines
        .iter()
        .filter(|(task_id, _)| task_filter.as_ref().map(|t| t == *task_id).unwrap_or(true))
        .map(|(task_id, v)| {
            let req = s.task_vote_requirements.get(task_id);
            let tier_level = req.map(|r| r.tier_level).unwrap_or(1);
            let tier = tier_from_level(tier_level);
            let expected_voters = req
                .map(|r| r.expected_voters)
                .unwrap_or_else(|| {
                    collect_known_members(&s)
                        .into_iter()
                        .filter(|id| s.agent_tiers.get(id).copied().unwrap_or(Tier::Executor) == tier)
                        .count()
                });
            let tier_members: Vec<String> = collect_known_members(&s)
                .into_iter()
                .filter(|id| s.agent_tiers.get(id).copied().unwrap_or(Tier::Executor) == tier)
                .collect();
            let voter_ids = v.voter_ids_for_debug();
            let missing_voter_names = tier_members
                .into_iter()
                .filter(|id| !voter_ids.iter().any(|voter| voter == id))
                .map(|id| s.agent_names.get(&id).cloned().unwrap_or_else(|| short_agent_label(&id)))
                .collect::<Vec<_>>();
            serde_json::json!({
                "task_id": task_id,
                "proposal_count": v.proposal_count(),
                "ballot_count": v.ballot_count(),
                "finalized": v.is_finalized(),
                "expected_voters": expected_voters,
                "missing_voter_names": missing_voter_names,
            })
        })
        .collect::<Vec<_>>();

    let rfp = s
        .rfp_coordinators
        .iter()
        .filter(|(task_id, _)| task_filter.as_ref().map(|t| t == *task_id).unwrap_or(true))
        .map(|(task_id, r)| {
            let req = s.task_vote_requirements.get(task_id);
            let tier_level = req.map(|r| r.tier_level).unwrap_or(1);
            let tier = tier_from_level(tier_level);
            let expected_proposers = req
                .map(|r| r.expected_proposers)
                .unwrap_or_else(|| {
                    collect_known_members(&s)
                        .into_iter()
                        .filter(|id| s.agent_tiers.get(id).copied().unwrap_or(Tier::Executor) == tier)
                        .count()
                });
            let tier_members: Vec<String> = collect_known_members(&s)
                .into_iter()
                .filter(|id| s.agent_tiers.get(id).copied().unwrap_or(Tier::Executor) == tier)
                .collect();

            let commit_ids = r
                .commits_for_debug()
                .iter()
                .map(|(agent, _)| agent.clone())
                .collect::<Vec<_>>();
            let missing_proposer_names = tier_members
                .into_iter()
                .filter(|id| !commit_ids.iter().any(|c| c == id))
                .map(|id| s.agent_names.get(&id).cloned().unwrap_or_else(|| short_agent_label(&id)))
                .collect::<Vec<_>>();

            let plans = r
                .reveals
                .values()
                .map(|p| {
                    let proposer_id = p.proposer.to_string();
                    serde_json::json!({
                        "proposer": proposer_id,
                        "proposer_name": s
                            .agent_names
                            .get(&p.proposer.to_string())
                            .cloned()
                            .unwrap_or_else(|| short_agent_label(&p.proposer.to_string())),
                        "plan_id": p.plan.plan_id,
                        "plan_hash": p.plan_hash,
                        "rationale": p.plan.rationale,
                        "subtask_count": p.plan.subtasks.len(),
                    })
                })
                .collect::<Vec<_>>();

            let (missing_voter_names, expected_voters) = if let Some(v) = s.voting_engines.get(task_id) {
                let req = s.task_vote_requirements.get(task_id);
                let expected_voters = req.map(|rr| rr.expected_voters).unwrap_or(0);
                let tier_members: Vec<String> = collect_known_members(&s)
                    .into_iter()
                    .filter(|id| s.agent_tiers.get(id).copied().unwrap_or(Tier::Executor) == tier)
                    .collect();
                let voter_ids = v.voter_ids_for_debug();
                (
                    tier_members
                        .into_iter()
                        .filter(|id| !voter_ids.iter().any(|vv| vv == id))
                        .map(|id| s.agent_names.get(&id).cloned().unwrap_or_else(|| short_agent_label(&id)))
                        .collect::<Vec<_>>(),
                    expected_voters,
                )
            } else {
                (Vec::new(), 0)
            };
            serde_json::json!({
                "task_id": task_id,
                "phase": format!("{:?}", r.phase()),
                "commit_count": r.commit_count(),
                "reveal_count": r.reveal_count(),
                "commits": r.commits_for_debug(),
                "plans": plans,
                "expected_proposers": expected_proposers,
                "expected_voters": expected_voters,
                "missing_proposer_names": missing_proposer_names,
                "missing_voter_names": missing_voter_names,
            })
        })
        .collect::<Vec<_>>();

    serde_json::json!({ "voting": voting, "rfp": rfp })
}

async fn api_messages(State(web): State<WebState>) -> Json<serde_json::Value> {
    Json(messages_payload(&web.state, None).await)
}

async fn api_messages_task(
    State(web): State<WebState>,
    AxumPath(task_id): AxumPath<String>,
) -> Json<serde_json::Value> {
    Json(messages_payload(&web.state, Some(task_id)).await)
}

async fn messages_payload(
    state: &Arc<RwLock<ConnectorState>>,
    task_filter: Option<String>,
) -> serde_json::Value {
    let s = state.read().await;
    let items: Vec<&MessageTraceEvent> = s
        .message_trace
        .iter()
        .rev()
        .filter(|m| {
            is_business_message(m)
                &&
            task_filter
                .as_ref()
                .map(|t| m.task_id.as_ref().map(|id| id == t).unwrap_or(false))
                .unwrap_or(true)
        })
        .take(1000)
        .collect();
    serde_json::to_value(items).unwrap_or_else(|_| serde_json::json!([]))
}

async fn api_tasks(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let mut tasks = s
        .task_details
        .values()
        .cloned()
        .collect::<Vec<_>>();
    tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let tasks = tasks
        .into_iter()
        .map(|task| {
            let result = s.task_results.get(&task.task_id);
            let result_text = s.task_result_text.get(&task.task_id).cloned();
            let assigned_to = task.assigned_to.as_ref().map(|a| a.to_string());
            let assigned_to_name = assigned_to
                .as_ref()
                .map(|id| {
                    s.agent_names
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| short_agent_label(id))
                });
            serde_json::json!({
                "task_id": task.task_id,
                "parent_task_id": task.parent_task_id,
                "description": task.description,
                "status": format!("{:?}", task.status),
                "tier_level": task.tier_level,
                "assigned_to": assigned_to,
                "assigned_to_name": assigned_to_name,
                "subtasks": task.subtasks,
                "created_at": task.created_at,
                "deadline": task.deadline,
                "has_result": result.is_some(),
                "result_artifact": result,
                "result_text": result_text,
            })
        })
        .collect::<Vec<_>>();
    Json(serde_json::json!({"tasks": tasks}))
}

#[derive(Deserialize)]
struct TaskSubmitRequest {
    description: String,
}

async fn api_submit_task(
    State(web): State<WebState>,
    headers: HeaderMap,
    Json(req): Json<TaskSubmitRequest>,
) -> impl IntoResponse {
    if let Ok(required) = std::env::var("WWS_WEB_TOKEN") {
        if !required.trim().is_empty() {
            let provided = headers
                .get("x-ops-token")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            if provided != required {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({"ok": false, "error": "invalid_operator_token"})),
                );
            }
        }
    }

    if req.description.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"ok": false, "error": "missing_description"})),
        );
    }

    let params = serde_json::json!({ "description": req.description });
    let response = crate::rpc_server::handle_inject_task(
        Some("web-submit-task".to_string()),
        &params,
        &web.state,
        &web.network_handle,
    )
    .await;

    if let Some(err) = response.error {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"ok": false, "error": err.message})),
        );
    }

    let result = response.result.unwrap_or_else(|| serde_json::json!({"ok": true}));
    (StatusCode::OK, Json(result))
}

async fn api_task_timeline(
    State(web): State<WebState>,
    AxumPath(task_id): AxumPath<String>,
) -> Json<serde_json::Value> {
    let s = web.state.read().await;

    let timeline = s.task_timelines.get(&task_id).cloned().unwrap_or_default();
    let task = s.task_details.get(&task_id).cloned().map(|t| {
        let assigned_to = t.assigned_to.as_ref().map(|a| a.to_string());
        let assigned_to_name = assigned_to
            .as_ref()
            .map(|id| {
                s.agent_names
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| short_agent_label(id))
            });
        serde_json::json!({
            "task_id": t.task_id,
            "parent_task_id": t.parent_task_id,
            "description": t.description,
            "status": format!("{:?}", t.status),
            "tier_level": t.tier_level,
            "assigned_to": assigned_to,
            "assigned_to_name": assigned_to_name,
            "subtasks": t.subtasks,
            "created_at": t.created_at,
            "deadline": t.deadline,
        })
    });
    let task_result = s.task_results.get(&task_id).cloned();
    let task_result_text = s.task_result_text.get(&task_id).cloned();
    let messages = s
        .message_trace
        .iter()
        .filter(|m| m.task_id.as_ref().map(|id| id == &task_id).unwrap_or(false))
        .cloned()
        .collect::<Vec<_>>();

    let descendants = collect_task_descendants(&task_id, &s.task_details)
        .into_iter()
        .map(|t| {
            let result = s.task_results.get(&t.task_id).cloned();
            let result_text = s.task_result_text.get(&t.task_id).cloned();
            let assigned_to = t.assigned_to.as_ref().map(|a| a.to_string());
            let assigned_to_name = assigned_to
                .as_ref()
                .map(|id| {
                    s.agent_names
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| short_agent_label(id))
                });
            serde_json::json!({
                "task_id": t.task_id,
                "parent_task_id": t.parent_task_id,
                "description": t.description,
                "status": format!("{:?}", t.status),
                "tier_level": t.tier_level,
                "assigned_to": assigned_to,
                "assigned_to_name": assigned_to_name,
                "subtasks": t.subtasks,
                "created_at": t.created_at,
                "deadline": t.deadline,
                "has_result": result.is_some(),
                "result_artifact": result,
                "result_text": result_text,
            })
        })
        .collect::<Vec<_>>();

    Json(serde_json::json!({
        "task": task,
        "result_artifact": task_result,
        "result_text": task_result_text,
        "timeline": timeline,
        "descendants": descendants,
        "messages": messages,
    }))
}

async fn api_agents(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let now = chrono::Utc::now();
    let members = collect_known_members(&s);

    let agents = members
        .into_iter()
        .map(|id| {
            let seen_secs = s
                .member_last_seen
                .get(&id)
                .and_then(|ts| now.signed_duration_since(*ts).to_std().ok())
                .map(|d| d.as_secs());
            let last_task_poll_secs = s
                .member_last_task_poll
                .get(&id)
                .and_then(|ts| now.signed_duration_since(*ts).to_std().ok())
                .map(|d| d.as_secs());
            let last_result_secs = s
                .member_last_result
                .get(&id)
                .and_then(|ts| now.signed_duration_since(*ts).to_std().ok())
                .map(|d| d.as_secs());

            serde_json::json!({
                "agent_id": id,
                "name": s.agent_names.get(&id).cloned().unwrap_or_else(|| short_agent_label(&id)),
                "tier": format!("{:?}", s.agent_tiers.get(&id).copied().unwrap_or(Tier::Executor)),
                "seen_secs": seen_secs,
                "last_task_poll_secs": last_task_poll_secs,
                "last_result_secs": last_result_secs,
                "tasks_assigned_count": s.agent_activity.get(&id).map(|a| a.tasks_assigned_count).unwrap_or(0),
                "tasks_processed_count": s.agent_activity.get(&id).map(|a| a.tasks_processed_count).unwrap_or(0),
                "plans_proposed_count": s.agent_activity.get(&id).map(|a| a.plans_proposed_count).unwrap_or(0),
                "plans_revealed_count": s.agent_activity.get(&id).map(|a| a.plans_revealed_count).unwrap_or(0),
                "votes_cast_count": s.agent_activity.get(&id).map(|a| a.votes_cast_count).unwrap_or(0),
                "is_self": id == s.agent_id.to_string(),
                "connected": seen_secs.map(|v| v <= 60).unwrap_or(false),
                "loop_active": last_task_poll_secs.map(|v| v <= 120).unwrap_or(false),
                "not_responding": last_task_poll_secs.map(|v| v > 180).unwrap_or(true),
            })
        })
        .collect::<Vec<_>>();

    Json(serde_json::json!({ "agents": agents }))
}

fn collect_task_descendants(
    root: &str,
    details: &HashMap<String, wws_protocol::Task>,
) -> Vec<wws_protocol::Task> {
    let mut out = Vec::new();
    let mut frontier = vec![root.to_string()];
    while let Some(parent) = frontier.pop() {
        for task in details.values() {
            if task.parent_task_id.as_deref() == Some(parent.as_str()) {
                out.push(task.clone());
                frontier.push(task.task_id.clone());
            }
        }
    }
    out
}

async fn api_topology(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let members = collect_known_members(&s);

    let mut nodes = members
        .iter()
        .map(|id| {
            serde_json::json!({
                "id": id,
                "name": s.agent_names.get(id).cloned().unwrap_or_else(|| short_agent_label(id)),
                "tier": format!("{:?}", s.agent_tiers.get(id).copied().unwrap_or(Tier::Executor)),
                "is_self": *id == s.agent_id.to_string(),
            })
        })
        .collect::<Vec<_>>();

    nodes.push(serde_json::json!({
        "id": "zero0",
        "name": "zero0",
        "tier": "Root",
        "is_self": false,
    }));

    let mut edges = s
        .agent_parents
        .iter()
        .map(|(child, parent)| {
            serde_json::json!({"source": parent, "target": child, "kind": "hierarchy"})
        })
        .collect::<Vec<_>>();

    for (id, tier) in &s.agent_tiers {
        if *tier == Tier::Tier1 && members.iter().any(|m| m == id) {
            edges.push(serde_json::json!({
                "source": "zero0",
                "target": id,
                "kind": "root_hierarchy"
            }));
        }
    }

    for peer in s.agent_set.elements() {
        edges.push(serde_json::json!({
            "source": s.agent_id.to_string(),
            "target": format!("did:swarm:{}", peer),
            "kind": "peer_link"
        }));
    }

    Json(serde_json::json!({"nodes": nodes, "edges": edges}))
}

fn short_agent_label(agent_id: &str) -> String {
    if let Some(last) = agent_id.split(':').next_back() {
        if last.len() > 12 {
            return last[..12].to_string();
        }
        return last.to_string();
    }
    agent_id.to_string()
}

fn collect_known_members(s: &ConnectorState) -> Vec<String> {
    let mut members: Vec<String> = s
        .agent_tiers
        .keys()
        .cloned()
        .chain(s.member_last_seen.keys().cloned())
        .collect();
    members.push(s.agent_id.to_string());
    members.sort();
    members.dedup();
    members
}

fn tier_from_level(level: u32) -> Tier {
    match level {
        1 => Tier::Tier1,
        2 => Tier::Tier2,
        n => Tier::TierN(n),
    }
}

fn is_business_message(msg: &MessageTraceEvent) -> bool {
    match msg.method.as_deref() {
        Some(method)
            if method.contains("keepalive")
                || method == "swarm.announce"
                || method == "swarm.join"
                || method == "swarm.join_response"
                || method == "swarm.leave"
                || method == "hierarchy.assign_tier" =>
        {
            false
        }
        Some(_) => true,
        None => false,
    }
}

async fn api_flow(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let mut counters: HashMap<String, usize> = HashMap::new();
    for events in s.task_timelines.values() {
        for event in events {
            *counters.entry(event.stage.clone()).or_insert(0) += 1;
        }
    }

    Json(serde_json::json!({
        "counters": counters,
        "active_tasks": s.task_set.len(),
        "voting_engines": s.voting_engines.len(),
        "rfp_rounds": s.rfp_coordinators.len(),
        "message_trace_size": s.message_trace.len(),
    }))
}

async fn api_audit(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let rows = s
        .event_log
        .iter()
        .rev()
        .filter(|e| e.message.starts_with("AUDIT "))
        .take(500)
        .map(|e| {
            serde_json::json!({
                "timestamp": e.timestamp,
                "category": format!("{:?}", e.category),
                "message": e.message,
            })
        })
        .collect::<Vec<_>>();
    Json(serde_json::json!({"events": rows}))
}

async fn api_ui_recommendations() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "recommended_features": [
            "Task SLA panel (stuck task detector + age heatmap)",
            "Election/succession timeline and incident replay",
            "Agent throughput and reliability leaderboard",
            "Topology drift alerts (partition/churn detection)",
            "Task graph playback over time",
            "Exportable forensic bundle per task (plans, votes, logs, artifacts)",
            "Role-based access control and audit log for operator actions"
        ]
    }))
}

async fn api_stream(
    ws: WebSocketUpgrade,
    State(web): State<WebState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| stream_loop(socket, web.state))
}

async fn stream_loop(mut socket: WebSocket, state: Arc<RwLock<ConnectorState>>) {
    let mut interval = tokio::time::interval(Duration::from_secs(2));
    loop {
        interval.tick().await;
        let payload = {
            let s = state.read().await;
            let recent_messages = s
                .message_trace
                .iter()
                .rev()
                .take(40)
                .cloned()
                .collect::<Vec<_>>();
            let recent_events = s
                .event_log
                .iter()
                .rev()
                .take(40)
                .cloned()
                .collect::<Vec<_>>();
            serde_json::json!({
                "type": "snapshot",
                "time": chrono::Utc::now(),
                "active_tasks": s.task_set.len(),
                "known_agents": s.active_member_count(Duration::from_secs(ACTIVE_MEMBER_STALENESS_SECS)),
                "messages": recent_messages,
                "events": recent_events,
            })
            .to_string()
        };

        if socket.send(Message::Text(payload.into())).await.is_err() {
            break;
        }
    }
}

// ── Holonic API Handlers ────────────────────────────────────────────────────

async fn api_holons(State(s): State<WebState>) -> impl IntoResponse {
    let state = s.state.read().await;
    let holons: Vec<serde_json::Value> = state.active_holons.values().map(|h| {
        serde_json::json!({
            "task_id": h.task_id,
            "chair": h.chair.to_string(),
            "members": h.members.iter().map(|m| m.to_string()).collect::<Vec<_>>(),
            "adversarial_critic": h.adversarial_critic.as_ref().map(|a| a.to_string()),
            "depth": h.depth,
            "parent_holon": h.parent_holon,
            "child_holons": h.child_holons,
            "status": format!("{:?}", h.status),
            "created_at": h.created_at,
            "member_count": h.members.len(),
        })
    }).collect();
    Json(serde_json::json!({ "holons": holons }))
}

async fn api_holon_detail(
    State(s): State<WebState>,
    AxumPath(task_id): AxumPath<String>,
) -> impl IntoResponse {
    let state = s.state.read().await;
    match state.active_holons.get(&task_id) {
        Some(h) => Json(serde_json::json!({
            "task_id": h.task_id,
            "chair": h.chair.to_string(),
            "members": h.members.iter().map(|m| m.to_string()).collect::<Vec<_>>(),
            "adversarial_critic": h.adversarial_critic.as_ref().map(|a| a.to_string()),
            "depth": h.depth,
            "parent_holon": h.parent_holon,
            "child_holons": h.child_holons,
            "subtask_assignments": h.subtask_assignments.iter()
                .map(|(k, v)| (k.clone(), v.to_string()))
                .collect::<std::collections::HashMap<_, _>>(),
            "status": format!("{:?}", h.status),
            "created_at": h.created_at,
        })).into_response(),
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "holon not found"}))).into_response(),
    }
}

async fn api_task_deliberation(
    State(s): State<WebState>,
    AxumPath(task_id): AxumPath<String>,
) -> impl IntoResponse {
    let state = s.state.read().await;
    let messages: Vec<serde_json::Value> = state.deliberation_messages
        .get(&task_id)
        .map(|msgs| msgs.iter().map(|m| serde_json::json!({
            "id": m.id,
            "task_id": m.task_id,
            "timestamp": m.timestamp,
            "speaker": m.speaker.to_string(),
            "round": m.round,
            "message_type": format!("{:?}", m.message_type),
            "content": m.content,
            "referenced_plan_id": m.referenced_plan_id,
            "critic_scores": m.critic_scores,
        })).collect())
        .unwrap_or_default();
    Json(serde_json::json!({ "task_id": task_id, "messages": messages }))
}

async fn api_task_ballots(
    State(s): State<WebState>,
    AxumPath(task_id): AxumPath<String>,
) -> impl IntoResponse {
    let state = s.state.read().await;
    let ballots: Vec<serde_json::Value> = state.ballot_records
        .get(&task_id)
        .map(|records| records.iter().map(|b| serde_json::json!({
            "task_id": b.task_id,
            "voter": b.voter.to_string(),
            "rankings": b.rankings,
            "critic_scores": b.critic_scores,
            "timestamp": b.timestamp,
            "irv_round_when_eliminated": b.irv_round_when_eliminated,
        })).collect())
        .unwrap_or_default();
    Json(serde_json::json!({ "task_id": task_id, "ballots": ballots }))
}

async fn api_task_irv_rounds(
    State(s): State<WebState>,
    AxumPath(task_id): AxumPath<String>,
) -> impl IntoResponse {
    let state = s.state.read().await;
    let rounds: Vec<serde_json::Value> = state.irv_rounds
        .get(&task_id)
        .map(|rounds| rounds.iter().map(|r| serde_json::json!({
            "task_id": r.task_id,
            "round_number": r.round_number,
            "tallies": r.tallies,
            "eliminated": r.eliminated,
            "continuing_candidates": r.continuing_candidates,
        })).collect())
        .unwrap_or_default();
    Json(serde_json::json!({ "task_id": task_id, "irv_rounds": rounds }))
}

// ── WWS Identity / Reputation / Names / Network / Directory / Keys ──────────

async fn api_identity(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let did = s.agent_id.to_string();
    let wws_name = s.registered_names.values()
        .find(|r| r.did == did)
        .map(|r| r.name.clone());
    let uptime_secs = chrono::Utc::now()
        .signed_duration_since(s.start_time)
        .num_seconds()
        .max(0);
    Json(serde_json::json!({
        "did": did,
        "peer_id": did.split(':').next_back().unwrap_or("").chars().take(20).collect::<String>(),
        "wws_name": wws_name,
        "tier": format!("{:?}", s.my_tier).to_lowercase(),
        "key_healthy": true,
        "uptime_secs": uptime_secs,
    }))
}

async fn api_reputation(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let my_id = s.agent_id.to_string();
    let activity = s.agent_activity.get(&my_id);
    let tasks_done = activity.map(|a| a.tasks_processed_count).unwrap_or(0);
    let score = 10u64 + tasks_done * 5;
    let rep_tier = rep_tier_for_score(score);
    Json(serde_json::json!({
        "score": score,
        "tier": rep_tier,
        "next_tier_at": rep_next_threshold(score),
        "positive_total": tasks_done,
        "negative_total": 0,
        "decay": 0.0,
    }))
}

async fn api_reputation_events(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let my_id = s.agent_id.to_string();
    let events: Vec<serde_json::Value> = s.event_log.iter().rev()
        .filter(|e| e.message.contains("task") || e.message.contains("AUDIT"))
        .take(50)
        .map(|e| serde_json::json!({
            "timestamp": e.timestamp,
            "kind": "task_event",
            "delta": 5,
            "description": e.message.replace(&my_id, "[self]"),
        }))
        .collect();
    Json(serde_json::json!({ "events": events }))
}

async fn api_names(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let did = s.agent_id.to_string();
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let names: Vec<serde_json::Value> = s.registered_names.values()
        .filter(|r| r.did == did)
        .map(|r| serde_json::json!({
            "name": r.name,
            "did": r.did,
            "registered_at": r.registered_at,
            "expires_at": r.expires_at,
            "ttl_secs": (r.expires_at as i64 - now_unix as i64).max(0),
        }))
        .collect();
    Json(serde_json::json!({ "names": names }))
}

#[derive(Deserialize)]
struct NameRegisterBody { name: String }

async fn api_names_register(
    State(web): State<WebState>,
    Json(body): Json<NameRegisterBody>,
) -> impl IntoResponse {
    let name = body.name.to_lowercase();
    if name.trim().is_empty() || name.contains(' ') {
        return (StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"ok": false, "error": "invalid_name"}))).into_response();
    }
    let mut s = web.state.write().await;
    let did = s.agent_id.to_string();
    if s.registered_names.contains_key(&name) {
        return (StatusCode::CONFLICT,
            Json(serde_json::json!({"ok": false, "error": "name_taken"}))).into_response();
    }
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let record = wws_network::name_registry::NameRecord {
        name: name.clone(),
        did,
        peer_id: String::new(),
        registered_at: now_unix,
        expires_at: now_unix + 365 * 24 * 3600,
        pow_nonce: 0,
        signature: vec![],
    };
    s.registered_names.insert(name, record);
    (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
}

async fn api_names_renew(
    State(web): State<WebState>,
    AxumPath(name): AxumPath<String>,
) -> impl IntoResponse {
    let mut s = web.state.write().await;
    let did = s.agent_id.to_string();
    if let Some(rec) = s.registered_names.get_mut(&name) {
        if rec.did != did {
            return (StatusCode::FORBIDDEN,
                Json(serde_json::json!({"ok": false, "error": "not_owner"}))).into_response();
        }
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        rec.expires_at = now_unix + 365 * 24 * 3600;
        (StatusCode::OK, Json(serde_json::json!({"ok": true, "expires_at": rec.expires_at}))).into_response()
    } else {
        (StatusCode::NOT_FOUND,
            Json(serde_json::json!({"ok": false, "error": "not_found"}))).into_response()
    }
}

async fn api_names_release(
    State(web): State<WebState>,
    AxumPath(name): AxumPath<String>,
) -> impl IntoResponse {
    let mut s = web.state.write().await;
    let did = s.agent_id.to_string();
    match s.registered_names.get(&name) {
        Some(rec) if rec.did != did =>
            return (StatusCode::FORBIDDEN,
                Json(serde_json::json!({"ok": false, "error": "not_owner"}))).into_response(),
        None =>
            return (StatusCode::NOT_FOUND,
                Json(serde_json::json!({"ok": false, "error": "not_found"}))).into_response(),
        _ => {}
    }
    s.registered_names.remove(&name);
    (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
}

async fn api_network(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let peer_count = s.member_last_seen.len();
    let swarm_size = s.agent_set.len().max(peer_count);
    let bootstrap_connected = peer_count > 0;
    Json(serde_json::json!({
        "bootstrap_connected": bootstrap_connected,
        "peer_count": peer_count,
        "swarm_size_estimate": swarm_size,
        "nat_type": "unknown",
        "current_epoch": s.network_stats.current_epoch,
    }))
}

async fn api_peers(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let now = chrono::Utc::now();
    let peers: Vec<serde_json::Value> = s.member_last_seen.iter()
        .map(|(id, ts)| {
            let seen_secs = now.signed_duration_since(*ts).num_seconds().max(0);
            serde_json::json!({
                "did": id,
                "name": s.agent_names.get(id).cloned().unwrap_or_else(|| short_agent_label(id)),
                "tier": format!("{:?}", s.agent_tiers.get(id).copied().unwrap_or(Tier::Executor)).to_lowercase(),
                "seen_secs": seen_secs,
                "online": seen_secs <= 60,
            })
        })
        .collect();
    Json(serde_json::json!({ "peers": peers }))
}

#[derive(Deserialize, Default)]
struct DirectoryQuery {
    q: Option<String>,
    tier: Option<String>,
    sort: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

async fn api_directory(
    State(web): State<WebState>,
    Query(params): Query<DirectoryQuery>,
) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let now = chrono::Utc::now();
    let members = collect_known_members(&s);
    let my_id = s.agent_id.to_string();

    let q_lower = params.q.as_ref().map(|q| q.to_lowercase());
    let tier_filter = params.tier.as_ref().map(|t| t.to_lowercase());

    let mut agents: Vec<serde_json::Value> = members.into_iter()
        .map(|id| {
            let name = s.agent_names.get(&id).cloned().unwrap_or_else(|| short_agent_label(&id));
            let tier = format!("{:?}", s.agent_tiers.get(&id).copied().unwrap_or(Tier::Executor)).to_lowercase();
            let seen_secs = s.member_last_seen.get(&id)
                .map(|ts| now.signed_duration_since(*ts).num_seconds().max(0));
            let online = seen_secs.map(|v| v <= 60).unwrap_or(id == my_id);
            let activity = s.agent_activity.get(&id);
            let tasks_done = activity.map(|a| a.tasks_processed_count).unwrap_or(0);
            let score = 10u64 + tasks_done * 5;
            serde_json::json!({
                "did": id,
                "name": name,
                "tier": tier,
                "score": score,
                "online": online,
                "task_count": tasks_done,
                "is_self": id == my_id,
            })
        })
        .filter(|a| {
            let name = a["name"].as_str().unwrap_or("").to_lowercase();
            let did = a["did"].as_str().unwrap_or("").to_lowercase();
            let tier = a["tier"].as_str().unwrap_or("").to_lowercase();
            let q_ok = q_lower.as_ref().map(|q| name.contains(q.as_str()) || did.contains(q.as_str())).unwrap_or(true);
            let tier_ok = tier_filter.as_ref().map(|t| tier.contains(t.as_str())).unwrap_or(true);
            q_ok && tier_ok
        })
        .collect();

    let sort = params.sort.as_deref().unwrap_or("reputation");
    if sort == "reputation" {
        agents.sort_by(|a, b| b["score"].as_u64().unwrap_or(0).cmp(&a["score"].as_u64().unwrap_or(0)));
    } else {
        agents.sort_by(|a, b| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")));
    }

    let total = agents.len();
    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(100);
    let page: Vec<_> = agents.into_iter().skip(offset).take(limit).collect();

    Json(serde_json::json!({ "agents": page, "total": total }))
}

async fn api_keys(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let did = s.agent_id.to_string();
    let pubkey_hex = did.split(':').next_back().unwrap_or("").to_string();
    Json(serde_json::json!({
        "did": did,
        "pubkey_hex": pubkey_hex,
        "key_type": "Ed25519",
        "guardian_count": 0,
        "threshold": 1,
        "last_rotation": s.start_time,
    }))
}

async fn api_graph(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let members = collect_known_members(&s);
    let my_id = s.agent_id.to_string();
    let now = chrono::Utc::now();

    let nodes: Vec<serde_json::Value> = members.iter().map(|id| {
        let name = s.agent_names.get(id).cloned().unwrap_or_else(|| short_agent_label(id));
        let tier = format!("{:?}", s.agent_tiers.get(id).copied().unwrap_or(Tier::Executor)).to_lowercase();
        let wws_name = s.registered_names.values().find(|r| &r.did == id).map(|r| r.name.clone());
        let seen_secs = s.member_last_seen.get(id)
            .map(|ts| now.signed_duration_since(*ts).num_seconds().max(0));
        let status = if *id == my_id {
            "healthy"
        } else if seen_secs.map(|v| v <= 60).unwrap_or(false) {
            "healthy"
        } else {
            "unknown"
        };
        serde_json::json!({
            "id": id,
            "did": id,
            "name": name,
            "wws_name": wws_name,
            "tier": tier,
            "type": "agent",
            "status": status,
            "is_self": *id == my_id,
        })
    }).collect();

    let mut edges: Vec<serde_json::Value> = s.agent_parents.iter()
        .map(|(child, parent)| serde_json::json!({"from": parent, "to": child}))
        .collect();

    for peer in s.agent_set.elements() {
        let peer_did = format!("did:swarm:{}", peer);
        edges.push(serde_json::json!({"from": my_id, "to": peer_did}));
    }

    Json(serde_json::json!({"nodes": nodes, "edges": edges}))
}

/// Server-Sent Events stream — replaces WebSocket for browser EventSource compatibility.
async fn api_events(State(web): State<WebState>) -> Response {
    let state = web.state.clone();
    let sse_stream = stream::unfold((state, false), |(state, skip_first_sleep)| async move {
        if skip_first_sleep {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        let payload = {
            let s = state.read().await;
            let recent_messages = s.message_trace.iter().rev().take(20).cloned().collect::<Vec<_>>();
            let recent_events = s.event_log.iter().rev().take(20).cloned().collect::<Vec<_>>();
            serde_json::json!({
                "type": "snapshot",
                "time": chrono::Utc::now(),
                "active_tasks": s.task_set.len(),
                "known_agents": s.active_member_count(Duration::from_secs(ACTIVE_MEMBER_STALENESS_SECS)),
                "messages": recent_messages,
                "events": recent_events,
            })
        };
        let text = format!("data: {}\n\n", payload);
        Some((Ok::<Bytes, std::convert::Infallible>(Bytes::from(text)), (state, true)))
    });

    Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header("x-accel-buffering", "no")
        .body(Body::from_stream(sse_stream))
        .unwrap()
}

fn rep_tier_for_score(score: u64) -> &'static str {
    match score {
        0..=14  => "newcomer",
        15..=49 => "member",
        50..=99 => "trusted",
        100..=199 => "established",
        200..=499 => "veteran",
        _ => "legend",
    }
}

fn rep_next_threshold(score: u64) -> u64 {
    match score {
        0..=14  => 15,
        15..=49 => 50,
        50..=99 => 100,
        100..=199 => 200,
        _ => 500,
    }
}
