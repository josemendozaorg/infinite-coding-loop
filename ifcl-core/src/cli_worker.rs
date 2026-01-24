use crate::{Event, EventBus, Task, Worker, WorkerOutputPayload, WorkerProfile, WorkerRole};
use async_trait::async_trait;
use chrono::Utc;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use uuid::Uuid;

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

    async fn execute(
        &self,
        bus: Arc<dyn EventBus>,
        task: &Task,
        workspace_path: &str,
        session_id: Uuid,
    ) -> anyhow::Result<String> {
        tokio::fs::create_dir_all(workspace_path).await?;

        let (cmd, args): (&str, Vec<String>) = match task.name.as_str() {
            // Git operations
            "Initialize Project" | "Init Repo" | "Git Init" => ("git", vec!["init".to_string()]),
            "Git Add" => ("git", vec!["add".to_string(), ".".to_string()]),
            "Git Status" => ("git", vec!["status".to_string()]),
            "Git Commit" => (
                "git",
                vec![
                    "commit".to_string(),
                    "-m".to_string(),
                    task.description.clone(),
                ],
            ),

            // File operations
            "Create Directory" => ("mkdir", vec!["-p".to_string(), task.description.clone()]),
            "Create File" => ("touch", vec![task.description.clone()]),

            // Build tools
            "Cargo Build" | "Build Project" => ("cargo", vec!["build".to_string()]),
            "Cargo Test" | "Run Tests" => ("cargo", vec!["test".to_string()]),
            "Cargo Check" => ("cargo", vec!["check".to_string()]),
            "NPM Install" => ("npm", vec!["install".to_string()]),
            "NPM Build" => ("npm", vec!["run".to_string(), "build".to_string()]),

            // Generic shell command (description contains the command)
            "Run Command" | "Shell" => {
                 // Use sh -c to allow chaining and shell builtins
                 ("sh", vec!["-c".to_string(), task.description.clone()])
            }

            _ => {
                anyhow::bail!(
                    "CliWorker: Unknown task type '{}'. Strict execution mode enabled.",
                    task.name
                );
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
                let _ = bus_stdout
                    .publish(Event {
                        id: Uuid::new_v4(),
                        session_id,
                        trace_id: Uuid::new_v4(),
                        timestamp: Utc::now(),
                        worker_id: worker_id_stdout.clone(),
                        event_type: "WorkerOutput".to_string(),
                        payload: serde_json::to_string(&WorkerOutputPayload {
                            content: line,
                            is_stderr: false,
                        })
                        .unwrap(),
                    })
                    .await;
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
                let _ = bus_stderr
                    .publish(Event {
                        id: Uuid::new_v4(),
                        session_id,
                        trace_id: Uuid::new_v4(),
                        timestamp: Utc::now(),
                        worker_id: worker_id_stderr.clone(),
                        event_type: "WorkerOutput".to_string(),
                        payload: serde_json::to_string(&WorkerOutputPayload {
                            content: line,
                            is_stderr: true,
                        })
                        .unwrap(),
                    })
                    .await;
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

impl CliWorker {
    /// Helper method to execute a command with streaming output
    async fn execute_command(
        &self,
        cmd: &str,
        args: Vec<String>,
        workspace_path: &str,
        bus: Arc<dyn EventBus>,
        session_id: Uuid,
    ) -> anyhow::Result<String> {
        let mut child = Command::new(cmd)
            .args(&args)
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

        // Stream stdout
        let worker_id_stdout = worker_id.clone();
        let bus_stdout = Arc::clone(&bus_c);
        let stdout_handle = tokio::spawn(async move {
            let mut out = String::new();
            while let Ok(Some(line)) = stdout_reader.next_line().await {
                out.push_str(&line);
                out.push('\n');
                let _ = bus_stdout
                    .publish(Event {
                        id: Uuid::new_v4(),
                        session_id,
                        trace_id: Uuid::new_v4(),
                        timestamp: Utc::now(),
                        worker_id: worker_id_stdout.clone(),
                        event_type: "WorkerOutput".to_string(),
                        payload: serde_json::to_string(&WorkerOutputPayload {
                            content: line,
                            is_stderr: false,
                        })
                        .unwrap(),
                    })
                    .await;
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
                let _ = bus_stderr
                    .publish(Event {
                        id: Uuid::new_v4(),
                        session_id,
                        trace_id: Uuid::new_v4(),
                        timestamp: Utc::now(),
                        worker_id: worker_id_stderr.clone(),
                        event_type: "WorkerOutput".to_string(),
                        payload: serde_json::to_string(&WorkerOutputPayload {
                            content: line,
                            is_stderr: true,
                        })
                        .unwrap(),
                    })
                    .await;
            }
            out
        });

        let status = child.wait().await?;
        let stdout_res = stdout_handle.await?;
        let stderr_res = stderr_handle.await?;

        let mut full_output = String::new();
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
    use crate::{InMemoryEventBus, TaskStatus};
    use tempfile::tempdir;
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
            retry_count: 0,
        };

        let result = worker
            .execute(
                bus,
                &task,
                workspace.path().to_str().unwrap(),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        assert!(result.contains("Initialized empty Git repository"));
        assert!(workspace.path().join(".git").exists());
    }

    #[tokio::test]
    async fn test_cli_worker_cargo_build() {
        let worker = CliWorker::new("BuildBot", WorkerRole::Coder);
        let workspace = tempdir().unwrap();

        // Create a minimal Cargo.toml
        std::fs::write(
            workspace.path().join("Cargo.toml"),
            r#"[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#,
        )
        .unwrap();
        std::fs::create_dir(workspace.path().join("src")).unwrap();
        std::fs::write(workspace.path().join("src/main.rs"), "fn main() {}").unwrap();

        let bus = Arc::new(InMemoryEventBus::new(10));
        let task = Task {
            id: Uuid::new_v4(),
            name: "Cargo Build".to_string(),
            description: "Build the project".to_string(),
            status: TaskStatus::Pending,
            assigned_worker: None,
            retry_count: 0,
        };

        let result = worker
            .execute(
                bus,
                &task,
                workspace.path().to_str().unwrap(),
                Uuid::new_v4(),
            )
            .await;
        assert!(result.is_ok(), "Cargo build should succeed: {:?}", result);
    }

    #[tokio::test]
    async fn test_cli_worker_create_directory() {
        let worker = CliWorker::new("FileBot", WorkerRole::Coder);
        let workspace = tempdir().unwrap();
        let bus = Arc::new(InMemoryEventBus::new(10));

        let task = Task {
            id: Uuid::new_v4(),
            name: "Create Directory".to_string(),
            description: "src/components".to_string(),
            status: TaskStatus::Pending,
            assigned_worker: None,
            retry_count: 0,
        };

        let result = worker
            .execute(
                bus,
                &task,
                workspace.path().to_str().unwrap(),
                Uuid::new_v4(),
            )
            .await;
        assert!(
            result.is_ok(),
            "Create directory should succeed: {:?}",
            result
        );
        assert!(
            workspace.path().join("src/components").exists(),
            "Directory should be created"
        );
    }

    #[tokio::test]
    async fn test_cli_worker_git_add() {
        let worker = CliWorker::new("GitBot", WorkerRole::Git);
        let workspace = tempdir().unwrap();
        let bus = Arc::new(InMemoryEventBus::new(10));

        // Initialize git first
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(workspace.path())
            .output()
            .unwrap();

        // Create a file to add
        std::fs::write(workspace.path().join("test.txt"), "hello").unwrap();

        let task = Task {
            id: Uuid::new_v4(),
            name: "Git Add".to_string(),
            description: "Stage all files".to_string(),
            status: TaskStatus::Pending,
            assigned_worker: None,
            retry_count: 0,
        };

        let result = worker
            .execute(
                bus,
                &task,
                workspace.path().to_str().unwrap(),
                Uuid::new_v4(),
            )
            .await;
        assert!(result.is_ok(), "Git add should succeed: {:?}", result);
    }

    #[tokio::test]
    async fn test_cli_worker_shell_command() {
        let worker = CliWorker::new("ShellBot", WorkerRole::Ops);
        let workspace = tempdir().unwrap();
        let bus = Arc::new(InMemoryEventBus::new(10));

        let task = Task {
            id: Uuid::new_v4(),
            name: "Run Command".to_string(),
            description: "echo hello_world".to_string(),
            status: TaskStatus::Pending,
            assigned_worker: None,
            retry_count: 0,
        };

        let result = worker
            .execute(
                bus,
                &task,
                workspace.path().to_str().unwrap(),
                Uuid::new_v4(),
            )
            .await;
        assert!(result.is_ok(), "Shell command should succeed: {:?}", result);
        assert!(
            result.unwrap().contains("hello_world"),
            "Output should contain echo result"
        );
    }

    #[tokio::test]
    async fn test_cli_worker_unknown_task_fails() {
        let worker = CliWorker::new("Bot", WorkerRole::Coder);
        let workspace = tempdir().unwrap();
        let bus = Arc::new(InMemoryEventBus::new(10));

        let task = Task {
            id: Uuid::new_v4(),
            name: "Unknown Task Type".to_string(),
            description: "This should fail".to_string(),
            status: TaskStatus::Pending,
            assigned_worker: None,
            retry_count: 0,
        };

        let result = worker
            .execute(
                bus,
                &task,
                workspace.path().to_str().unwrap(),
                Uuid::new_v4(),
            )
            .await;
        assert!(result.is_err(), "Unknown task should fail with strict mode");
    }
}
