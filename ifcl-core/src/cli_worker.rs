use crate::{Worker, WorkerRole, WorkerProfile, Task, Event, WorkerOutputPayload, EventBus};
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use std::process::Stdio;

pub struct CliWorker {
    pub profile: WorkerProfile,
}

impl CliWorker {
    pub fn new(name: &str, role: WorkerRole) -> Self {
        Self {
            profile: WorkerProfile {
                name: name.to_string(),
                role,
                model: None,
            },
        }
    }
}

#[async_trait]
impl Worker for CliWorker {
    fn id(&self) -> &str {
        &self.profile.name
    }

    fn role(&self) -> WorkerRole {
        self.profile.role
    }

    fn metadata(&self) -> &WorkerProfile {
        &self.profile
    }

    async fn execute(&self, bus: Arc<dyn EventBus>, task: &Task, workspace_path: &str) -> anyhow::Result<String> {
        tokio::fs::create_dir_all(workspace_path).await?;
        
        let (cmd, args) = match task.name.as_str() {
            "Initialize Project" | "Init Repo" => ("git", vec!["init"]),
            _ => {
                anyhow::bail!("CliWorker: Unknown task type '{}'. Strict execution mode enabled.", task.name);
            }
        };

        let mut child = Command::new(cmd)
            .args(args)
            .current_dir(workspace_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let bus_c = Arc::clone(&bus);
        let worker_id = self.id().to_string();
        
        let mut full_output = String::new();

        // Stream stdout
        let worker_id_stdout = worker_id.clone();
        let bus_stdout = Arc::clone(&bus_c);
        let stdout_handle = tokio::spawn(async move {
            let mut out = String::new();
            while let Ok(Some(line)) = stdout_reader.next_line().await {
                out.push_str(&line);
                out.push('\n');
                let _ = bus_stdout.publish(Event {
                    id: Uuid::new_v4(),
                    session_id: Uuid::nil(), // session_id is not passed yet, using nil
                    trace_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    worker_id: worker_id_stdout.clone(),
                    event_type: "WorkerOutput".to_string(),
                    payload: serde_json::to_string(&WorkerOutputPayload {
                        content: line,
                        is_stderr: false,
                    }).unwrap(),
                }).await;
            }
            out
        });

        // Stream stderr
        let worker_id_stderr = worker_id.clone();
        let bus_stderr = Arc::clone(&bus_c);
        let stderr_handle = tokio::spawn(async move {
            let mut out = String::new();
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                out.push_str(&line);
                out.push('\n');
                let _ = bus_stderr.publish(Event {
                    id: Uuid::new_v4(),
                    session_id: Uuid::nil(),
                    trace_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    worker_id: worker_id_stderr.clone(),
                    event_type: "WorkerOutput".to_string(),
                    payload: serde_json::to_string(&WorkerOutputPayload {
                        content: line,
                        is_stderr: true,
                    }).unwrap(),
                }).await;
            }
            out
        });

        let status = child.wait().await?;
        let stdout_res = stdout_handle.await?;
        let stderr_res = stderr_handle.await?;

        full_output.push_str(&stdout_res);
        full_output.push_str(&stderr_res);

        if status.success() {
            Ok(full_output)
        } else {
            anyhow::bail!("Command failed:\n{}", stderr_res)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::{TaskStatus, InMemoryEventBus};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_cli_worker_git_init() {
        let worker = CliWorker::new("GitBot", WorkerRole::Git);
        let workspace = tempdir().unwrap();
        let bus = Arc::new(InMemoryEventBus::new(10));
        let task = Task {
            id: Uuid::new_v4(),
            name: "Initialize Project".to_string(),
            description: "Setup git".to_string(),
            status: TaskStatus::Pending,
            assigned_worker: None,
        };

        let result = worker.execute(bus, &task, workspace.path().to_str().unwrap()).await.unwrap();
        assert!(result.contains("Initialized empty Git repository"));
        assert!(workspace.path().join(".git").exists());
    }


}
