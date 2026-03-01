//! Request for Proposal (RFP) protocol: task injection, plan generation,
//! and commit-reveal scheme.
//!
//! The RFP protocol ensures fair plan selection by using a two-phase
//! commit-reveal approach:
//!
//! 1. **Commit Phase**: Each Tier-1 agent generates a plan and publishes
//!    only the SHA-256 hash of the plan. This prevents copying.
//! 2. **Reveal Phase**: After all commits are received (or timeout),
//!    agents reveal their full plans. Plans must match their committed hash.
//! 3. **Evaluation**: Plans are passed to voting for selection.
//!
//! Plan generation is delegated to a `PlanGenerator` trait that abstracts
//! the LLM/AI component, allowing different backends.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use wws_protocol::{
    AgentId, CriticScore, Plan, ProposalCommitParams, ProposalRevealParams, Task,
    COMMIT_REVEAL_TIMEOUT_SECS,
};

use crate::ConsensusError;

// ---------------------------------------------------------------------------
// Plan Generator trait
// ---------------------------------------------------------------------------

/// Context provided to the plan generator for creating decomposition plans.
#[derive(Debug, Clone)]
pub struct PlanContext {
    /// The task to decompose.
    pub task: Task,
    /// Current epoch number.
    pub epoch: u64,
    /// Number of agents available at the next tier level.
    pub available_agents: u64,
    /// Branching factor (k) of the hierarchy.
    pub branching_factor: u32,
    /// Capabilities of known agents (for informed decomposition).
    pub known_capabilities: Vec<String>,
}

/// Trait for plan generation, abstracting the LLM/AI component.
///
/// Implementations connect to different AI backends (e.g., GPT-4, Claude, local models)
/// to generate task decomposition plans.
pub trait PlanGenerator: Send + Sync {
    /// Generate a decomposition plan for the given task and context.
    ///
    /// The implementation should:
    /// 1. Analyze the task description
    /// 2. Consider available agents and their capabilities
    /// 3. Produce a set of subtasks that collectively solve the task
    /// 4. Estimate parallelism and complexity
    fn generate_plan<'a>(
        &'a self,
        context: &'a PlanContext,
    ) -> Pin<Box<dyn Future<Output = Result<Plan, ConsensusError>> + Send + 'a>>;
}

// ---------------------------------------------------------------------------
// RFP State Machine
// ---------------------------------------------------------------------------

/// State of an RFP round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RfpPhase {
    /// Waiting for task injection.
    Idle,
    /// Commit phase: collecting plan hashes.
    CommitPhase,
    /// Reveal phase: collecting full plans.
    RevealPhase,
    /// Critique phase: members score each other's proposals.
    CritiquePhase,
    /// All plans revealed; ready for voting.
    ReadyForVoting,
    /// RFP completed (plan selected).
    Completed,
}

/// A committed but not yet revealed proposal.
#[derive(Debug, Clone)]
struct PendingCommit {
    #[allow(dead_code)]
    proposer: AgentId,
    plan_hash: String,
    #[allow(dead_code)]
    committed_at: DateTime<Utc>,
}

/// A fully revealed proposal.
#[derive(Debug, Clone)]
pub struct RevealedProposal {
    pub proposer: AgentId,
    pub plan: Plan,
    pub plan_hash: String,
}

/// Coordinates the Request for Proposal process for a single task.
///
/// Lifecycle:
/// 1. `inject_task()` - start the RFP
/// 2. `record_commit()` - collect plan hash commits
/// 3. `transition_to_reveal()` - move to reveal phase
/// 4. `record_reveal()` - collect and verify revealed plans
/// 5. `transition_to_critique()` - move to critique phase (optional)
/// 6. `finalize()` - get all verified proposals for voting
pub struct RfpCoordinator {
    task_id: String,
    epoch: u64,
    phase: RfpPhase,
    /// Pending commits (hash only).
    commits: HashMap<AgentId, PendingCommit>,
    /// Verified revealed proposals.
    pub reveals: HashMap<AgentId, RevealedProposal>,
    /// When the commit phase started.
    commit_started_at: Option<DateTime<Utc>>,
    /// Timeout duration for commit phase.
    commit_timeout_secs: u64,
    /// Expected number of proposers (Tier-1 agents).
    expected_proposers: usize,
    /// Critique plan scores received during critique phase.
    pub critique_scores: HashMap<AgentId, HashMap<String, CriticScore>>,
    /// Critique content messages.
    pub critique_content: HashMap<AgentId, String>,
}

