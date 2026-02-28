//! Mock plan generator for testing
//!
//! This provides a simple plan generator that splits tasks into N equal subtasks
//! based on the number of available subordinate agents.

use std::future::Future;
use std::pin::Pin;
use openswarm_protocol::{AgentId, Plan, PlanSubtask};
use crate::rfp::{PlanGenerator, PlanContext};
use crate::ConsensusError;

/// Mock plan generator that creates simple decompositions
pub struct MockPlanGenerator {
    pub agent_id: AgentId,
}

impl MockPlanGenerator {
    pub fn new(agent_id: AgentId) -> Self {
        Self { agent_id }
    }
}

impl PlanGenerator for MockPlanGenerator {
    fn generate_plan<'a>(
        &'a self,
        context: &'a PlanContext,
    ) -> Pin<Box<dyn Future<Output = Result<Plan, ConsensusError>> + Send + 'a>> {
        Box::pin(async move {
            // Determine number of subtasks based on available agents
            let subtask_count = if context.available_agents > 0 {
                context.available_agents.min(context.branching_factor as u64) as u32
            } else {
                // Default to branching factor if no agent info available
                context.branching_factor
            };

            let mut plan = Plan::new(
                context.task.task_id.clone(),
                self.agent_id.clone(),
                context.epoch,
            );

            // Create subtasks by splitting the parent task
            for i in 0..subtask_count {
                plan.subtasks.push(PlanSubtask {
                    index: i + 1,
                    description: format!(
                        "Part {}/{}: {}",
                        i + 1,
                        subtask_count,
                        context.task.description
                    ),
                    required_capabilities: vec![],
                    estimated_complexity: 1.0 / subtask_count as f64,
                });
            }

            plan.rationale = format!(
                "Decomposed into {} parallel subtasks for distributed execution. \
                 Each subtask represents an equal portion of the work.",
                subtask_count
            );
            plan.estimated_parallelism = subtask_count as f64;

            tracing::info!(
                task_id = %context.task.task_id,
                subtasks = subtask_count,
                "Generated mock plan"
            );

            Ok(plan)
        })
    }
}
