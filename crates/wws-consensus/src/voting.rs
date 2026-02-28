//! Ranked Choice Voting with Instant Runoff Voting (IRV) algorithm.
//!
//! Implements the plan selection mechanism:
//! - Agents rank proposed plans from most to least preferred
//! - Self-vote prohibition: an agent cannot rank their own proposal first
//! - Senate sampling: for large swarms, a random subset votes to reduce overhead
//! - IRV elimination: if no plan has majority, the plan with fewest first-choice
//!   votes is eliminated and its votes are redistributed
//!
//! The IRV algorithm:
//! 1. Count first-choice votes for each plan
//! 2. If a plan has > 50% of first-choice votes, it wins
//! 3. Otherwise, eliminate the plan with the fewest first-choice votes
//! 4. Redistribute eliminated plan's votes to each voter's next preference
//! 5. Repeat until a plan has majority or one plan remains

use std::collections::{HashMap, HashSet};

use wws_protocol::{AgentId, CriticScore, RankedVote};
use rand::seq::SliceRandom;

use crate::ConsensusError;

/// Configuration for the voting engine.
#[derive(Debug, Clone)]
pub struct VotingConfig {
    /// Maximum number of voters (senate size). If the swarm is larger,
    /// a random subset is selected.
    pub senate_size: usize,
    /// Whether self-voting is prohibited (agent cannot rank own plan first).
    pub prohibit_self_vote: bool,
    /// Minimum number of votes required for a valid election.
    pub min_votes: usize,
    /// Random seed for reproducible senate sampling (None = random).
    pub senate_seed: Option<u64>,
}

impl Default for VotingConfig {
    fn default() -> Self {
        Self {
            senate_size: 100,
            prohibit_self_vote: true,
            min_votes: 1,
            senate_seed: None,
        }
    }
}

/// Result of a voting round.
#[derive(Debug, Clone)]
pub struct VotingResult {
    /// The winning plan ID.
    pub winner: String,
    /// Number of IRV rounds it took.
    pub rounds: usize,
    /// Plan IDs in elimination order (first eliminated = weakest).
    pub elimination_order: Vec<String>,
    /// Final vote counts for remaining plans.
    pub final_tallies: HashMap<String, usize>,
    /// Total number of votes processed.
    pub total_votes: usize,
    /// Aggregate critic scores for the winning plan.
    pub winner_critic_score: Option<CriticScore>,
}

/// A single ballot in the IRV system.
#[derive(Debug, Clone)]
pub struct Ballot {
    pub voter: AgentId,
    /// Remaining ranked choices (first = most preferred).
    pub remaining_choices: Vec<String>,
    /// Critic scores provided by this voter.
    pub critic_scores: HashMap<String, CriticScore>,
    /// Original rankings before any IRV elimination (for record-keeping).
    pub original_rankings: Vec<String>,
}

/// Coordinates Ranked Choice Voting with Instant Runoff for plan selection.
///
/// Lifecycle:
/// 1. `set_proposals()` - register the plan IDs being voted on
/// 2. `record_vote()` - collect ranked ballots from agents
/// 3. `run_irv()` - execute the IRV algorithm and determine the winner
pub struct VotingEngine {
    config: VotingConfig,
    task_id: String,
    epoch: u64,
    /// Plan IDs eligible for voting.
    proposal_ids: HashSet<String>,
    /// Map from plan ID to proposer agent ID (for self-vote checking).
    plan_proposers: HashMap<String, AgentId>,
    /// Collected ballots.
    pub ballots: Vec<Ballot>,
    /// Agents selected for the senate (if sampling).
    senate: Option<HashSet<AgentId>>,
    /// Whether voting has been finalized.
    finalized: bool,
    /// IRV round history (populated after run_irv()).
    pub irv_rounds: Vec<wws_protocol::IrvRound>,
}

impl VotingEngine {
    /// Create a new voting engine for a specific task in a specific epoch.
    pub fn new(config: VotingConfig, task_id: String, epoch: u64) -> Self {
        Self {
            config,
            task_id,
            epoch,
            proposal_ids: HashSet::new(),
            plan_proposers: HashMap::new(),
            ballots: Vec::new(),
            senate: None,
            finalized: false,
            irv_rounds: Vec::new(),
        }
    }

