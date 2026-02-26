//! JSON-RPC 2.0 server over TCP implementing the Swarm API.
//!
//! Provides the following methods for the local AI agent:
//! - `swarm.connect()` - Connect to a peer by multiaddress
//! - `swarm.get_network_stats()` - Get current network statistics
//! - `swarm.propose_plan()` - Submit a task decomposition plan
//! - `swarm.submit_result()` - Submit a task execution result
//! - `swarm.receive_task()` - Poll for assigned tasks
//! - `swarm.get_task()` - Get full details for a task by ID
//! - `swarm.get_task_timeline()` - Get lifecycle timeline for a task
//! - `swarm.get_status()` - Get connector and agent status
//! - `swarm.register_agent()` - Register an execution agent identity
//! - `swarm.list_swarms()` - List all known swarms with their info
//! - `swarm.create_swarm()` - Create a new private swarm
//! - `swarm.join_swarm()` - Join an existing swarm
//!
//! The server listens on localhost TCP and speaks JSON-RPC 2.0.
//! Each line received is a JSON-RPC request; each line sent is a response.

use std::sync::Arc;
use std::time::Duration;

use openswarm_consensus::rfp::RfpPhase;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

use openswarm_protocol::*;

use crate::connector::{ConnectorState, SwarmRecord, TaskTimelineEvent};

const ACTIVE_MEMBER_STALENESS_SECS: u64 = 45;

/// The JSON-RPC 2.0 server.
pub struct RpcServer {
    /// TCP listener address.
    bind_addr: String,
    /// Shared connector state.
    state: Arc<RwLock<ConnectorState>>,
    /// Network handle for network operations.
    network_handle: openswarm_network::SwarmHandle,
    /// Maximum concurrent connections.
    max_connections: usize,
}

impl RpcServer {
    /// Create a new RPC server.
    pub fn new(
        bind_addr: String,
        state: Arc<RwLock<ConnectorState>>,
        network_handle: openswarm_network::SwarmHandle,
        max_connections: usize,
    ) -> Self {
        Self {
            bind_addr,
            state,
            network_handle,
            max_connections,
        }
    }

    /// Start the RPC server, listening for connections.
    pub async fn run(self) -> Result<(), anyhow::Error> {
        let listener = TcpListener::bind(&self.bind_addr).await?;
        tracing::info!(addr = %self.bind_addr, "JSON-RPC server listening");

        let state = Arc::clone(&self.state);
        let network_handle = self.network_handle.clone();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.max_connections));

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            tracing::debug!(peer = %peer_addr, "RPC client connected");

            let state = Arc::clone(&state);
            let network_handle = network_handle.clone();
            let permit = semaphore.clone().acquire_owned().await?;

            tokio::spawn(async move {
                if let Err(e) =
                    handle_connection(stream, state, network_handle).await
                {
                    tracing::warn!(
                        peer = %peer_addr,
                        error = %e,
                        "RPC connection error"
                    );
                }
                drop(permit);
            });
        }
    }
}

/// Handle a single RPC client connection.
///
/// Reads newline-delimited JSON-RPC requests and sends back responses.
async fn handle_connection(
    stream: tokio::net::TcpStream,
    state: Arc<RwLock<ConnectorState>>,
    network_handle: openswarm_network::SwarmHandle,
) -> Result<(), anyhow::Error> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await? {
        let response = process_request(&line, &state, &network_handle).await;
        let response_json = serde_json::to_string(&response)?;
        writer.write_all(response_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }

    Ok(())
}

/// Process a single JSON-RPC request and return a response.
async fn process_request(
    request_str: &str,
    state: &Arc<RwLock<ConnectorState>>,
    network_handle: &openswarm_network::SwarmHandle,
) -> SwarmResponse {
    // Parse the request.
    let request: SwarmMessage = match serde_json::from_str(request_str) {
        Ok(r) => r,
        Err(e) => {
            return SwarmResponse::error(
                None,
                -32700, // Parse error
                format!("Invalid JSON: {}", e),
            );
        }
    };

    let request_id = request.id.clone();

    match request.method.as_str() {
        "swarm.connect" => handle_connect(request_id, &request.params, network_handle).await,
        "swarm.get_network_stats" => handle_get_network_stats(request_id, state).await,
        "swarm.propose_plan" => {
            handle_propose_plan(request_id, &request.params, state, network_handle).await
        }
        "swarm.submit_vote" => {
            handle_submit_vote(request_id, &request.params, state, network_handle).await
        }
        "swarm.get_voting_state" => handle_get_voting_state(request_id, &request.params, state).await,
        "swarm.submit_result" => {
            handle_submit_result(request_id, &request.params, state, network_handle).await
        }
        "swarm.receive_task" => handle_receive_task(request_id, state).await,
        "swarm.get_task" => handle_get_task(request_id, &request.params, state).await,
        "swarm.get_task_timeline" => {
            handle_get_task_timeline(request_id, &request.params, state).await
        }
        "swarm.get_status" => handle_get_status(request_id, state).await,
        "swarm.register_agent" => {
            handle_register_agent(request_id, &request.params, state, network_handle).await
        }
        "swarm.list_swarms" => handle_list_swarms(request_id, state).await,
        "swarm.create_swarm" => {
            handle_create_swarm(request_id, &request.params, state).await
        }
        "swarm.join_swarm" => {
            handle_join_swarm(request_id, &request.params, state).await
        }
        "swarm.inject_task" => {
            handle_inject_task(request_id, &request.params, state, network_handle).await
        }
        "swarm.get_hierarchy" => handle_get_hierarchy(request_id, state).await,
        _ => SwarmResponse::error(
            request_id,
            -32601, // Method not found
            format!("Unknown method: {}", request.method),
        ),
    }
}

