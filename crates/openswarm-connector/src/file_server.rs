//! HTTP server for onboarding docs, APIs, and web dashboard assets.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use tokio::sync::RwLock;
use tower_http::services::{ServeDir, ServeFile};

use openswarm_protocol::{ProtocolMethod, SwarmMessage, SwarmTopics, TaskInjectionParams, Tier};

use crate::connector::{ConnectorState, MessageTraceEvent};

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
    network_handle: openswarm_network::SwarmHandle,
}

pub struct FileServer {
    bind_addr: String,
    state: Arc<RwLock<ConnectorState>>,
    network_handle: openswarm_network::SwarmHandle,
    web_root: PathBuf,
}

impl FileServer {
    pub fn new(
        bind_addr: String,
        state: Arc<RwLock<ConnectorState>>,
        network_handle: openswarm_network::SwarmHandle,
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
            .route("/api/hierarchy", get(api_hierarchy))
            .route("/api/voting", get(api_voting))
            .route("/api/voting/:task_id", get(api_voting_task))
            .route("/api/messages", get(api_messages))
            .route("/api/messages/:task_id", get(api_messages_task))
            .route("/api/tasks", get(api_tasks).post(api_submit_task))
            .route("/api/tasks/:task_id/timeline", get(api_task_timeline))
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
    if let Ok(path) = std::env::var("OPENSWARM_WEBAPP_DIR") {
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
        "name": "OpenSwarm Connector",
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
    Json(serde_json::json!({"ok": true, "service": "openswarm-web"}))
}

async fn api_auth_status() -> Json<serde_json::Value> {
    let token_required = std::env::var("OPENSWARM_WEB_TOKEN")
        .ok()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    Json(serde_json::json!({"token_required": token_required}))
}

async fn api_hierarchy(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let active = s.active_member_ids(Duration::from_secs(180));

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
            serde_json::json!({
                "task_id": task_id,
                "proposal_count": v.proposal_count(),
                "ballot_count": v.ballot_count(),
                "finalized": v.is_finalized(),
            })
        })
        .collect::<Vec<_>>();

    let rfp = s
        .rfp_coordinators
        .iter()
        .filter(|(task_id, _)| task_filter.as_ref().map(|t| t == *task_id).unwrap_or(true))
        .map(|(task_id, r)| {
            let plans = r
                .reveals
                .values()
                .map(|p| {
                    serde_json::json!({
                        "proposer": p.proposer.to_string(),
                        "plan_id": p.plan.plan_id,
                        "plan_hash": p.plan_hash,
                        "rationale": p.plan.rationale,
                        "subtask_count": p.plan.subtasks.len(),
                    })
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "task_id": task_id,
                "phase": format!("{:?}", r.phase()),
                "commit_count": r.commit_count(),
                "reveal_count": r.reveal_count(),
                "commits": r.commits_for_debug(),
                "plans": plans,
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
    let tasks = s
        .task_set
        .elements()
        .into_iter()
        .map(|task_id| {
            let task = s.task_details.get(&task_id);
            serde_json::json!({
                "task_id": task_id,
                "description": task.map(|t| t.description.clone()).unwrap_or_default(),
                "status": task.map(|t| format!("{:?}", t.status)).unwrap_or_else(|| "Unknown".to_string()),
                "tier_level": task.map(|t| t.tier_level).unwrap_or(0),
                "assigned_to": task.and_then(|t| t.assigned_to.as_ref().map(|a| a.to_string())),
                "subtasks": task.map(|t| t.subtasks.clone()).unwrap_or_default(),
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
    if let Ok(required) = std::env::var("OPENSWARM_WEB_TOKEN") {
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

    let mut state_guard = web.state.write().await;
    let epoch = state_guard.epoch_manager.current_epoch();
    let task = openswarm_protocol::Task::new(req.description.clone(), 1, epoch);
    let task_id = task.task_id.clone();
    let originator = state_guard.agent_id.clone();
    let actor = state_guard.agent_id.to_string();
    let audit_actor = actor.clone();
    let swarm_id = state_guard.current_swarm_id.as_str().to_string();

    state_guard.task_set.add(task_id.clone());
    state_guard.task_details.insert(task_id.clone(), task.clone());
    state_guard.push_task_timeline_event(
        &task_id,
        "injected",
        format!("Task injected via web dashboard: {}", req.description),
        Some(actor),
    );
    state_guard.push_log(
        crate::tui::LogCategory::Task,
        format!("Task injected via web UI: {} ({})", task_id, req.description),
    );
    state_guard.push_log(
        crate::tui::LogCategory::System,
        format!(
            "AUDIT web.submit_task actor={} task_id={} description={}",
            audit_actor, task_id, req.description
        ),
    );
    drop(state_guard);

    let inject_params = TaskInjectionParams { task, originator };
    let msg = SwarmMessage::new(
        ProtocolMethod::TaskInjection.as_str(),
        serde_json::to_value(&inject_params).unwrap_or_default(),
        String::new(),
    );

    if let Ok(data) = serde_json::to_vec(&msg) {
        let topic = SwarmTopics::tasks_for(&swarm_id, 1);
        let _ = web.network_handle.publish(&topic, data).await;
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "ok": true,
            "task_id": task_id,
            "description": req.description
        })),
    )
}

async fn api_task_timeline(
    State(web): State<WebState>,
    AxumPath(task_id): AxumPath<String>,
) -> Json<serde_json::Value> {
    let s = web.state.read().await;

    let timeline = s.task_timelines.get(&task_id).cloned().unwrap_or_default();
    let task = s.task_details.get(&task_id).cloned();
    let messages = s
        .message_trace
        .iter()
        .filter(|m| m.task_id.as_ref().map(|id| id == &task_id).unwrap_or(false))
        .cloned()
        .collect::<Vec<_>>();

    let descendants = collect_task_descendants(&task_id, &s.task_details);

    Json(serde_json::json!({
        "task": task,
        "timeline": timeline,
        "descendants": descendants,
        "messages": messages,
    }))
}

fn collect_task_descendants(
    root: &str,
    details: &HashMap<String, openswarm_protocol::Task>,
) -> Vec<openswarm_protocol::Task> {
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
    let members = s.active_member_ids(Duration::from_secs(180));

    let nodes = members
        .iter()
        .map(|id| {
            serde_json::json!({
                "id": id,
                "tier": format!("{:?}", s.agent_tiers.get(id).copied().unwrap_or(Tier::Executor)),
                "is_self": *id == s.agent_id.to_string(),
            })
        })
        .collect::<Vec<_>>();

    let mut edges = s
        .agent_parents
        .iter()
        .map(|(child, parent)| {
            serde_json::json!({"source": parent, "target": child, "kind": "hierarchy"})
        })
        .collect::<Vec<_>>();

    for peer in s.agent_set.elements() {
        edges.push(serde_json::json!({
            "source": s.agent_id.to_string(),
            "target": format!("did:swarm:{}", peer),
            "kind": "peer_link"
        }));
    }

    Json(serde_json::json!({"nodes": nodes, "edges": edges}))
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
                "known_agents": s.active_member_count(Duration::from_secs(180)),
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