impl RfpCoordinator {
    /// Create a new RFP coordinator for a task.
    pub fn new(task_id: String, epoch: u64, expected_proposers: usize) -> Self {
        Self {
            task_id,
            epoch,
            phase: RfpPhase::Idle,
            commits: HashMap::new(),
            reveals: HashMap::new(),
            commit_started_at: None,
            commit_timeout_secs: COMMIT_REVEAL_TIMEOUT_SECS,
            expected_proposers,
            critique_scores: HashMap::new(),
            critique_content: HashMap::new(),
        }
    }

    /// Start the RFP by injecting a task. Moves to CommitPhase.
    pub fn inject_task(&mut self, task: &Task) -> Result<(), ConsensusError> {
        if self.phase != RfpPhase::Idle {
            return Err(ConsensusError::RfpFailed(format!(
                "Cannot inject task in phase {:?}",
                self.phase
            )));
        }

        if task.task_id != self.task_id {
            return Err(ConsensusError::TaskNotFound(self.task_id.clone()));
        }

        self.phase = RfpPhase::CommitPhase;
        self.commit_started_at = Some(Utc::now());

        tracing::info!(
            task_id = %self.task_id,
            epoch = self.epoch,
            expected_proposers = self.expected_proposers,
            "RFP commit phase started"
        );

        Ok(())
    }

    /// Record a commit (plan hash) from a proposer.
    pub fn record_commit(
        &mut self,
        params: &ProposalCommitParams,
    ) -> Result<(), ConsensusError> {
        if matches!(self.phase, RfpPhase::RevealPhase | RfpPhase::ReadyForVoting)
            && self.commits.len() < self.expected_proposers
        {
            self.phase = RfpPhase::CommitPhase;
            tracing::warn!(
                task_id = %self.task_id,
                commits = self.commits.len(),
                expected = self.expected_proposers,
                "Reopening commit phase due additional expected proposers"
            );
        }

        if self.phase != RfpPhase::CommitPhase {
            return Err(ConsensusError::RfpFailed(format!(
                "Not in commit phase (currently {:?})",
                self.phase
            )));
        }

        if params.task_id != self.task_id {
            return Err(ConsensusError::TaskNotFound(self.task_id.clone()));
        }

        if params.epoch != self.epoch {
            return Err(ConsensusError::EpochMismatch {
                expected: self.epoch,
                got: params.epoch,
            });
        }

        if self.commits.contains_key(&params.proposer) {
            return Err(ConsensusError::DuplicateCommit(
                self.task_id.clone(),
                params.proposer.to_string(),
            ));
        }

        self.commits.insert(
            params.proposer.clone(),
            PendingCommit {
                proposer: params.proposer.clone(),
                plan_hash: params.plan_hash.clone(),
                committed_at: Utc::now(),
            },
        );

        tracing::debug!(
            task_id = %self.task_id,
            proposer = %params.proposer,
            commits = self.commits.len(),
            expected = self.expected_proposers,
            "Recorded proposal commit"
        );

        // Auto-transition if all expected commits received.
        if self.commits.len() >= self.expected_proposers {
            self.phase = RfpPhase::RevealPhase;
            tracing::info!(
                task_id = %self.task_id,
                "All commits received, transitioning to reveal phase"
            );
        }

        Ok(())
    }

    /// Manually transition to reveal phase (e.g., on timeout).
    pub fn transition_to_reveal(&mut self) -> Result<(), ConsensusError> {
        if self.phase != RfpPhase::CommitPhase {
            return Err(ConsensusError::RfpFailed(format!(
                "Cannot transition to reveal from {:?}",
                self.phase
            )));
        }

        if self.commits.is_empty() {
            return Err(ConsensusError::NoProposals(self.task_id.clone()));
        }

        self.phase = RfpPhase::RevealPhase;
        tracing::info!(
            task_id = %self.task_id,
            commits = self.commits.len(),
            "Transitioning to reveal phase (timeout or manual)"
        );
        Ok(())
    }