/// Handle `swarm.submit_vote` - submit a ranked vote for a task.
async fn handle_submit_vote(
    id: Option<String>,
    params: &serde_json::Value,
    state: &Arc<RwLock<ConnectorState>>,
    network_handle: &openswarm_network::SwarmHandle,
) -> SwarmResponse {
    let task_id = match params.get("task_id").and_then(|v| v.as_str()) {
        Some(v) if !v.is_empty() => v.to_string(),
        _ => {
            return SwarmResponse::error(id, -32602, "Missing 'task_id' parameter".to_string());
        }
    };

    let rankings: Vec<String> = match params.get("rankings").and_then(|v| v.as_array()) {
        Some(arr) if !arr.is_empty() => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => {
            return SwarmResponse::error(
                id,
                -32602,
                "Missing or empty 'rankings' parameter".to_string(),
            );
        }
    };

    let epoch = {
        let state = state.read().await;
        params
            .get("epoch")
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| state.epoch_manager.current_epoch())
    };

    let (voter, swarm_id, ballot_count, proposal_count) = {
        let mut state = state.write().await;
        let voter = state.agent_id.clone();

        let proposals: std::collections::HashMap<String, AgentId> = rankings
            .iter()
            .map(|plan_id| {
                (
                    plan_id.clone(),
                    AgentId::new(format!("did:swarm:proposal-owner:{}", plan_id)),
                )
            })
            .collect();

        let (ballot_count, proposal_count) = {
            let voting = state.voting_engines.entry(task_id.clone()).or_insert_with(|| {
                let mut engine = openswarm_consensus::VotingEngine::new(
                    openswarm_consensus::voting::VotingConfig::default(),
                    task_id.clone(),
                    epoch,
                );
                engine.set_proposals(proposals.clone());
                engine
            });

            let ranked_vote = RankedVote {
                voter: voter.clone(),
                task_id: task_id.clone(),
                epoch,
                rankings: rankings.clone(),
                critic_scores: std::collections::HashMap::new(),
            };

            if let Err(e) = voting.record_vote(ranked_vote) {
                return SwarmResponse::error(id, -32000, format!("Failed to record vote: {}", e));
            }

            (voting.ballot_count(), voting.proposal_count())
        };

        state.push_task_timeline_event(
            &task_id,
            "vote_recorded",
            format!("Vote submitted via RPC: {}", rankings.join(" > ")),
            Some(voter.to_string()),
        );
        state.push_log(
            crate::tui::LogCategory::Vote,
            format!(
                "Vote submitted for task {} by {} ({})",
                task_id,
                voter,
                rankings.join(" > ")
            ),
        );

        (
            voter,
            state.current_swarm_id.as_str().to_string(),
            ballot_count,
            proposal_count,
        )
    };

    let vote_msg = SwarmMessage::new(
        ProtocolMethod::ConsensusVote.as_str(),
        serde_json::json!({
            "task_id": task_id,
            "voter": voter,
            "epoch": epoch,
            "rankings": rankings,
            "critic_scores": {},
        }),
        String::new(),
    );

    if let Ok(data) = serde_json::to_vec(&vote_msg) {
        let topic = SwarmTopics::voting_for(&swarm_id, &task_id);
        let _ = network_handle.publish(&topic, data).await;
    }

    SwarmResponse::success(
        id,
        serde_json::json!({
            "task_id": task_id,
            "accepted": true,
            "ballot_count": ballot_count,
            "proposal_count": proposal_count,
        }),
    )
}

