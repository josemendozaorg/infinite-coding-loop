use crate::{EventBus, Mission, Planner, Task, Worker, WorkerProfile, WorkerRole};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct ReplanContext {
    pub goal: String,
    pub mission: Mission,
    pub failed_task_id: Uuid,
}

pub struct PlannerWorker {
    pub profile: WorkerProfile,
    pub planner: Arc<dyn Planner>,
}

impl PlannerWorker {
    pub fn new(planner: Arc<dyn Planner>) -> Self {
        Self {
            profile: WorkerProfile {
                name: "Planner".to_string(),
                role: WorkerRole::Planner,
                model: Some("System-Internal".to_string()),
            },
            planner,
        }
    }
}

#[async_trait]
impl Worker for PlannerWorker {
    fn id(&self) -> &str {
        &self.profile.name
    }

    fn role(&self) -> WorkerRole {
        self.profile.role
    }

    fn metadata(&self) -> &WorkerProfile {
        &self.profile
    }

    async fn execute(
        &self,
        _bus: Arc<dyn EventBus>,
        task: &Task,
        workspace_path: &str,
        _session_id: Uuid,
    ) -> anyhow::Result<String> {
        // Enrich context
        let enricher = crate::ContextEnricher::new();
        let context = enricher.collect(workspace_path);

        if task.name.starts_with("Replan") {
            // Expect description to be JSON of ReplanContext
            let context_data: ReplanContext = serde_json::from_str(&task.description)?;

            // Note: We can't easily inject context into 'replan_on_failure' without changing the trait signature
            // or the inputs.
            // However, ReplanContext has 'mission' and 'failed_task_id'.
            // The Planner trait 'replan_on_failure' takes (goal, mission, failed_task_id).
            // We might need to update the Planner trait to accept context, OR we prepend context to the Goal?
            // "Goal: <Goal> \n Context: <Context>"
            // This is a hack but works for LLM planners.

            let enriched_goal = format!("{}\n\n{}", context_data.goal, context);

            let new_missions = self
                .planner
                .replan_on_failure(
                    &enriched_goal,
                    &context_data.mission,
                    context_data.failed_task_id,
                )
                .await;

            // Return missions as JSON
            Ok(serde_json::to_string(&new_missions)?)
        } else {
            // Default capability: Generate initial missions
            let enriched_goal = format!("{}\n\n{}", task.description, context);
            let missions = self.planner.generate_initial_missions(&enriched_goal).await;
            Ok(serde_json::to_string(&missions)?)
        }
    }
}