    /// Register the proposals being voted on.
    ///
    /// `proposals` maps plan_id to the proposer's agent_id.
    pub fn set_proposals(&mut self, proposals: HashMap<String, AgentId>) {
        for (plan_id, proposer) in &proposals {
            self.proposal_ids.insert(plan_id.clone());
            self.plan_proposers
                .insert(plan_id.clone(), proposer.clone());
        }
    }

    /// Select a senate from the list of eligible voters.
    ///
    /// If the voter pool is larger than `senate_size`, a random subset
    /// is selected to keep voting overhead bounded.
    pub fn select_senate(&mut self, eligible_voters: &[AgentId]) {
        if eligible_voters.len() <= self.config.senate_size {
            self.senate = Some(eligible_voters.iter().cloned().collect());
            return;
        }

        let mut rng = if let Some(seed) = self.config.senate_seed {
            use rand::SeedableRng;
            rand::rngs::StdRng::seed_from_u64(seed)
        } else {
            use rand::SeedableRng;
            rand::rngs::StdRng::from_entropy()
        };

        let mut voters = eligible_voters.to_vec();
        voters.shuffle(&mut rng);
        let senate: HashSet<AgentId> =
            voters.into_iter().take(self.config.senate_size).collect();

        tracing::info!(
            task_id = %self.task_id,
            senate_size = senate.len(),
            total_eligible = eligible_voters.len(),
            "Senate selected for voting"
        );

        self.senate = Some(senate);
    }

    /// Record a ranked choice vote from an agent.
    ///
    /// Validates:
    /// - The voter is in the senate (if senate sampling is active)
    /// - Self-vote prohibition (voter cannot rank own plan first)
    /// - All ranked plan IDs are valid proposals
    pub fn record_vote(&mut self, vote: RankedVote) -> Result<(), ConsensusError> {
        if self.finalized {
            return Err(ConsensusError::VotingError(
                "Voting already finalized".into(),
            ));
        }

        if vote.task_id != self.task_id {
            return Err(ConsensusError::TaskNotFound(self.task_id.clone()));
        }

        if vote.epoch != self.epoch {
            return Err(ConsensusError::EpochMismatch {
                expected: self.epoch,
                got: vote.epoch,
            });
        }

        // Check senate membership.
        if let Some(ref senate) = self.senate {
            if !senate.contains(&vote.voter) {
                return Err(ConsensusError::VotingError(format!(
                    "Agent {} is not in the senate",
                    vote.voter
                )));
            }
        }

        // Self-vote prohibition: voter cannot rank their own plan first.
        if self.config.prohibit_self_vote && self.proposal_ids.len() > 1 {
            if let Some(first_choice) = vote.rankings.first() {
                if let Some(proposer) = self.plan_proposers.get(first_choice) {
                    if proposer == &vote.voter {
                        return Err(ConsensusError::SelfVoteProhibited(
                            vote.voter.to_string(),
                        ));
                    }
                }
            }
        }

        // Filter rankings to only include valid proposal IDs.
        let valid_rankings: Vec<String> = vote
            .rankings
            .iter()
            .filter(|id| self.proposal_ids.contains(*id))
            .cloned()
            .collect();

        if valid_rankings.is_empty() {
            return Err(ConsensusError::VotingError(
                "No valid proposals in rankings".into(),
            ));
        }

        self.ballots.push(Ballot {
            voter: vote.voter.clone(),
            original_rankings: valid_rankings.clone(),
            remaining_choices: valid_rankings,
            critic_scores: vote.critic_scores,
        });

        tracing::debug!(
            task_id = %self.task_id,
            voter = %vote.voter,
            ballots = self.ballots.len(),
            "Recorded vote"
        );

        Ok(())
    }