/// Handle `swarm.get_voting_state` - inspect voting and proposal state.
async fn handle_get_voting_state(
    id: Option<String>,
    params: &serde_json::Value,
    state: &Arc<RwLock<ConnectorState>>,
) -> SwarmResponse {
    let maybe_task_id = params
        .get("task_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let state = state.read().await;

    let voting_entries: Vec<serde_json::Value> = state
        .voting_engines
        .iter()
        .filter(|(task_id, _)| maybe_task_id.as_ref().map(|t| t == *task_id).unwrap_or(true))
        .map(|(task_id, voting)| {
            serde_json::json!({
                "task_id": task_id,
                "proposal_count": voting.proposal_count(),
                "ballot_count": voting.ballot_count(),
                "quorum_reached": voting.ballot_count() >= voting.proposal_count() && voting.ballot_count() > 0,
            })
        })
        .collect();

    let rfp_entries: Vec<serde_json::Value> = state
        .rfp_coordinators
        .iter()
        .filter(|(task_id, _)| maybe_task_id.as_ref().map(|t| t == *task_id).unwrap_or(true))
        .map(|(task_id, rfp)| {
            serde_json::json!({
                "task_id": task_id,
                "phase": format!("{:?}", rfp.phase()),
                "commit_count": rfp.commit_count(),
                "reveal_count": rfp.reveal_count(),
            })
        })
        .collect();

    SwarmResponse::success(
        id,
        serde_json::json!({
            "voting_engines": voting_entries,
            "rfp_coordinators": rfp_entries,
        }),
    )
}

/// Handle `swarm.connect` - connect to a peer by multiaddress.
async fn handle_connect(
    id: Option<String>,
    params: &serde_json::Value,
    network_handle: &openswarm_network::SwarmHandle,
) -> SwarmResponse {
    let addr_str = match params.get("addr").and_then(|v| v.as_str()) {
        Some(a) => a,
        None => {
            return SwarmResponse::error(
                id,
                -32602,
                "Missing 'addr' parameter".into(),
            );
        }
    };

    let addr: openswarm_network::Multiaddr = match addr_str.parse() {
        Ok(a) => a,
        Err(e) => {
            return SwarmResponse::error(
                id,
                -32602,
                format!("Invalid multiaddress: {}", e),
            );
        }
    };

    match network_handle.dial(addr).await {
        Ok(()) => SwarmResponse::success(id, serde_json::json!({"connected": true})),
        Err(e) => SwarmResponse::error(id, -32000, format!("Dial failed: {}", e)),
    }
}

/// Handle `swarm.get_network_stats` - return current network statistics.
async fn handle_get_network_stats(
    id: Option<String>,
    state: &Arc<RwLock<ConnectorState>>,
) -> SwarmResponse {
    let state = state.read().await;
    let mut stats = state.network_stats.clone();
    stats.total_agents =
        state.active_member_count(Duration::from_secs(ACTIVE_MEMBER_STALENESS_SECS)) as u64;

    SwarmResponse::success(
        id,
        serde_json::to_value(&stats).unwrap_or_default(),
    )
}

/// Handle `swarm.propose_plan` - submit a task decomposition plan.
pub(crate) async fn handle_propose_plan(
    id: Option<String>,
    params: &serde_json::Value,
    state: &Arc<RwLock<ConnectorState>>,
    network_handle: &openswarm_network::SwarmHandle,
) -> SwarmResponse {
    let mut plan: Plan = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return SwarmResponse::error(
                id,
                -32602,
                format!("Invalid plan: {}", e),
            );
        }
    };

    {
        let state = state.read().await;
        plan.proposer = state.agent_id.clone();
    }

    let plan_hash = match openswarm_consensus::RfpCoordinator::compute_plan_hash(&plan) {
        Ok(h) => h,
        Err(e) => {
            return SwarmResponse::error(
                id,
                -32000,
                format!("Hash computation failed: {}", e),
            );
        }
    };

    let (swarm_id, has_task, subtask_count) = {
        let mut state = state.write().await;

        let task = state.task_details.get(&plan.task_id).cloned().unwrap_or_else(|| Task {
            task_id: plan.task_id.clone(),
            parent_task_id: None,
            epoch: plan.epoch,
            status: TaskStatus::ProposalPhase,
            description: "Task proposed for decomposition".to_string(),
            assigned_to: Some(plan.proposer.clone()),
            tier_level: 1,
            subtasks: plan
                .subtasks
                .iter()
                .map(|s| format!("{}:{}", s.index, s.description))
                .collect(),
            created_at: chrono::Utc::now(),
            deadline: None,
        });

        let coordinator = state
            .rfp_coordinators
            .entry(plan.task_id.clone())
            .or_insert_with(|| openswarm_consensus::RfpCoordinator::new(plan.task_id.clone(), plan.epoch, 1));

        if matches!(coordinator.phase(), RfpPhase::Idle) {
            if let Err(e) = coordinator.inject_task(&task) {
                return SwarmResponse::error(
                    id,
                    -32000,
                    format!("Failed to initialize RFP: {}", e),
                );
            }
        }

        let commit = ProposalCommitParams {
            task_id: plan.task_id.clone(),
            proposer: plan.proposer.clone(),
            epoch: plan.epoch,
            plan_hash: plan_hash.clone(),
        };
        if let Err(e) = coordinator.record_commit(&commit) {
            return SwarmResponse::error(
                id,
                -32000,
                format!("Failed to record proposal commit: {}", e),
            );
        }

        if matches!(coordinator.phase(), RfpPhase::CommitPhase) {
            let _ = coordinator.transition_to_reveal();
        }

        let reveal = ProposalRevealParams {
            task_id: plan.task_id.clone(),
            plan: plan.clone(),
        };
        if let Err(e) = coordinator.record_reveal(&reveal) {
            return SwarmResponse::error(
                id,
                -32000,
                format!("Failed to record proposal reveal: {}", e),
            );
        }

        state.push_log(
            crate::tui::LogCategory::Task,
            format!(
                "Plan proposed for task {}: {} subtasks (plan {}) -> {}",
                plan.task_id,
                plan.subtasks.len(),
                plan.plan_id,
                plan
                    .subtasks
                    .iter()
                    .map(|s| format!("{}:{}", s.index, s.description))
                    .collect::<Vec<_>>()
                    .join(" | ")
            ),
        );
        state.push_task_timeline_event(
            &plan.task_id,
            "proposed",
            format!("Plan {} proposed with {} subtasks", plan.plan_id, plan.subtasks.len()),
            Some(plan.proposer.to_string()),
        );
        state.push_log(
            crate::tui::LogCategory::System,
            format!(
                "AUDIT plan.propose actor={} task_id={} plan_id={} subtasks={}",
                plan.proposer,
                plan.task_id,
                plan.plan_id,
                plan.subtasks.len()
            ),
        );

        (
            state.current_swarm_id.as_str().to_string(),
            state.task_details.contains_key(&plan.task_id),
            plan.subtasks.len(),
        )
    };

    if !has_task {
        let mut state = state.write().await;
        state.task_details.insert(
            plan.task_id.clone(),
            Task {
                task_id: plan.task_id.clone(),
                parent_task_id: None,
                epoch: plan.epoch,
                status: TaskStatus::ProposalPhase,
                description: "Task proposed for decomposition".to_string(),
                assigned_to: Some(plan.proposer.clone()),
                tier_level: 1,
                subtasks: plan
                    .subtasks
                    .iter()
                    .map(|s| format!("{}:{}", s.index, s.description))
                    .collect(),
                created_at: chrono::Utc::now(),
                deadline: None,
            },
        );
    }

    // Materialize subtasks and prepare assignment messages.
    let assignment_payloads: Vec<(String, Vec<u8>)> = {
        let mut state = state.write().await;
        let assignees: Vec<AgentId> = state
            .subordinates
            .get(plan.proposer.as_str())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(AgentId::new)
            .collect();
        let parent_tier = state
            .task_details
            .get(&plan.task_id)
            .map(|t| t.tier_level)
            .unwrap_or(1);
        let mut subtask_ids = Vec::with_capacity(plan.subtasks.len());
        let mut payloads = Vec::new();

        for (idx, st) in plan.subtasks.iter().enumerate() {
            let subtask_id = format!("{}-st-{}", plan.task_id, idx + 1);
            let assignee = if assignees.is_empty() {
                None
            } else {
                Some(assignees[idx % assignees.len()].clone())
            };
            let subtask = Task {
                task_id: subtask_id.clone(),
                parent_task_id: Some(plan.task_id.clone()),
                epoch: plan.epoch,
                status: if assignee.is_some() {
                    TaskStatus::InProgress
                } else {
                    TaskStatus::Pending
                },
                description: st.description.clone(),
                assigned_to: assignee.clone(),
                tier_level: (parent_tier + 1).min(openswarm_protocol::MAX_HIERARCHY_DEPTH),
                subtasks: Vec::new(),
                created_at: chrono::Utc::now(),
                deadline: None,
            };

            state.task_details.insert(subtask_id.clone(), subtask.clone());
            state.push_task_timeline_event(
                &plan.task_id,
                "subtask_created",
                format!("{} -> {}", subtask_id, st.description),
                assignee.as_ref().map(|a| a.to_string()),
            );
            subtask_ids.push(subtask_id.clone());

            if let Some(assignee) = assignee {
                let assign_params = TaskAssignmentParams {
                    task: subtask,
                    assignee,
                    parent_task_id: plan.task_id.clone(),
                    winning_plan_id: plan.plan_id.clone(),
                };
                let assign_msg = SwarmMessage::new(
                    ProtocolMethod::TaskAssignment.as_str(),
                    serde_json::to_value(&assign_params).unwrap_or_default(),
                    String::new(),
                );
                if let Ok(data) = serde_json::to_vec(&assign_msg) {
                    let topic = SwarmTopics::tasks_for(&swarm_id, assign_params.task.tier_level);
                    payloads.push((topic, data));
                }
            }
        }

        if let Some(parent) = state.task_details.get_mut(&plan.task_id) {
            parent.subtasks = subtask_ids;
            parent.status = TaskStatus::InProgress;
        }
        payloads
    };

    let proposals_topic = SwarmTopics::proposals_for(&swarm_id, &plan.task_id);
    let voting_topic = SwarmTopics::voting_for(&swarm_id, &plan.task_id);
    let results_topic = SwarmTopics::results_for(&swarm_id, &plan.task_id);

    if let Err(e) = network_handle.subscribe(&proposals_topic).await {
        tracing::debug!(error = %e, topic = %proposals_topic, "Failed to subscribe proposals topic");
    }
    if let Err(e) = network_handle.subscribe(&voting_topic).await {
        tracing::debug!(error = %e, topic = %voting_topic, "Failed to subscribe voting topic");
    }
    if let Err(e) = network_handle.subscribe(&results_topic).await {
        tracing::debug!(error = %e, topic = %results_topic, "Failed to subscribe results topic");
    }

    let commit_params = ProposalCommitParams {
        task_id: plan.task_id.clone(),
        proposer: plan.proposer.clone(),
        epoch: plan.epoch,
        plan_hash: plan_hash.clone(),
    };
    let commit_msg = SwarmMessage::new(
        ProtocolMethod::ProposalCommit.as_str(),
        serde_json::to_value(&commit_params).unwrap_or_default(),
        String::new(),
    );
    let commit_data = match serde_json::to_vec(&commit_msg) {
        Ok(data) => data,
        Err(e) => {
            return SwarmResponse::error(
                id,
                -32000,
                format!("Failed to serialize proposal commit: {}", e),
            );
        }
    };
    let commit_published = match network_handle.publish(&proposals_topic, commit_data).await {
        Ok(()) => true,
        Err(e) => {
            tracing::debug!(error = %e, topic = %proposals_topic, "Failed to publish proposal commit");
            false
        }
    };

    let reveal_params = ProposalRevealParams {
        task_id: plan.task_id.clone(),
        plan: plan.clone(),
    };
    let reveal_msg = SwarmMessage::new(
        ProtocolMethod::ProposalReveal.as_str(),
        serde_json::to_value(&reveal_params).unwrap_or_default(),
        String::new(),
    );
    let reveal_data = match serde_json::to_vec(&reveal_msg) {
        Ok(data) => data,
        Err(e) => {
            return SwarmResponse::error(
                id,
                -32000,
                format!("Failed to serialize proposal reveal: {}", e),
            );
        }
    };
    let reveal_published = match network_handle.publish(&proposals_topic, reveal_data).await {
        Ok(()) => true,
        Err(e) => {
            tracing::debug!(error = %e, topic = %proposals_topic, "Failed to publish proposal reveal");
            false
        }
    };

    let mut assignment_published = 0usize;
    for (topic, data) in assignment_payloads {
        if network_handle.publish(&topic, data).await.is_ok() {
            assignment_published += 1;
        }
    }

    {
        let mut state = state.write().await;
        if let Some(task) = state.task_details.get_mut(&plan.task_id) {
            // Keep subtask IDs populated by decomposition block.
            if task.subtasks.is_empty() {
                task.subtasks = plan
                    .subtasks
                    .iter()
                    .map(|s| format!("{}:{}", s.index, s.description))
                    .collect();
            }
            if task.status != TaskStatus::InProgress {
                task.status = TaskStatus::VotingPhase;
            }
        }
        state.push_log(
            crate::tui::LogCategory::Task,
            format!(
                "Plan {} published for task {} (subtasks: {}, commit: {}, reveal: {})",
                plan.plan_id,
                plan.task_id,
                subtask_count,
                commit_published,
                reveal_published
            ),
        );
        state.push_task_timeline_event(
            &plan.task_id,
            "published",
            format!(
                "Plan {} published (commit={}, reveal={}, assignments={})",
                plan.plan_id, commit_published, reveal_published, assignment_published
            ),
            Some(plan.proposer.to_string()),
        );
        state.push_log(
            crate::tui::LogCategory::System,
            format!(
                "AUDIT assignment.publish actor={} task_id={} assignments={}",
                plan.proposer, plan.task_id, assignment_published
            ),
        );

    }

    SwarmResponse::success(
        id,
        serde_json::json!({
            "plan_id": plan.plan_id,
            "plan_hash": plan_hash,
            "task_id": plan.task_id,
            "accepted": true,
            "commit_published": commit_published,
            "reveal_published": reveal_published,
            "subtasks_created": subtask_count,
            "assignments_published": assignment_published,
        }),
    )
}

