use crate::{CliExecutor, Mission, Task, TaskStatus};
use async_trait::async_trait;
use serde::Deserialize;
use uuid::Uuid;

/// Trait for a planner that generates missions based on a goal.
#[async_trait]
pub trait Planner: Send + Sync {
    async fn generate_initial_missions(&self, goal: &str) -> Vec<Mission>;
    async fn replan_on_failure(
        &self,
        goal: &str,
        mission: &Mission,
        failed_task_id: Uuid,
    ) -> Vec<Mission>;
}

/// A basic, rule-based planner that generates a few standard missions for any goal.
pub struct BasicPlanner;

#[async_trait]
impl Planner for BasicPlanner {
    async fn generate_initial_missions(&self, _goal: &str) -> Vec<Mission> {
        vec![Mission {
            id: Uuid::new_v4(),
            session_id: Uuid::nil(),
            name: "Phase 1: Setup".to_string(),
            tasks: vec![Task {
                id: Uuid::new_v4(),
                name: "Init Repo".to_string(),
                description: "Setup git and base file structure.".to_string(),
                status: TaskStatus::Pending,
                assigned_worker: Some("Git-Bot".to_string()),
                retry_count: 0,
            }],
            workspace_path: None,
        }]
    }

    async fn replan_on_failure(
        &self,
        _goal: &str,
        mission: &Mission,
        _failed_task_id: Uuid,
    ) -> Vec<Mission> {
        // Basic replanner: just retry the same mission with a "Retry" prefix
        let mut retried = mission.clone();
        retried.id = Uuid::new_v4();
        retried.name = format!("RETRY: {}", mission.name);
        for task in &mut retried.tasks {
            if task.status == TaskStatus::Failure {
                task.status = TaskStatus::Pending;
            }
        }
        vec![retried]
    }
}

/// An LLM-powered planner that uses an external CLI for decomposition.
pub struct LLMPlanner {
    pub executor: CliExecutor,
}

#[derive(Deserialize)]
struct LlmMission {
    name: String,
    tasks: Vec<LlmTask>,
}

#[derive(Deserialize)]
struct LlmTask {
    name: String,
    description: String,
    assigned_worker: Option<String>,
}

impl LLMPlanner {
    fn convert_to_domain(&self, llm_missions: Vec<LlmMission>, _goal: &str) -> Vec<Mission> {
        llm_missions
            .into_iter()
            .map(|lm| Mission {
                id: Uuid::new_v4(),
                session_id: Uuid::nil(),
                name: lm.name,
                tasks: lm
                    .tasks
                    .into_iter()
                    .map(|lt| Task {
                        id: Uuid::new_v4(),
                        name: lt.name,
                        description: lt.description,
                        status: TaskStatus::Pending,
                        assigned_worker: lt.assigned_worker,
                        retry_count: 0,
                    })
                    .collect(),
                workspace_path: None, // Will be set by main
            })
            .collect()
    }
}

#[async_trait]
impl Planner for LLMPlanner {
    async fn generate_initial_missions(&self, goal: &str) -> Vec<Mission> {
        let prompt = format!("Decompose the goal '{}' into a JSON list of missions with tasks. \
        IMPORTANT: You must use ONLY the following values for 'task.name': \
        - 'Create File' (description: file path) \
        - 'Create Directory' (description: path) \
        - 'Run Command' (description: shell command, e.g., 'cargo add serde') \
        - 'Git Init' \
        - 'Git Add' \
        - 'Git Commit' \
        \
        For 'Create File', use 'task.name': 'Run Command' and in the description write a shell command to create the file (e.g., `echo \"content\" > file` or `cat <<EOF > file ... EOF`). SET 'assigned_worker' to 'Git-Bot'. \
        For git or shell commands, SET 'assigned_worker' to 'Git-Bot'. \
        Output must be valid JSON matching the Mission struct fields name, tasks. Each task must have name, description, assigned_worker.", goal);

        match self.executor.execute(&prompt).await {
            Ok(result) => {
                println!("DEBUG LLM PLANNER RAW STDOUT: {}", result.stdout);
                println!("DEBUG LLM PLANNER RAW STDERR: {}", result.stderr);
                // Try to clean output: find first [ and last ]
                let clean_json = if let Some(start) = result.stdout.find('[') {
                    if let Some(end) = result.stdout.rfind(']') {
                        if start <= end {
                            &result.stdout[start..=end]
                        } else {
                            &result.stdout
                        }
                    } else {
                        &result.stdout
                    }
                } else {
                    &result.stdout
                };

                if let Ok(llm_missions) = serde_json::from_str::<Vec<LlmMission>>(clean_json) {
                     self.convert_to_domain(llm_missions, goal)
                } else {
                    BasicPlanner.generate_initial_missions(goal).await
                }
            }
            Err(_) => BasicPlanner.generate_initial_missions(goal).await,
        }
    }

    async fn replan_on_failure(
        &self,
        goal: &str,
        mission: &Mission,
        failed_task_id: Uuid,
    ) -> Vec<Mission> {
        let prompt = format!(
            "Task ID {} failed in mission '{}' for goal '{}'. Generate a recovery plan as JSON missions.",
            failed_task_id, mission.name, goal
        );
        match self.executor.execute(&prompt).await {
            Ok(result) => {
                if let Ok(missions) = serde_json::from_str::<Vec<Mission>>(&result.stdout) {
                    missions
                } else {
                    BasicPlanner
                        .replan_on_failure(goal, mission, failed_task_id)
                        .await
                }
            }
            Err(_) => {
                BasicPlanner
                    .replan_on_failure(goal, mission, failed_task_id)
                    .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_planner_generation() {
        let planner = BasicPlanner;
        let missions = planner.generate_initial_missions("Build a Rust CLI").await;

        assert!(!missions.is_empty());
        assert!(missions[0].name.contains("Setup"));
        assert_eq!(missions[0].tasks.len(), 1);
    }

    #[tokio::test]
    async fn test_basic_planner_replanning() {
        let planner = BasicPlanner;
        let mission = Mission {
            id: Uuid::new_v4(),
            session_id: Uuid::nil(),
            name: "Test Mission".to_string(),
            tasks: vec![Task {
                id: Uuid::new_v4(),
                name: "Fail Task".to_string(),
                description: "Test".to_string(),
                status: TaskStatus::Failure,
                assigned_worker: None,
                retry_count: 0,
            }],
            workspace_path: None,
        };
        let failed_id = mission.tasks[0].id;
        let replanned = planner.replan_on_failure("goal", &mission, failed_id).await;

        assert_eq!(replanned.len(), 1);
        assert!(replanned[0].name.contains("RETRY"));
        assert_eq!(replanned[0].tasks[0].status, TaskStatus::Pending);
    }

    #[tokio::test]
    async fn test_llm_planner_fallback() {
        // Use 'false' to simulate an error in the CLI
        let planner = LLMPlanner {
            executor: CliExecutor::new("false".to_string(), vec![]),
        };
        let missions = planner.generate_initial_missions("Goal").await;
        // Should fallback to BasicPlanner output
        assert!(!missions.is_empty());
    }
}