    /// Check if the commit phase has timed out.
    pub fn is_commit_timed_out(&self) -> bool {
        if let Some(started) = self.commit_started_at {
            let elapsed = Utc::now()
                .signed_duration_since(started)
                .num_seconds() as u64;
            elapsed >= self.commit_timeout_secs
        } else {
            false
        }
    }

    /// Record a reveal (full plan) from a proposer.
    ///
    /// Verifies that the plan's hash matches the previously committed hash.
    pub fn record_reveal(
        &mut self,
        params: &ProposalRevealParams,
    ) -> Result<(), ConsensusError> {
        if self.phase != RfpPhase::RevealPhase {
            return Err(ConsensusError::RfpFailed(format!(
                "Not in reveal phase (currently {:?})",
                self.phase
            )));
        }

        if params.task_id != self.task_id {
            return Err(ConsensusError::TaskNotFound(self.task_id.clone()));
        }

        let proposer = &params.plan.proposer;

        // Verify the reveal matches the commit.
        let commit = self.commits.get(proposer).ok_or_else(|| {
            ConsensusError::RfpFailed(format!(
                "No commit found for proposer {}",
                proposer
            ))
        })?;

        // Compute hash of the revealed plan.
        let plan_json = serde_json::to_vec(&params.plan)
            .map_err(|e| ConsensusError::Serialization(e.to_string()))?;
        let computed_hash = hex_encode(&Sha256::digest(&plan_json));

        if computed_hash != commit.plan_hash {
            return Err(ConsensusError::HashMismatch {
                expected: commit.plan_hash.clone(),
                got: computed_hash,
            });
        }

        self.reveals.insert(
            proposer.clone(),
            RevealedProposal {
                proposer: proposer.clone(),
                plan: params.plan.clone(),
                plan_hash: computed_hash,
            },
        );

        tracing::debug!(
            task_id = %self.task_id,
            proposer = %proposer,
            reveals = self.reveals.len(),
            "Recorded proposal reveal"
        );

        // Auto-transition if all committed proposals have been revealed.
        if self.reveals.len() >= self.commits.len() {
            self.phase = RfpPhase::ReadyForVoting;
            tracing::info!(
                task_id = %self.task_id,
                proposals = self.reveals.len(),
                "All proposals revealed, ready for voting"
            );
        }

        Ok(())
    }

    /// Transition from RevealPhase to CritiquePhase.
    pub fn transition_to_critique(&mut self) -> Result<(), ConsensusError> {
        if !matches!(self.phase, RfpPhase::RevealPhase | RfpPhase::ReadyForVoting) {
            return Err(ConsensusError::RfpFailed(format!(
                "Cannot transition to critique from {:?}",
                self.phase
            )));
        }
        self.phase = RfpPhase::CritiquePhase;
        tracing::info!(
            task_id = %self.task_id,
            "Transitioning to critique phase"
        );
        Ok(())
    }

    /// Record a critique from a board member.
    pub fn record_critique(
        &mut self,
        voter: AgentId,
        plan_scores: HashMap<String, CriticScore>,
        content: String,
    ) -> Result<(), ConsensusError> {
        self.critique_scores.insert(voter.clone(), plan_scores);
        self.critique_content.insert(voter, content);
        Ok(())
    }

    /// Transition from CritiquePhase to ReadyForVoting.
    pub fn transition_to_voting(&mut self) -> Result<(), ConsensusError> {
        if !matches!(
            self.phase,
            RfpPhase::CritiquePhase | RfpPhase::RevealPhase | RfpPhase::ReadyForVoting
        ) {
            return Err(ConsensusError::RfpFailed(format!(
                "Cannot transition to voting from {:?}",
                self.phase
            )));
        }
        self.phase = RfpPhase::ReadyForVoting;
        tracing::info!(
            task_id = %self.task_id,
            "Transitioning to ready-for-voting phase"
        );
        Ok(())
    }