/// Aggregate results from all subtasks of a parent task.
fn aggregate_subtask_results(state: &ConnectorState, parent_task_id: &str) -> Artifact {
    use sha2::{Digest, Sha256};

    let parent_task = state.task_details.get(parent_task_id);
    let subtask_ids = parent_task
        .map(|t| t.subtasks.clone())
        .unwrap_or_default();

    // Collect all subtask results
    let mut subtask_results = Vec::new();
    for subtask_id in &subtask_ids {
        if let Some(result) = state.task_results.get(subtask_id) {
            subtask_results.push(result.clone());
        }
    }

    // Create aggregated content (concatenate content CIDs)
    let aggregated_content = subtask_results
        .iter()
        .map(|r| format!("subtask:{} -> cid:{}", r.task_id, r.content_cid))
        .collect::<Vec<_>>()
        .join("\n");

    // Compute content-addressed ID for aggregated result
    let mut hasher = Sha256::new();
    hasher.update(aggregated_content.as_bytes());
    let content_cid = format!("{:x}", hasher.finalize());

    // Compute Merkle hash (aggregate subtask merkle hashes)
    let mut merkle_hasher = Sha256::new();
    for result in &subtask_results {
        merkle_hasher.update(result.merkle_hash.as_bytes());
    }
    let merkle_hash = format!("{:x}", merkle_hasher.finalize());

    Artifact {
        artifact_id: format!("{}-aggregated", parent_task_id),
        task_id: parent_task_id.to_string(),
        producer: state.agent_id.clone(),
        content_cid,
        merkle_hash,
        content_type: "application/json; aggregated".to_string(),
        size_bytes: aggregated_content.len() as u64,
        created_at: chrono::Utc::now(),
    }
}

