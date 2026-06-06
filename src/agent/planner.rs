use crate::models::plan::{Plan, PlanStep, StepStatus};
use std::collections::HashMap;

/// Plan generator and DAG dependency manager.
pub struct Planner {
    #[allow(dead_code)]
    max_iterations: u32,
}

impl Planner {
    pub fn new(max_iterations: u32) -> Self {
        Self { max_iterations }
    }

    /// Create a new plan for a given task.
    pub fn create_plan(&self, goal: &str, steps: Vec<PlanStep>) -> Plan {
        Plan {
            goal: goal.to_string(),
            steps,
            variables: HashMap::new(),
        }
    }

    /// Get the next step that is ready to execute (all deps satisfied).
    pub fn next_ready_step(&self, plan: &Plan) -> Option<usize> {
        plan.steps.iter().position(|step| {
            if step.status != StepStatus::Pending {
                return false;
            }
            step.depends_on.iter().all(|dep_id| {
                plan.steps.iter().any(|s| s.id == *dep_id && s.status == StepStatus::Completed)
            })
        })
    }

    /// Mark a step as completed with result.
    pub fn complete_step(plan: &mut Plan, index: usize, result: String) {
        if let Some(step) = plan.steps.get_mut(index) {
            step.status = StepStatus::Completed;
            step.result = Some(result);
        }
    }

    /// Mark a step as failed with error.
    pub fn fail_step(plan: &mut Plan, index: usize, error: String) {
        if let Some(step) = plan.steps.get_mut(index) {
            step.status = StepStatus::Failed;
            step.error = Some(error);
        }
    }

    /// Get steps that need to be retried.
    pub fn retryable_steps(&self, plan: &Plan) -> Vec<usize> {
        plan.steps.iter().enumerate()
            .filter(|(_, step)| step.status == StepStatus::Failed && step.retry_count < 3)
            .map(|(i, _)| i)
            .collect()
    }

    /// Increment retry count for a step.
    pub fn increment_retry(plan: &mut Plan, index: usize) {
        if let Some(step) = plan.steps.get_mut(index) {
            step.retry_count += 1;
            step.status = StepStatus::Pending;
            step.error = None;
        }
    }
}
