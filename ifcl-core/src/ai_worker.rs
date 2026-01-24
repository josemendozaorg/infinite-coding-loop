use uuid::Uuid;
use std::sync::Arc;
use tokio::fs;
use tokio::process::Command;
use tokio::io::{BufReader, AsyncBufReadExt};
use std::process::Stdio;
use async_trait::async_trait;
use crate::{Worker, WorkerRole, Task, EventBus, Event, WorkerOutputPayload, WorkerProfile};
use chrono::Utc;
use serde_json;

#[derive(Debug, Clone)]
pub struct AiGenericWorker {
    pub profile: WorkerProfile,
    pub binary: String,
    pub model_flag: Option<String>,
}

impl AiGenericWorker {
    pub fn new(name: String, role: WorkerRole, binary: String, model_flag: Option<String>) -> Self {
        Self {
            profile: WorkerProfile {
                name,
                role,
                model: model_flag.clone(),
            },
            binary,
            model_flag,
        }
    }
}

#[async_trait]
impl Worker for AiGenericWorker {
    fn id(&self) -> &str {
        &self.profile.name
    }
    
    fn role(&self) -> WorkerRole {
        self.profile.role.clone()
    }

    fn metadata(&self) -> &WorkerProfile {
        &self.profile
    }

    async fn execute(&self, bus: Arc<dyn EventBus>, task: &Task, workspace_path: &str, session_id: Uuid) -> anyhow::Result<String> {
        let prompt = format!("Task: {}\nDescription: {}\n\nPerform this task and return the result.", task.name, task.description);
        
        let mut args = vec![prompt];
        if let Some(model) = &self.model_flag {
            args.insert(0, model.clone());
            args.insert(0, "--model".to_string());
        }

        let mut child = Command::new(&self.binary)
            .args(&args)
            .current_dir(workspace_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let worker_id = self.profile.name.clone();
        let bus_out = bus.clone();
        let bus_err = bus.clone();

        // Stream Output
        let h_out = tokio::spawn(async move {
             let mut full = String::new();
             while let Ok(Some(line)) = stdout_reader.next_line().await {
                 full.push_str(&line);
                 full.push('\n');
                 let _ = bus_out.publish(Event {
                     id: Uuid::new_v4(),
                     session_id,
                     trace_id: Uuid::new_v4(),
                     timestamp: Utc::now(),
                     worker_id: worker_id.clone(),
                     event_type: "WorkerOutput".to_string(),
                     payload: serde_json::to_string(&WorkerOutputPayload {
                         content: line,
                         is_stderr: false, 
                     }).unwrap_or_default(),
                 }).await;
             }
             full
        });

        let worker_id_2 = self.profile.name.clone();
        let h_err = tokio::spawn(async move {
             let mut full = String::new();
             while let Ok(Some(line)) = stderr_reader.next_line().await {
                 full.push_str(&line);
                 full.push('\n');
                 let _ = bus_err.publish(Event {
                    id: Uuid::new_v4(),
                     session_id,
                     trace_id: Uuid::new_v4(),
                     timestamp: Utc::now(),
                     worker_id: worker_id_2.clone(),
                     event_type: "WorkerOutput".to_string(),
                     payload: serde_json::to_string(&WorkerOutputPayload {
                         content: line,
                         is_stderr: true, 
                     }).unwrap_or_default(),
                 }).await;
             }
             full
        });

        let status = child.wait().await?;
        let out_str = h_out.await?;
        let err_str = h_err.await?;

        if status.success() {
            Ok(out_str)
        } else {
            anyhow::bail!("AI Worker Failed: {}", err_str)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InMemoryEventBus;
    use crate::TaskStatus;

    #[tokio::test]
    async fn test_ai_generic_worker_echo() {
        // Use 'echo' as a mock AI binary
        let worker = AiGenericWorker::new(
            "Echo-Bot".to_string(),
            WorkerRole::Coder,
            "echo".to_string(),
            None,
        );

        let bus = Arc::new(InMemoryEventBus::new(10));
        let task = Task {
            id: Uuid::new_v4(),
            name: "Test Task".to_string(),
            description: "Say Hello".to_string(),
            status: TaskStatus::Pending,
            assigned_worker: None,
        };
        let workspace = std::env::temp_dir();
        let session_id = Uuid::new_v4();

        let result = worker.execute(bus, &task, workspace.to_str().unwrap(), session_id).await;
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert!(output.contains("Task: Test Task"));
        assert!(output.contains("Description: Say Hello"));
    }

    #[tokio::test]
    async fn test_ai_generic_worker_with_model() {
        // Use 'echo' again. If model flag is passed, echo receives it as first arg.
        let worker = AiGenericWorker::new(
            "Echo-Bot-Pro".to_string(),
            WorkerRole::Coder,
            "echo".to_string(),
            Some("gpt-4".to_string()),
        );

        let bus = Arc::new(InMemoryEventBus::new(10));
        let task = Task {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            status: TaskStatus::Pending,
            assigned_worker: None,
        };
        let workspace = std::env::temp_dir();

        let result = worker.execute(bus, &task, workspace.to_str().unwrap(), Uuid::new_v4()).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        
        // Output should contain "--model gpt-4" because echo prints all args
        assert!(output.contains("--model"));
        assert!(output.contains("gpt-4"));
    }
}