/// Handle `swarm.submit_result` - submit a task execution result.
pub(crate) async fn handle_submit_result(
    id: Option<String>,
    params: &serde_json::Value,
    state: &Arc<RwLock<ConnectorState>>,
    network_handle: &openswarm_network::SwarmHandle,
) -> SwarmResponse {
    let mut submission: ResultSubmissionParams = match serde_json::from_value(params.clone()) {
        Ok(s) => s,
        Err(e) => {
            return SwarmResponse::error(
                id,
                -32602,
                format!("Invalid result submission: {}", e),
            );
        }
    };

    {
        let state = state.read().await;
        submission.agent_id = state.agent_id.clone();
        submission.artifact.producer = state.agent_id.clone();
    }

    // Add to Merkle DAG and update task state.
    let (dag_nodes, parent_propagation_info) = {
        let mut state = state.write().await;

        if let Some(task) = state.task_details.get(&submission.task_id) {
            if !task.subtasks.is_empty() {
                let all_subtasks_done = task.subtasks.iter().all(|sub_id| {
                    state
                        .task_details
                        .get(sub_id)
                        .map(|t| t.status == TaskStatus::Completed)
                        .unwrap_or(false)
                });
                if !all_subtasks_done {
                    return SwarmResponse::error(
                        id,
                        -32010,
                        format!(
                            "Cannot submit aggregated result for {} before all subtasks are completed",
                            submission.task_id
                        ),
                    );
                }
            }
        }

        let parent_task_id = state
            .task_details
            .get(&submission.task_id)
            .and_then(|t| t.parent_task_id.clone());

        if let Some(task) = state.task_details.get_mut(&submission.task_id) {
            task.status = TaskStatus::Completed;
            task.assigned_to = Some(submission.agent_id.clone());
        }
        state.task_set.remove(&submission.task_id);
        state.mark_member_seen(submission.agent_id.as_str());
        state.merkle_dag.add_leaf(
            submission.task_id.clone(),
            submission.artifact.content_cid.as_bytes(),
        );
        let nodes = state.merkle_dag.node_count();
        state.push_task_timeline_event(
            &submission.task_id,
            "result_submitted",
            format!("Artifact {} (dag_nodes={})", submission.artifact.artifact_id, nodes),
            Some(submission.agent_id.to_string()),
        );
        state.push_log(
            crate::tui::LogCategory::Task,
            format!(
                "Result submitted for task {} by {} (artifact {}, dag_nodes={})",
                submission.task_id,
                submission.agent_id,
                submission.artifact.artifact_id,
                nodes
            ),
        );
        state.push_log(
            crate::tui::LogCategory::System,
            format!(
                "AUDIT result.submit actor={} task_id={} artifact={}",
                submission.agent_id, submission.task_id, submission.artifact.artifact_id
            ),
        );

        // Store the result for potential aggregation
        state.task_results.insert(submission.task_id.clone(), submission.artifact.clone());

        let propagation_info = if let Some(parent_id) = parent_task_id {
            let parent_completed = state
                .task_details
                .get(&parent_id)
                .map(|parent| {
                    !parent.subtasks.is_empty()
                        && parent.subtasks.iter().all(|sub_id| {
                            state
                                .task_details
                                .get(sub_id)
                                .map(|t| t.status == TaskStatus::Completed)
                                .unwrap_or(false)
                        })
                })
                .unwrap_or(false);

            if parent_completed {
                // Aggregate results from all subtasks
                let aggregated_artifact = aggregate_subtask_results(&state, &parent_id);

                if let Some(parent) = state.task_details.get_mut(&parent_id) {
                    parent.status = TaskStatus::Completed;
                }
                state.task_set.remove(&parent_id);

                // Store aggregated result
                state.task_results.insert(parent_id.clone(), aggregated_artifact.clone());

                state.push_task_timeline_event(
                    &parent_id,
                    "aggregated",
                    format!("All subtasks completed; parent {} marked completed", parent_id),
                    Some(submission.agent_id.to_string()),
                );
                state.push_log(
                    crate::tui::LogCategory::Task,
                    format!("Parent task {} completed via subtask aggregation", parent_id),
                );

                // Get grandparent for recursive propagation
                let grandparent_id = state
                    .task_details
                    .get(&parent_id)
                    .and_then(|p| p.parent_task_id.clone());

                Some((parent_id, aggregated_artifact, grandparent_id))
            } else {
                None
            }
        } else {
            None
        };

        (nodes, propagation_info)
    };

    // Publish result to the results topic.
    let swarm_id = {
        let state = state.read().await;
        state.current_swarm_id.as_str().to_string()
    };
    let topic = SwarmTopics::results_for(&swarm_id, &submission.task_id);
    let msg = SwarmMessage::new(
        ProtocolMethod::ResultSubmission.as_str(),
        serde_json::to_value(&submission).unwrap_or_default(),
        String::new(),
    );
    if let Ok(data) = serde_json::to_vec(&msg) {
        if let Err(e) = network_handle.publish(&topic, data).await {
            tracing::warn!(error = %e, "Failed to publish result");
        }
    }

    // Hierarchical propagation: if parent was aggregated, submit aggregated result
    // for the parent task as a normal result event. If a grandparent exists,
    // recursive propagation will continue in the nested call.
    if let Some((parent_id, aggregated_artifact, grandparent_id)) = parent_propagation_info {
        let my_agent_id = {
            let state = state.read().await;
            state.agent_id.clone()
        };

        tracing::info!(
            parent_task_id = %parent_id,
            grandparent_task_id = ?grandparent_id,
            "Propagating aggregated result up hierarchy"
        );

        // Recursively submit aggregated result to grandparent
        let propagation_submission = ResultSubmissionParams {
            task_id: parent_id.clone(),
            agent_id: my_agent_id.clone(),
            artifact: aggregated_artifact,
            merkle_proof: vec![], // TODO: proper merkle proof
        };

        // Recursively call handle_submit_result for the parent task
        let _ = Box::pin(handle_submit_result(
            None,
            &serde_json::to_value(&propagation_submission).unwrap_or_default(),
            state,
            network_handle,
        ))
        .await;
    }

    SwarmResponse::success(
        id,
        serde_json::json!({
            "task_id": submission.task_id,
            "artifact_id": submission.artifact.artifact_id,
            "accepted": true,
            "dag_nodes": dag_nodes,
        }),
    )
}