    /// Finalize the RFP and get all verified proposals for voting.
    pub fn finalize(&mut self) -> Result<Vec<RevealedProposal>, ConsensusError> {
        if self.phase != RfpPhase::ReadyForVoting && self.phase != RfpPhase::RevealPhase {
            return Err(ConsensusError::RfpFailed(format!(
                "Cannot finalize in phase {:?}",
                self.phase
            )));
        }

        if self.reveals.is_empty() {
            return Err(ConsensusError::NoProposals(self.task_id.clone()));
        }

        self.phase = RfpPhase::Completed;

        let proposals: Vec<RevealedProposal> = self.reveals.values().cloned().collect();

        tracing::info!(
            task_id = %self.task_id,
            proposals = proposals.len(),
            "RFP finalized"
        );

        Ok(proposals)
    }

    /// Get the current phase.
    pub fn phase(&self) -> &RfpPhase {
        &self.phase
    }

    /// Get the task ID.
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    /// Get the number of commits received.
    pub fn commit_count(&self) -> usize {
        self.commits.len()
    }

    /// Get the number of reveals received.
    pub fn reveal_count(&self) -> usize {
        self.reveals.len()
    }

    /// Debug view of committed hashes: (proposer, plan_hash).
    pub fn commits_for_debug(&self) -> Vec<(String, String)> {
        self.commits
            .iter()
            .map(|(agent, pending)| (agent.to_string(), pending.plan_hash.clone()))
            .collect()
    }

    /// Compute the commit hash for a plan (for use by proposers).
    pub fn compute_plan_hash(plan: &Plan) -> Result<String, ConsensusError> {
        let plan_json = serde_json::to_vec(plan)
            .map_err(|e| ConsensusError::Serialization(e.to_string()))?;
        Ok(hex_encode(&Sha256::digest(&plan_json)))
    }
}

