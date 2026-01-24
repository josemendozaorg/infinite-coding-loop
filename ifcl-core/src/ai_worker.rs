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
use regex::Regex;
use std::path::Path;

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
        // Updated prompt with "Code Concierge" instructions
        let prompt = format!(
            "Task: {}\nDescription: {}\n\nIMPORTANT: If you generate code, you MUST use markdown blocks with the file path as the language suffix, e.g.\n```rust:src/main.rs\nfn main() ...\n```\nThis will be automatically saved to disk.\n\nPerform this task and return the result.", 
            task.name, task.description
        );
        
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
            // Code Concierge: Parse and Write Files
            // Regex to find ```lang:path\ncontent```
            // Matches: ```rust:src/main.rs\ncode\n```
            let re = Regex::new(r"(?s)```[\w\-\+]+:([^\n]+)\n(.*?)```").unwrap();
            
            for cap in re.captures_iter(&out_str) {
                let path_str = cap[1].trim();
                let content = &cap[2];
                
                // Validate path (basic check)
                if path_str.contains("..") || path_str.starts_with('/') {
                    // Skip unsafe paths for now
                    continue;
                }

                let path = Path::new(workspace_path).join(path_str);
                
                if let Some(parent) = path.parent() {
                    // Ignore errors (e.g. if parent exists)
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                
                match tokio::fs::write(&path, content).await {
                    Ok(_) => {
                        // Publish FileWritten event?
                        let _ = bus.publish(Event {
                             id: Uuid::new_v4(),
                             session_id,
                             trace_id: Uuid::new_v4(),
                             timestamp: Utc::now(),
                             worker_id: "system".to_string(),
                             event_type: "FileWritten".to_string(),
                             payload: format!("Auto-saved file: {}", path_str),
                        }).await;
                    },
                    Err(_) => {
                         // ignore write error
                    }
                }
            }
            
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

    #[tokio::test]
    async fn test_ai_parse_and_write() {
        // Mock output with a markdown block
        // echo will print the prompt, so we simulate the prompt CONTAINING the output we want to parse.
        // We put the code block in the DESCRIPTION so it ends up in the prompt argument.
        
        let worker = AiGenericWorker::new(
            "Writer-Bot".to_string(),
            WorkerRole::Coder,
            "echo".to_string(),
            None,
        );

        let bus = Arc::new(InMemoryEventBus::new(10));
        let task = Task {
            id: Uuid::new_v4(),
            name: "Write Code".to_string(),
            description: "ignored".to_string(),
            status: TaskStatus::Pending,
            assigned_worker: None,
        };
        
        let temp_dir = std::env::temp_dir().join(format!("test_ai_writer_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        
        let code_content = "fn main() { println!(\"Hello\"); }";
        let relative_path = "src/hello.rs";
        let markdown = format!("Here is the code:\n```rust:{}\n{}\n```", relative_path, code_content);
        
        let task_with_payload = Task {
            description: markdown, 
            ..task
        };

        let result = worker.execute(bus, &task_with_payload, temp_dir.to_str().unwrap(), Uuid::new_v4()).await;
        assert!(result.is_ok());

        // Check file existence
        let file_path = temp_dir.join(relative_path);
        
        // Assert failure first (TDD)
        if !file_path.exists() {
             // Clean up before panicking to avoid littering
             std::fs::remove_dir_all(&temp_dir).unwrap();
             panic!("File {:?} was NOT created by auto-write (Expected behavior: TDD Failure)", file_path);
        }
        
        let content = std::fs::read_to_string(file_path).unwrap();
        assert_eq!(content.trim(), code_content);
        
        std::fs::remove_dir_all(temp_dir).unwrap();
    }
}