/// Handle `swarm.receive_task` - poll for assigned tasks.
async fn handle_receive_task(
    id: Option<String>,
    state: &Arc<RwLock<ConnectorState>>,
) -> SwarmResponse {
    let mut state = state.write().await;
    let my_id = state.agent_id.clone();
    state.mark_member_polled_tasks(my_id.as_str());
    let my_tier = state.my_tier;
    let my_tier_level = my_tier.depth();

    let mut tasks: Vec<&Task> = state
        .task_details
        .values()
        .filter(|task| {
            if !state.task_set.contains(&task.task_id) {
                return false;
            }

            if matches!(
                task.status,
                TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Rejected
            ) {
                return false;
            }

            match my_tier {
                Tier::Executor => task.assigned_to.as_ref() == Some(&my_id),
                _ => {
                    task.tier_level == my_tier_level
                        && (task.assigned_to.is_none() || task.assigned_to.as_ref() == Some(&my_id))
                }
            }
        })
        .collect();
    tasks.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    let tasks: Vec<String> = tasks.into_iter().map(|t| t.task_id.clone()).collect();

    SwarmResponse::success(
        id,
        serde_json::json!({
            "pending_tasks": tasks,
            "agent_id": state.agent_id.to_string(),
            "tier": format!("{:?}", state.my_tier),
        }),
    )
}

/// Handle `swarm.get_task` - fetch full metadata for a task by ID.
async fn handle_get_task(
    id: Option<String>,
    params: &serde_json::Value,
    state: &Arc<RwLock<ConnectorState>>,
) -> SwarmResponse {
    let task_id = match params.get("task_id").and_then(|v| v.as_str()) {
        Some(t) if !t.trim().is_empty() => t,
        _ => {
            return SwarmResponse::error(
                id,
                -32602,
                "Missing 'task_id' parameter".into(),
            );
        }
    };

    let state = state.read().await;
    let task = match state.task_details.get(task_id) {
        Some(task) => task,
        None => {
            return SwarmResponse::error(
                id,
                -32004,
                format!("Task not found: {}", task_id),
            );
        }
    };

    SwarmResponse::success(
        id,
        serde_json::json!({
            "task": task,
            "is_pending": state.task_set.contains(&task.task_id),
        }),
    )
}

/// Handle `swarm.get_task_timeline` - fetch lifecycle events for a task.
async fn handle_get_task_timeline(
    id: Option<String>,
    params: &serde_json::Value,
    state: &Arc<RwLock<ConnectorState>>,
) -> SwarmResponse {
    let task_id = match params.get("task_id").and_then(|v| v.as_str()) {
        Some(t) if !t.trim().is_empty() => t,
        _ => {
            return SwarmResponse::error(
                id,
                -32602,
                "Missing 'task_id' parameter".into(),
            );
        }
    };

    let limit = params
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(200)
        .min(1000);

    let state = state.read().await;
    let timeline: Vec<TaskTimelineEvent> = state
        .task_timelines
        .get(task_id)
        .cloned()
        .unwrap_or_default();
    let total = timeline.len();
    let start = total.saturating_sub(limit);
    let events = timeline.into_iter().skip(start).collect::<Vec<_>>();

    SwarmResponse::success(
        id,
        serde_json::json!({
            "task_id": task_id,
            "events": events,
            "event_count": total,
        }),
    )
}

/// Handle `swarm.get_status` - get connector and agent status.
async fn handle_get_status(
    id: Option<String>,
    state: &Arc<RwLock<ConnectorState>>,
) -> SwarmResponse {
    let state = state.read().await;
    let known_agents = state.active_member_count(Duration::from_secs(ACTIVE_MEMBER_STALENESS_SECS));

    SwarmResponse::success(
        id,
        serde_json::json!({
            "agent_id": state.agent_id.to_string(),
            "status": format!("{:?}", state.status),
            "tier": format!("{:?}", state.my_tier),
            "epoch": state.epoch_manager.current_epoch(),
            "parent_id": state.parent_id.as_ref().map(|p| p.to_string()),
            "active_tasks": state.task_set.len(),
            "known_agents": known_agents,
            "content_items": state.content_store.item_count(),
        }),
    )
}