    /// Execute the Instant Runoff Voting algorithm.
    ///
    /// Returns the winning plan and metadata about the election process.
    pub fn run_irv(&mut self) -> Result<VotingResult, ConsensusError> {
        if self.ballots.len() < self.config.min_votes {
            return Err(ConsensusError::NoVotes(self.task_id.clone()));
        }

        let mut active_ballots: Vec<Ballot> = self.ballots.clone();
        let mut eliminated: HashSet<String> = HashSet::new();
        let mut elimination_order: Vec<String> = Vec::new();
        let mut round = 0;

        loop {
            round += 1;

            // Count first-choice votes for each active proposal.
            let mut tallies: HashMap<String, usize> = HashMap::new();
            for proposal_id in &self.proposal_ids {
                if !eliminated.contains(proposal_id) {
                    tallies.insert(proposal_id.clone(), 0);
                }
            }

            let mut valid_ballot_count = 0;
            for ballot in &active_ballots {
                if let Some(first_choice) = ballot
                    .remaining_choices
                    .iter()
                    .find(|id| !eliminated.contains(*id))
                {
                    *tallies.entry(first_choice.clone()).or_insert(0) += 1;
                    valid_ballot_count += 1;
                }
            }

            if tallies.is_empty() || valid_ballot_count == 0 {
                return Err(ConsensusError::VotingError(
                    "All proposals eliminated with no winner".into(),
                ));
            }

            let majority_threshold = valid_ballot_count / 2 + 1;

            tracing::debug!(
                round,
                tallies = ?tallies,
                threshold = majority_threshold,
                "IRV round"
            );

            // Check for majority winner.
            if let Some((winner, &count)) = tallies
                .iter()
                .max_by_key(|(_, &count)| count)
            {
                if count >= majority_threshold || tallies.len() == 1 {
                    // Record final round (no elimination).
                    self.irv_rounds.push(wws_protocol::IrvRound {
                        task_id: self.task_id.clone(),
                        round_number: round as u32,
                        tallies: tallies.clone(),
                        eliminated: None,
                        continuing_candidates: tallies.keys().cloned().collect(),
                    });

                    let winner_critic = self.aggregate_critic_scores(&winner);
                    self.finalized = true;

                    return Ok(VotingResult {
                        winner: winner.clone(),
                        rounds: round,
                        elimination_order,
                        final_tallies: tallies,
                        total_votes: self.ballots.len(),
                        winner_critic_score: winner_critic,
                    });
                }
            }

            // Find the plan with fewest first-choice votes (to eliminate).
            let (to_eliminate, _) = tallies
                .iter()
                .min_by_key(|(_, &count)| count)
                .expect("tallies is non-empty");

            tracing::debug!(
                round,
                eliminated = %to_eliminate,
                "Eliminating plan with fewest first-choice votes"
            );

            // Record this elimination round.
            let continuing: Vec<String> = tallies.keys()
                .filter(|k| k.as_str() != to_eliminate.as_str())
                .cloned()
                .collect();
            self.irv_rounds.push(wws_protocol::IrvRound {
                task_id: self.task_id.clone(),
                round_number: round as u32,
                tallies: tallies.clone(),
                eliminated: Some(to_eliminate.clone()),
                continuing_candidates: continuing,
            });

            eliminated.insert(to_eliminate.clone());
            elimination_order.push(to_eliminate.clone());

            // Remove eliminated choices from all ballots.
            for ballot in &mut active_ballots {
                ballot
                    .remaining_choices
                    .retain(|id| !eliminated.contains(id));
            }
        }
    }

    /// Get IRV round history (populated after run_irv).
    pub fn irv_rounds(&self) -> &[wws_protocol::IrvRound] {
        &self.irv_rounds
    }

    /// Get ballot data as serializable JSON values for API exposure.
    pub fn ballots_as_json(&self) -> Vec<serde_json::Value> {
        self.ballots.iter().map(|b| {
            serde_json::json!({
                "voter": b.voter.to_string(),
                "rankings": b.original_rankings,
                "critic_scores": b.critic_scores,
            })
        }).collect()
    }

    /// Aggregate critic scores for a plan across all ballots that scored it.
    fn aggregate_critic_scores(&self, plan_id: &str) -> Option<CriticScore> {
        let mut total_feasibility = 0.0;
        let mut total_parallelism = 0.0;
        let mut total_completeness = 0.0;
        let mut total_risk = 0.0;
        let mut count = 0.0;

        for ballot in &self.ballots {
            if let Some(score) = ballot.critic_scores.get(plan_id) {
                total_feasibility += score.feasibility;
                total_parallelism += score.parallelism;
                total_completeness += score.completeness;
                total_risk += score.risk;
                count += 1.0;
            }
        }

        if count < f64::EPSILON {
            return None;
        }

        Some(CriticScore {
            feasibility: total_feasibility / count,
            parallelism: total_parallelism / count,
            completeness: total_completeness / count,
            risk: total_risk / count,
        })
    }