/// Hex-encode a byte slice.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wws_protocol::PlanSubtask;

    fn make_plan(task_id: &str, proposer: &str, epoch: u64) -> Plan {
        let mut plan = Plan::new(
            task_id.to_string(),
            AgentId::new(proposer.to_string()),
            epoch,
        );
        plan.subtasks.push(PlanSubtask {
            index: 0,
            description: "Subtask A".to_string(),
            required_capabilities: vec!["python".to_string()],
            estimated_complexity: 0.5,
        });
        plan.rationale = "Test plan".to_string();
        plan
    }

    #[test]
    fn test_rfp_lifecycle() {
        let task = Task::new("Test task".into(), 1, 1);
        let task_id = task.task_id.clone();
        let mut rfp = RfpCoordinator::new(task_id.clone(), 1, 1);

        // Inject task.
        rfp.inject_task(&task).unwrap();
        assert_eq!(*rfp.phase(), RfpPhase::CommitPhase);

        // Create and commit a plan.
        let plan = make_plan(&task_id, "alice", 1);
        let hash = RfpCoordinator::compute_plan_hash(&plan).unwrap();

        rfp.record_commit(&ProposalCommitParams {
            task_id: task_id.clone(),
            proposer: AgentId::new("alice".into()),
            epoch: 1,
            plan_hash: hash,
        })
        .unwrap();

        // Should auto-transition since expected_proposers = 1.
        assert_eq!(*rfp.phase(), RfpPhase::RevealPhase);

        // Reveal.
        rfp.record_reveal(&ProposalRevealParams {
            task_id: task_id.clone(),
            plan,
        })
        .unwrap();

        assert_eq!(*rfp.phase(), RfpPhase::ReadyForVoting);

        // Finalize.
        let proposals = rfp.finalize().unwrap();
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].proposer, AgentId::new("alice".into()));
    }

    #[test]
    fn test_hash_mismatch_rejected() {
        let task = Task::new("Test".into(), 1, 1);
        let task_id = task.task_id.clone();
        let mut rfp = RfpCoordinator::new(task_id.clone(), 1, 1);
        rfp.inject_task(&task).unwrap();

        // Commit with a fake hash.
        rfp.record_commit(&ProposalCommitParams {
            task_id: task_id.clone(),
            proposer: AgentId::new("alice".into()),
            epoch: 1,
            plan_hash: "fake_hash".into(),
        })
        .unwrap();

        // Reveal with a different plan.
        let plan = make_plan(&task_id, "alice", 1);
        let result = rfp.record_reveal(&ProposalRevealParams {
            task_id: task_id.clone(),
            plan,
        });

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConsensusError::HashMismatch { .. }));
    }

    #[test]
    fn test_critique_phase_transition() {
        let task = Task::new("Critique test".into(), 1, 1);
        let task_id = task.task_id.clone();
        let mut rfp = RfpCoordinator::new(task_id.clone(), 1, 1);
        rfp.inject_task(&task).unwrap();

        let plan = make_plan(&task_id, "alice", 1);
        let hash = RfpCoordinator::compute_plan_hash(&plan).unwrap();
        rfp.record_commit(&ProposalCommitParams {
            task_id: task_id.clone(),
            proposer: AgentId::new("alice".into()),
            epoch: 1,
            plan_hash: hash,
        }).unwrap();

        rfp.record_reveal(&ProposalRevealParams {
            task_id: task_id.clone(),
            plan: plan.clone(),
        }).unwrap();

        assert_eq!(*rfp.phase(), RfpPhase::ReadyForVoting);

        // Transition to critique phase.
        rfp.transition_to_critique().unwrap();
        assert_eq!(*rfp.phase(), RfpPhase::CritiquePhase);

        // Record a critique.
        let mut scores = HashMap::new();
        scores.insert(plan.plan_id.clone(), CriticScore {
            feasibility: 0.9,
            parallelism: 0.8,
            completeness: 0.85,
            risk: 0.1,
        });
        rfp.record_critique(AgentId::new("bob".into()), scores, "Looks good".to_string()).unwrap();
        assert_eq!(rfp.critique_scores.len(), 1);

        // Transition to voting.
        rfp.transition_to_voting().unwrap();
        assert_eq!(*rfp.phase(), RfpPhase::ReadyForVoting);
    }

    #[test]
    fn test_multiple_critiques_from_different_agents() {
        let task = Task::new("Multi-critique test".into(), 1, 1);
        let task_id = task.task_id.clone();
        let mut rfp = RfpCoordinator::new(task_id.clone(), 1, 2); // 2 proposers expected

        rfp.inject_task(&task).unwrap();

        // Two proposers commit and reveal
        let plan_alice = make_plan(&task_id, "alice", 1);
        let plan_bob = make_plan(&task_id, "bob", 1);
        let hash_alice = RfpCoordinator::compute_plan_hash(&plan_alice).unwrap();
        let hash_bob = RfpCoordinator::compute_plan_hash(&plan_bob).unwrap();

        rfp.record_commit(&ProposalCommitParams {
            task_id: task_id.clone(),
            proposer: AgentId::new("alice".into()),
            epoch: 1,
            plan_hash: hash_alice,
        }).unwrap();
        rfp.record_commit(&ProposalCommitParams {
            task_id: task_id.clone(),
            proposer: AgentId::new("bob".into()),
            epoch: 1,
            plan_hash: hash_bob,
        }).unwrap();

        // Should auto-transition to reveal after 2 commits
        assert_eq!(*rfp.phase(), RfpPhase::RevealPhase);

        rfp.record_reveal(&ProposalRevealParams { task_id: task_id.clone(), plan: plan_alice.clone() }).unwrap();
        rfp.record_reveal(&ProposalRevealParams { task_id: task_id.clone(), plan: plan_bob.clone() }).unwrap();
        assert_eq!(*rfp.phase(), RfpPhase::ReadyForVoting);

        // Transition to critique
        rfp.transition_to_critique().unwrap();
        assert_eq!(*rfp.phase(), RfpPhase::CritiquePhase);

        // Three different agents provide critiques
        let agents = ["carol", "dave", "eve"];
        for agent_name in &agents {
            let mut scores = HashMap::new();
            scores.insert(plan_alice.plan_id.clone(), CriticScore {
                feasibility: 0.8,
                parallelism: 0.7,
                completeness: 0.9,
                risk: 0.15,
            });
            scores.insert(plan_bob.plan_id.clone(), CriticScore {
                feasibility: 0.6,
                parallelism: 0.5,
                completeness: 0.7,
                risk: 0.3,
            });
            rfp.record_critique(
                AgentId::new(agent_name.to_string()),
                scores,
                format!("Analysis from {}", agent_name),
            ).unwrap();
        }

        assert_eq!(rfp.critique_scores.len(), 3);
        assert_eq!(rfp.critique_content.len(), 3);

        // All content present
        let carol = AgentId::new("carol".to_string());
        assert!(rfp.critique_content[&carol].contains("carol"));

        // Transition to voting
        rfp.transition_to_voting().unwrap();
        assert_eq!(*rfp.phase(), RfpPhase::ReadyForVoting);
    }

    #[test]
    fn test_critique_overwrites_existing_for_same_agent() {
        let task = Task::new("Overwrite critique test".into(), 1, 1);
        let task_id = task.task_id.clone();
        let mut rfp = RfpCoordinator::new(task_id.clone(), 1, 1);
        rfp.inject_task(&task).unwrap();

        let plan = make_plan(&task_id, "alice", 1);
        let hash = RfpCoordinator::compute_plan_hash(&plan).unwrap();
        rfp.record_commit(&ProposalCommitParams {
            task_id: task_id.clone(),
            proposer: AgentId::new("alice".into()),
            epoch: 1,
            plan_hash: hash,
        }).unwrap();
        rfp.record_reveal(&ProposalRevealParams {
            task_id: task_id.clone(),
            plan: plan.clone(),
        }).unwrap();
        rfp.transition_to_critique().unwrap();

        let voter = AgentId::new("bob".into());
        let mut scores1 = HashMap::new();
        scores1.insert(plan.plan_id.clone(), CriticScore {
            feasibility: 0.5,
            parallelism: 0.5,
            completeness: 0.5,
            risk: 0.5,
        });
        rfp.record_critique(voter.clone(), scores1, "First critique".to_string()).unwrap();
        assert_eq!(rfp.critique_scores.len(), 1);
        assert_eq!(rfp.critique_content[&voter], "First critique");

        // Bob updates his critique
        let mut scores2 = HashMap::new();
        scores2.insert(plan.plan_id.clone(), CriticScore {
            feasibility: 0.9,
            parallelism: 0.9,
            completeness: 0.9,
            risk: 0.1,
        });
        rfp.record_critique(voter.clone(), scores2, "Updated critique".to_string()).unwrap();

        // Still only 1 entry (overwritten)
        assert_eq!(rfp.critique_scores.len(), 1);
        assert_eq!(rfp.critique_content[&voter], "Updated critique");
        assert!((rfp.critique_scores[&voter][&plan.plan_id].feasibility - 0.9).abs() < 1e-10);
    }

    #[test]
    fn test_transition_to_critique_invalid_from_commit_phase() {
        let task = Task::new("Invalid transition test".into(), 1, 1);
        let task_id = task.task_id.clone();
        let mut rfp = RfpCoordinator::new(task_id.clone(), 1, 1);
        rfp.inject_task(&task).unwrap();

        // Still in CommitPhase — cannot transition to critique
        assert_eq!(*rfp.phase(), RfpPhase::CommitPhase);
        let result = rfp.transition_to_critique();
        assert!(result.is_err());
        // Phase unchanged
        assert_eq!(*rfp.phase(), RfpPhase::CommitPhase);
    }

    #[test]
    fn test_rfp_critique_content_stored_per_agent() {
        let task = Task::new("Content storage test".into(), 1, 1);
        let task_id = task.task_id.clone();
        let mut rfp = RfpCoordinator::new(task_id.clone(), 1, 1);
        rfp.inject_task(&task).unwrap();

        let plan = make_plan(&task_id, "alice", 1);
        let hash = RfpCoordinator::compute_plan_hash(&plan).unwrap();
        rfp.record_commit(&ProposalCommitParams {
            task_id: task_id.clone(),
            proposer: AgentId::new("alice".into()),
            epoch: 1,
            plan_hash: hash,
        }).unwrap();
        rfp.record_reveal(&ProposalRevealParams { task_id: task_id.clone(), plan: plan.clone() }).unwrap();
        rfp.transition_to_critique().unwrap();

        let long_content = "This is a very detailed critique covering feasibility, \
            technical risk, parallelism potential, and completeness of the proposed \
            decomposition strategy for the given task.";

        rfp.record_critique(
            AgentId::new("critic1".into()),
            HashMap::new(),
            long_content.to_string(),
        ).unwrap();

        assert_eq!(rfp.critique_content[&AgentId::new("critic1".into())], long_content);
    }
}