/// Handle `swarm.register_agent` - register an execution agent identity.
async fn handle_register_agent(
    id: Option<String>,
    params: &serde_json::Value,
    state: &Arc<RwLock<ConnectorState>>,
    network_handle: &openswarm_network::SwarmHandle,
) -> SwarmResponse {
    let requested_agent_id = match params.get("agent_id").and_then(|v| v.as_str()) {
        Some(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => {
            return SwarmResponse::error(
                id,
                -32602,
                "Missing 'agent_id' parameter".into(),
            );
        }
    };

    let (known_agents, canonical_agent_id, swarm_id, epoch, hierarchy_assignments, announced_name) = {
        let mut state = state.write().await;
        let canonical_agent_id = state.agent_id.to_string();
        let requested_name = if requested_agent_id.starts_with("did:swarm:") {
            None
        } else {
            Some(requested_agent_id.as_str())
        };
        state.mark_member_seen_with_name(&canonical_agent_id, requested_name);
        state.push_log(
            crate::tui::LogCategory::System,
            format!(
                "Agent registered: requested={}, canonical={}",
                requested_agent_id, canonical_agent_id
            ),
        );

        let staleness = Duration::from_secs(ACTIVE_MEMBER_STALENESS_SECS);
        let active_members = state.active_member_ids(staleness);
        let swarm_size = active_members.len() as u64;
        let mut hierarchy_assignments = Vec::new();

        if swarm_size >= 2 {
            let mut sorted_agents: Vec<String> = active_members;
            sorted_agents.sort();

            state.agent_tiers.clear();
            state.agent_parents.clear();
            state.subordinates.clear();

            let k = dynamic_branching_factor(swarm_size) as usize;
            let distribution = openswarm_hierarchy::PyramidAllocator::distribute(swarm_size, k as u64);
            let tier_sizes: Vec<usize> = distribution.tiers.iter().map(|n| *n as usize).collect();
            let levels = tier_sizes.len().max(1);

            let mut offsets = Vec::with_capacity(levels + 1);
            offsets.push(0usize);
            for size in &tier_sizes {
                let prev = *offsets.last().unwrap_or(&0);
                offsets.push(prev + *size);
            }

            for level in 0..levels {
                let start = offsets[level];
                let end = *offsets.get(level + 1).unwrap_or(&start);
                for idx in start..end.min(sorted_agents.len()) {
                    let member_id = sorted_agents[idx].clone();
                    let tier = if levels == 1 {
                        Tier::Executor
                    } else if level == (levels - 1) {
                        Tier::Executor
                    } else if level == 0 {
                        Tier::Tier1
                    } else if level == 1 {
                        Tier::Tier2
                    } else {
                        Tier::TierN((level + 1) as u32)
                    };
                    state.agent_tiers.insert(member_id, tier);
                }
            }

            for level in 1..levels {
                let child_start = offsets[level];
                let child_end = *offsets.get(level + 1).unwrap_or(&child_start);
                let parent_start = offsets[level - 1];
                let parent_end = *offsets.get(level).unwrap_or(&parent_start);
                let parent_count = parent_end.saturating_sub(parent_start);
                if parent_count == 0 {
                    continue;
                }
                for child_idx in child_start..child_end.min(sorted_agents.len()) {
                    let local_child_idx = child_idx.saturating_sub(child_start);
                    let mut parent_local_idx = local_child_idx / k.max(1);
                    if parent_local_idx >= parent_count {
                        parent_local_idx = parent_count - 1;
                    }
                    let parent_idx = parent_start + parent_local_idx;
                    if let (Some(child_id), Some(parent_id)) =
                        (sorted_agents.get(child_idx), sorted_agents.get(parent_idx))
                    {
                        state.agent_parents.insert(child_id.clone(), parent_id.clone());
                        state
                            .subordinates
                            .entry(parent_id.clone())
                            .or_default()
                            .push(child_id.clone());
                    }
                }
            }

            for member_id in &sorted_agents {
                let tier = state
                    .agent_tiers
                    .get(member_id)
                    .copied()
                    .unwrap_or(Tier::Executor);
                let parent = state.agent_parents.get(member_id).cloned();
                hierarchy_assignments.push((member_id.clone(), tier, parent));
            }

            let my_agent_id = state.agent_id.as_str().to_string();
            if let Some(my_tier) = state.agent_tiers.get(&my_agent_id).copied() {
                state.my_tier = my_tier;
                state.parent_id = state.agent_parents.get(&my_agent_id).cloned().map(AgentId::new);
                state.network_stats.my_tier = my_tier;
                state.network_stats.parent_id = state.parent_id.clone();
                state.network_stats.subordinate_count = state
                    .subordinates
                    .get(&my_agent_id)
                    .map(|s| s.len())
                    .unwrap_or(0) as u32;
            }
            state.network_stats.hierarchy_depth = levels as u32;
            state.current_layout = openswarm_hierarchy::PyramidAllocator::new(openswarm_hierarchy::pyramid::PyramidConfig {
                branching_factor: k as u32,
                max_depth: openswarm_protocol::MAX_HIERARCHY_DEPTH,
            })
            .compute_layout(swarm_size)
            .ok();
        }

        let announced_name = if let Some(existing) = state.agent_names.get(&canonical_agent_id) {
            if existing.starts_with("did:swarm:") {
                None
            } else {
                Some(existing.clone())
            }
        } else if requested_agent_id.starts_with("did:swarm:") {
            None
        } else {
            Some(requested_agent_id.clone())
        };

        (
            state.active_member_count(staleness),
            canonical_agent_id,
            state.current_swarm_id.as_str().to_string(),
            state.epoch_manager.current_epoch(),
            hierarchy_assignments,
            announced_name,
        )
    };

    // Publish keepalive
    let keepalive = KeepAliveParams {
        agent_id: AgentId::new(canonical_agent_id.clone()),
        agent_name: announced_name,
        last_task_poll_at: None,
        last_result_at: None,
        epoch,
        timestamp: chrono::Utc::now(),
    };
    let msg = SwarmMessage::new(
        ProtocolMethod::AgentKeepAlive.as_str(),
        serde_json::to_value(&keepalive).unwrap_or_default(),
        String::new(),
    );
    if let Ok(data) = serde_json::to_vec(&msg) {
        let topic = SwarmTopics::keepalive_for(&swarm_id);
        let _ = network_handle.publish(&topic, data).await;
    }

    // Broadcast tier assignments if hierarchy was recomputed
    if !hierarchy_assignments.is_empty() {
        let branch_size = dynamic_branching_factor(known_agents as u64);
        for (member_id, tier, parent) in hierarchy_assignments {
            let params = TierAssignmentParams {
                assigned_agent: AgentId::new(member_id),
                tier,
                parent_id: parent.map(|p| AgentId::new(p)).unwrap_or_else(|| AgentId::new("root".to_string())),
                epoch,
                branch_size,
            };

            let msg = SwarmMessage::new(
                ProtocolMethod::TierAssignment.as_str(),
                serde_json::to_value(&params).unwrap_or_default(),
                String::new(),
            );

            if let Ok(data) = serde_json::to_vec(&msg) {
                let topic = SwarmTopics::hierarchy_for(&swarm_id);
                let _ = network_handle.publish(&topic, data).await;
            }
        }
    }

    SwarmResponse::success(
        id,
        serde_json::json!({
            "registered": true,
            "agent_id": canonical_agent_id,
            "requested_agent_id": requested_agent_id,
            "known_agents": known_agents,
        }),
    )
}

fn dynamic_branching_factor(swarm_size: u64) -> u64 {
    let approx = (swarm_size as f64).sqrt().round() as u64;
    approx.clamp(3, 10)
}

/// Handle `swarm.list_swarms` - list all known swarms with their info.
async fn handle_list_swarms(
    id: Option<String>,
    state: &Arc<RwLock<ConnectorState>>,
) -> SwarmResponse {
    let state = state.read().await;

    let swarms: Vec<serde_json::Value> = state
        .known_swarms
        .values()
        .map(|record| {
            serde_json::json!({
                "swarm_id": record.swarm_id.as_str(),
                "name": record.name,
                "is_public": record.is_public,
                "agent_count": record.agent_count,
                "joined": record.joined,
            })
        })
        .collect();

    SwarmResponse::success(
        id,
        serde_json::json!({
            "swarms": swarms,
            "current_swarm": state.current_swarm_id.as_str(),
        }),
    )
}

/// Handle `swarm.create_swarm` - create a new private swarm.
async fn handle_create_swarm(
    id: Option<String>,
    params: &serde_json::Value,
    state: &Arc<RwLock<ConnectorState>>,
) -> SwarmResponse {
    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => {
            return SwarmResponse::error(
                id,
                -32602,
                "Missing 'name' parameter".into(),
            );
        }
    };

    let secret = match params.get("secret").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            return SwarmResponse::error(
                id,
                -32602,
                "Missing 'secret' parameter".into(),
            );
        }
    };

    let swarm_id = SwarmId::generate();
    let token = SwarmToken::generate(&swarm_id, &secret);

    let record = SwarmRecord {
        swarm_id: swarm_id.clone(),
        name: name.clone(),
        is_public: false,
        agent_count: 1,
        joined: true,
        last_seen: chrono::Utc::now(),
    };

    {
        let mut state = state.write().await;
        state
            .known_swarms
            .insert(swarm_id.as_str().to_string(), record);
    }

    SwarmResponse::success(
        id,
        serde_json::json!({
            "swarm_id": swarm_id.as_str(),
            "token": token.as_str(),
            "name": name,
        }),
    )
}