    /// Get the number of ballots received.
    pub fn ballot_count(&self) -> usize {
        self.ballots.len()
    }

    /// Get the number of registered proposals.
    pub fn proposal_count(&self) -> usize {
        self.proposal_ids.len()
    }

    /// Check if voting has been finalized.
    pub fn is_finalized(&self) -> bool {
        self.finalized
    }

    /// Debug view of voter IDs that already cast ballots.
    pub fn voter_ids_for_debug(&self) -> Vec<String> {
        self.ballots
            .iter()
            .map(|b| b.voter.to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap as StdHashMap;

    fn make_vote(
        voter: &str,
        task_id: &str,
        epoch: u64,
        rankings: Vec<&str>,
    ) -> RankedVote {
        RankedVote {
            voter: AgentId::new(voter.to_string()),
            task_id: task_id.to_string(),
            epoch,
            rankings: rankings.into_iter().map(String::from).collect(),
            critic_scores: StdHashMap::new(),
        }
    }

    #[test]
    fn test_irv_clear_majority() {
        let mut engine = VotingEngine::new(
            VotingConfig {
                prohibit_self_vote: false,
                ..Default::default()
            },
            "task1".into(),
            1,
        );

        let mut proposals = HashMap::new();
        proposals.insert("planA".to_string(), AgentId::new("alice".into()));
        proposals.insert("planB".to_string(), AgentId::new("bob".into()));
        engine.set_proposals(proposals);

        // 3 votes for A, 1 for B → A wins in round 1.
        engine.record_vote(make_vote("v1", "task1", 1, vec!["planA", "planB"])).unwrap();
        engine.record_vote(make_vote("v2", "task1", 1, vec!["planA", "planB"])).unwrap();
        engine.record_vote(make_vote("v3", "task1", 1, vec!["planA", "planB"])).unwrap();
        engine.record_vote(make_vote("v4", "task1", 1, vec!["planB", "planA"])).unwrap();

        let result = engine.run_irv().unwrap();
        assert_eq!(result.winner, "planA");
        assert_eq!(result.rounds, 1);
    }

    #[test]
    fn test_irv_with_elimination() {
        let mut engine = VotingEngine::new(
            VotingConfig {
                prohibit_self_vote: false,
                ..Default::default()
            },
            "task1".into(),
            1,
        );

        let mut proposals = HashMap::new();
        proposals.insert("planA".to_string(), AgentId::new("alice".into()));
        proposals.insert("planB".to_string(), AgentId::new("bob".into()));
        proposals.insert("planC".to_string(), AgentId::new("carol".into()));
        engine.set_proposals(proposals);

        // A:2, B:2, C:1 → C eliminated → C's voter's 2nd choice is B → B:3, A:2 → B wins
        engine.record_vote(make_vote("v1", "task1", 1, vec!["planA", "planB", "planC"])).unwrap();
        engine.record_vote(make_vote("v2", "task1", 1, vec!["planA", "planC", "planB"])).unwrap();
        engine.record_vote(make_vote("v3", "task1", 1, vec!["planB", "planA", "planC"])).unwrap();
        engine.record_vote(make_vote("v4", "task1", 1, vec!["planB", "planC", "planA"])).unwrap();
        engine.record_vote(make_vote("v5", "task1", 1, vec!["planC", "planB", "planA"])).unwrap();

        let result = engine.run_irv().unwrap();
        assert_eq!(result.winner, "planB");
        assert_eq!(result.elimination_order, vec!["planC".to_string()]);
    }

    #[test]
    fn test_self_vote_prohibition() {
        let mut engine = VotingEngine::new(VotingConfig::default(), "task1".into(), 1);

        let mut proposals = HashMap::new();
        proposals.insert("planA".to_string(), AgentId::new("alice".into()));
        proposals.insert("planB".to_string(), AgentId::new("bob".into()));
        engine.set_proposals(proposals);

        // Alice tries to rank her own plan first.
        let result = engine.record_vote(make_vote("alice", "task1", 1, vec!["planA", "planB"]));
        assert!(matches!(result, Err(ConsensusError::SelfVoteProhibited(_))));

        // Alice can rank someone else's plan first.
        let result = engine.record_vote(make_vote("alice", "task1", 1, vec!["planB", "planA"]));
        assert!(result.is_ok());
    }

    #[test]
    fn test_irv_rounds_recorded_on_clear_majority() {
        let mut engine = VotingEngine::new(
            VotingConfig { prohibit_self_vote: false, ..Default::default() },
            "task-rounds".into(),
            1,
        );

        let mut proposals = HashMap::new();
        proposals.insert("planA".to_string(), AgentId::new("alice".into()));
        proposals.insert("planB".to_string(), AgentId::new("bob".into()));
        engine.set_proposals(proposals);

        engine.record_vote(make_vote("v1", "task-rounds", 1, vec!["planA", "planB"])).unwrap();
        engine.record_vote(make_vote("v2", "task-rounds", 1, vec!["planA", "planB"])).unwrap();
        engine.record_vote(make_vote("v3", "task-rounds", 1, vec!["planA", "planB"])).unwrap();
        engine.record_vote(make_vote("v4", "task-rounds", 1, vec!["planB", "planA"])).unwrap();

        let result = engine.run_irv().unwrap();
        assert_eq!(result.winner, "planA");

        // One round recorded (the final round with no elimination)
        let rounds = engine.irv_rounds();
        assert_eq!(rounds.len(), 1);
        assert_eq!(rounds[0].round_number, 1);
        assert!(rounds[0].eliminated.is_none());
        assert_eq!(rounds[0].tallies["planA"], 3);
        assert_eq!(rounds[0].tallies["planB"], 1);
    }

    #[test]
    fn test_irv_rounds_recorded_with_elimination() {
        let mut engine = VotingEngine::new(
            VotingConfig { prohibit_self_vote: false, ..Default::default() },
            "task-elim".into(),
            1,
        );

        let mut proposals = HashMap::new();
        proposals.insert("planA".to_string(), AgentId::new("alice".into()));
        proposals.insert("planB".to_string(), AgentId::new("bob".into()));
        proposals.insert("planC".to_string(), AgentId::new("carol".into()));
        engine.set_proposals(proposals);

        // A:2, B:2, C:1 → C eliminated → B:3, A:2 → B wins
        engine.record_vote(make_vote("v1", "task-elim", 1, vec!["planA", "planB", "planC"])).unwrap();
        engine.record_vote(make_vote("v2", "task-elim", 1, vec!["planA", "planC", "planB"])).unwrap();
        engine.record_vote(make_vote("v3", "task-elim", 1, vec!["planB", "planA", "planC"])).unwrap();
        engine.record_vote(make_vote("v4", "task-elim", 1, vec!["planB", "planC", "planA"])).unwrap();
        engine.record_vote(make_vote("v5", "task-elim", 1, vec!["planC", "planB", "planA"])).unwrap();

        let result = engine.run_irv().unwrap();
        assert_eq!(result.winner, "planB");

        // Two rounds: elimination round + final round
        let rounds = engine.irv_rounds();
        assert_eq!(rounds.len(), 2);

        // Round 1: C eliminated
        let round1 = &rounds[0];
        assert_eq!(round1.round_number, 1);
        assert_eq!(round1.eliminated, Some("planC".to_string()));
        assert!(round1.continuing_candidates.contains(&"planA".to_string()));
        assert!(round1.continuing_candidates.contains(&"planB".to_string()));
        assert_eq!(round1.continuing_candidates.len(), 2);

        // Round 2: final winner
        let round2 = &rounds[1];
        assert_eq!(round2.round_number, 2);
        assert!(round2.eliminated.is_none());
        assert_eq!(round2.tallies["planB"], 3);
    }

    #[test]
    fn test_ballot_original_rankings_preserved() {
        let mut engine = VotingEngine::new(
            VotingConfig { prohibit_self_vote: false, ..Default::default() },
            "task-ballot".into(),
            1,
        );

        let mut proposals = HashMap::new();
        proposals.insert("planA".to_string(), AgentId::new("alice".into()));
        proposals.insert("planB".to_string(), AgentId::new("bob".into()));
        proposals.insert("planC".to_string(), AgentId::new("carol".into()));
        engine.set_proposals(proposals);

        engine.record_vote(make_vote("v1", "task-ballot", 1, vec!["planC", "planA", "planB"])).unwrap();

        // Before IRV, original_rankings should be set
        assert_eq!(engine.ballots[0].original_rankings, vec!["planC", "planA", "planB"]);

        // Run IRV (C has 1 vote, A 0, B 0 → C wins as sole voter)
        // Actually with 1 voter, C gets all votes and wins
        let _ = engine.run_irv().unwrap();

        // After IRV, original_rankings should still be intact
        assert_eq!(engine.ballots[0].original_rankings, vec!["planC", "planA", "planB"]);
    }

    #[test]
    fn test_ballots_as_json() {
        let mut engine = VotingEngine::new(
            VotingConfig { prohibit_self_vote: false, ..Default::default() },
            "task-json".into(),
            1,
        );

        let mut proposals = HashMap::new();
        proposals.insert("planA".to_string(), AgentId::new("alice".into()));
        proposals.insert("planB".to_string(), AgentId::new("bob".into()));
        engine.set_proposals(proposals);

        let mut vote = make_vote("voter1", "task-json", 1, vec!["planA", "planB"]);
        vote.critic_scores.insert("planA".to_string(), wws_protocol::CriticScore {
            feasibility: 0.9,
            parallelism: 0.8,
            completeness: 0.85,
            risk: 0.1,
        });
        engine.record_vote(vote).unwrap();

        let json_ballots = engine.ballots_as_json();
        assert_eq!(json_ballots.len(), 1);
        assert_eq!(json_ballots[0]["voter"].as_str().unwrap(), "voter1");
        assert_eq!(json_ballots[0]["rankings"][0].as_str().unwrap(), "planA");
        // Critic scores present in output
        assert!(json_ballots[0]["critic_scores"].is_object());
    }

    #[test]
    fn test_aggregate_critic_scores_in_result() {
        let mut engine = VotingEngine::new(
            VotingConfig { prohibit_self_vote: false, ..Default::default() },
            "task-critic".into(),
            1,
        );

        let mut proposals = HashMap::new();
        proposals.insert("planA".to_string(), AgentId::new("alice".into()));
        proposals.insert("planB".to_string(), AgentId::new("bob".into()));
        engine.set_proposals(proposals);

        // Both voters prefer planA and provide critic scores
        let mut vote1 = make_vote("v1", "task-critic", 1, vec!["planA", "planB"]);
        vote1.critic_scores.insert("planA".to_string(), wws_protocol::CriticScore {
            feasibility: 0.8,
            parallelism: 0.7,
            completeness: 0.9,
            risk: 0.1,
        });

        let mut vote2 = make_vote("v2", "task-critic", 1, vec!["planA", "planB"]);
        vote2.critic_scores.insert("planA".to_string(), wws_protocol::CriticScore {
            feasibility: 0.6,
            parallelism: 0.9,
            completeness: 0.8,
            risk: 0.2,
        });

        engine.record_vote(vote1).unwrap();
        engine.record_vote(vote2).unwrap();

        let result = engine.run_irv().unwrap();
        assert_eq!(result.winner, "planA");

        // Check aggregated critic score
        let winner_score = result.winner_critic_score.unwrap();
        assert!((winner_score.feasibility - 0.7).abs() < 1e-10); // avg of 0.8 and 0.6
        assert!((winner_score.parallelism - 0.8).abs() < 1e-10); // avg of 0.7 and 0.9
    }

    #[test]
    fn test_irv_rounds_task_id_matches() {
        let mut engine = VotingEngine::new(
            VotingConfig { prohibit_self_vote: false, ..Default::default() },
            "specific-task-id".into(),
            1,
        );

        let mut proposals = HashMap::new();
        proposals.insert("planX".to_string(), AgentId::new("x".into()));
        engine.set_proposals(proposals);

        engine.record_vote(make_vote("v1", "specific-task-id", 1, vec!["planX"])).unwrap();
        engine.run_irv().unwrap();

        let rounds = engine.irv_rounds();
        assert!(!rounds.is_empty());
        for round in rounds {
            assert_eq!(round.task_id, "specific-task-id");
        }
    }
}
