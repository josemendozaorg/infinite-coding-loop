use uuid::Uuid;
use crate::{Mission, Task, TaskStatus};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct WorkerRequest {
    pub requester_id: String,
    pub target_role: String,
    pub context: String,
}

#[async_trait::async_trait]
pub trait Orchestrator: Send + Sync {
    async fn create_mission(&self, name: &str, tasks: Vec<(String, String)>, workspace_path: Option<String>) -> anyhow::Result<Mission>;
    async fn update_task_status(&self, mission_id: Uuid, task_id: Uuid, status: TaskStatus) -> anyhow::Result<()>;
    async fn execute_task(&self, bus: std::sync::Arc<dyn crate::EventBus>, mission_id: Uuid, task_id: Uuid, worker: &dyn crate::Worker) -> anyhow::Result<String>;
    async fn get_missions(&self) -> anyhow::Result<Vec<Mission>>;
    async fn handle_worker_request(&self, request: WorkerRequest) -> anyhow::Result<()>;
}

pub struct BasicOrchestrator {
    pub missions: tokio::sync::RwLock<Vec<Mission>>,
}

impl BasicOrchestrator {
    pub fn new() -> Self {
        Self {
            missions: tokio::sync::RwLock::new(Vec::new()),
        }
    }
}

impl Default for BasicOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Orchestrator for BasicOrchestrator {
    async fn create_mission(&self, name: &str, tasks: Vec<(String, String)>, workspace_path: Option<String>) -> anyhow::Result<Mission> {
        let task_list = tasks.into_iter().map(|(n, d)| Task {
            id: Uuid::new_v4(),
            name: n,
            description: d,
            status: TaskStatus::Pending,
            assigned_worker: None,
        }).collect();

        let mission = Mission {
            id: Uuid::new_v4(),
            name: name.to_string(),
            tasks: task_list,
            workspace_path,
        };

        let mut missions = self.missions.write().await;
        missions.push(mission.clone());
        Ok(mission)
    }

    async fn update_task_status(&self, mission_id: Uuid, task_id: Uuid, status: TaskStatus) -> anyhow::Result<()> {
        let mut missions = self.missions.write().await;
        if let Some(mission) = missions.iter_mut().find(|m| m.id == mission_id) {
            if let Some(task) = mission.tasks.iter_mut().find(|t| t.id == task_id) {
                task.status = status;
                return Ok(());
            }
        }
        anyhow::bail!("Mission or Task not found")
    }

    async fn get_missions(&self) -> anyhow::Result<Vec<Mission>> {
        let missions = self.missions.read().await;
        Ok(missions.clone())
    }

    async fn execute_task(&self, bus: std::sync::Arc<dyn crate::EventBus>, mission_id: Uuid, task_id: Uuid, worker: &dyn crate::Worker) -> anyhow::Result<String> {
        let (task, workspace, session_id) = {
            let missions = self.missions.read().await;
            let mission = missions.iter().find(|m| m.id == mission_id).ok_or_else(|| anyhow::anyhow!("Mission not found"))?;
            let task = mission.tasks.iter().find(|t| t.id == task_id).ok_or_else(|| anyhow::anyhow!("Task not found"))?;
            // We need session_id here to publish events. Assume for now or add to mission. 
            // In a real system, the Orchestrator might know the current session.
            (task.clone(), mission.workspace_path.clone(), Uuid::nil()) // Placeholder session_id
        };

        let workspace_path = workspace.unwrap_or_else(|| ".".to_string());
        
        self.update_task_status(mission_id, task_id, TaskStatus::Running).await?;
        
        match worker.execute(bus, &task, &workspace_path).await {
            Ok(output) => {
                self.update_task_status(mission_id, task_id, TaskStatus::Success).await?;
                Ok(output)
            }
            Err(e) => {
                self.update_task_status(mission_id, task_id, TaskStatus::Failure).await?;
                Err(e)
            }
        }
    }

    async fn handle_worker_request(&self, _request: WorkerRequest) -> anyhow::Result<()> {
        // Placeholder for future logic
        Ok(())
    }
}