/// Handle `swarm.join_swarm` - join an existing swarm.
async fn handle_join_swarm(
    id: Option<String>,
    params: &serde_json::Value,
    state: &Arc<RwLock<ConnectorState>>,
) -> SwarmResponse {
    let swarm_id_str = match params.get("swarm_id").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            return SwarmResponse::error(
                id,
                -32602,
                "Missing 'swarm_id' parameter".into(),
            );
        }
    };

    let token = params.get("token").and_then(|v| v.as_str()).map(String::from);

    let mut state = state.write().await;

    let record = match state.known_swarms.get_mut(&swarm_id_str) {
        Some(r) => r,
        None => {
            return SwarmResponse::error(
                id,
                -32001,
                format!("Unknown swarm: {}", swarm_id_str),
            );
        }
    };

    // Private swarms require a token.
    if !record.is_public && token.is_none() {
        return SwarmResponse::error(
            id,
            -32602,
            "Token required for private swarm".into(),
        );
    }

    record.joined = true;

    SwarmResponse::success(
        id,
        serde_json::json!({
            "swarm_id": swarm_id_str,
            "joined": true,
        }),
    )
}

/// Handle `swarm.inject_task` - inject a task into the swarm from the operator/external source.
pub(crate) async fn handle_inject_task(
    id: Option<String>,
    params: &serde_json::Value,
    state: &Arc<RwLock<ConnectorState>>,
    network_handle: &openswarm_network::SwarmHandle,
) -> SwarmResponse {
    let description = match params.get("description").and_then(|v| v.as_str()) {
        Some(d) => d.to_string(),
        None => {
            return SwarmResponse::error(
                id,
                -32602,
                "Missing 'description' parameter".into(),
            );
        }
    };

    let mut state_guard = state.write().await;
    let epoch = state_guard.epoch_manager.current_epoch();
    let task = openswarm_protocol::Task::new(description.clone(), 1, epoch);
    let task_id = task.task_id.clone();

    // Add task to the local task set (CRDT).
    state_guard.task_set.add(task_id.clone());
    state_guard.task_details.insert(task_id.clone(), task.clone());
    let actor = state_guard.agent_id.to_string();
    state_guard.push_task_timeline_event(
        &task_id,
        "injected",
        format!("Task injected via RPC: {}", description),
        Some(actor),
    );

    // Log the injection.
    state_guard.push_log(
        crate::tui::LogCategory::Task,
        format!("Task injected via RPC: {} ({})", task_id, description),
    );
    let audit_actor = state_guard.agent_id.to_string();
    state_guard.push_log(
        crate::tui::LogCategory::System,
        format!(
            "AUDIT task.inject actor={} task_id={} description={}",
            audit_actor, task_id, description
        ),
    );

    // Publish task injection to the swarm network.
    let inject_params = TaskInjectionParams {
        task: task.clone(),
        originator: state_guard.agent_id.clone(),
    };

    let msg = SwarmMessage::new(
        ProtocolMethod::TaskInjection.as_str(),
        serde_json::to_value(&inject_params).unwrap_or_default(),
        String::new(),
    );

    let swarm_id = state_guard.current_swarm_id.as_str().to_string();
    drop(state_guard);

    if let Ok(data) = serde_json::to_vec(&msg) {
        let topic = SwarmTopics::tasks_for(&swarm_id, 1);
        if let Err(e) = network_handle.publish(&topic, data).await {
            tracing::debug!(error = %e, "Failed to publish task injection");
        }

        let proposals_topic = SwarmTopics::proposals_for(&swarm_id, &task_id);
        let voting_topic = SwarmTopics::voting_for(&swarm_id, &task_id);
        let results_topic = SwarmTopics::results_for(&swarm_id, &task_id);

        if let Err(e) = network_handle.subscribe(&proposals_topic).await {
            tracing::debug!(error = %e, topic = %proposals_topic, "Failed to subscribe proposals topic");
        }
        if let Err(e) = network_handle.subscribe(&voting_topic).await {
            tracing::debug!(error = %e, topic = %voting_topic, "Failed to subscribe voting topic");
        }
        if let Err(e) = network_handle.subscribe(&results_topic).await {
            tracing::debug!(error = %e, topic = %results_topic, "Failed to subscribe results topic");
        }
    }

    SwarmResponse::success(
        id,
        serde_json::json!({
            "task_id": task_id,
            "description": description,
            "epoch": epoch,
            "injected": true,
        }),
    )
}

/// Handle `swarm.get_hierarchy` - return the agent hierarchy tree.
async fn handle_get_hierarchy(
    id: Option<String>,
    state: &Arc<RwLock<ConnectorState>>,
) -> SwarmResponse {
    let state = state.read().await;
    let active_members = state.active_member_ids(Duration::from_secs(ACTIVE_MEMBER_STALENESS_SECS));

    let self_agent = serde_json::json!({
        "agent_id": state.agent_id.to_string(),
        "tier": format!("{:?}", state.my_tier),
        "parent_id": state.parent_id.as_ref().map(|p| p.to_string()),
        "task_count": state.task_set.len(),
        "is_self": true,
    });

    let peers: Vec<serde_json::Value> = active_members
        .iter()
        .filter(|agent_id| *agent_id != &state.agent_id.to_string())
        .map(|peer_id| {
            let tier = state
                .agent_tiers
                .get(peer_id)
                .copied()
                .unwrap_or(Tier::Executor);
            let parent_id = state.agent_parents.get(peer_id).cloned();
            serde_json::json!({
                "agent_id": peer_id,
                "tier": format!("{:?}", tier),
                "parent_id": parent_id,
                "task_count": 0,
                "is_self": false,
            })
        })
        .collect();

    SwarmResponse::success(
        id,
        serde_json::json!({
            "self": self_agent,
            "peers": peers,
            "total_agents": active_members.len(),
            "hierarchy_depth": state.network_stats.hierarchy_depth,
            "branching_factor": state.network_stats.branching_factor,
            "epoch": state.epoch_manager.current_epoch(),
        }),
    )
}
