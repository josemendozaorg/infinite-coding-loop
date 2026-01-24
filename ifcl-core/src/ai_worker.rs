use uuid::Uuid;
use std::sync::Arc;
use async_trait::async_trait;
use crate::{Worker, WorkerRole, Task, EventBus, Event, WorkerOutputPayload, WorkerProfile};
use chrono::Utc;
use serde_json;
use regex::Regex;
use std::path::Path;
#[derive(Debug, Clone)]
pub struct AiGenericWorker {
    pub profile: WorkerProfile,
    // We wrap Agent in Arc/Mutex or just Arc because Worker must be Send+Sync+Clone
    // Box<dyn Agent> is not Clone by default.
    // Worker trait requires Clone? No, Worker logic in main.rs clones it?
    // main.rs: check usage. Worker trait doesn't require Clone, but we clone it?
    // main.rs logic: Box::new(AiGenericWorker...).
    // struct AiGenericWorker has #[derive(Clone)].
    // We need Arc<dyn Agent> to support Clone.
    pub agent: std::sync::Arc<dyn crate::Agent>,
}

impl AiGenericWorker {
    pub fn new(name: String, role: WorkerRole, agent: Box<dyn crate::Agent>) -> Self {
        Self {
            profile: WorkerProfile {
                name,
                role,
                model: Some("agent-based".to_string()), 
            },
            agent: std::sync::Arc::from(agent),
        }
    }
}

#[async_trait]
impl Worker for AiGenericWorker {
    fn id(&self) -> &str {
        &self.profile.name
    }
    
    fn role(&self) -> WorkerRole {
        self.profile.role
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
        
        // Stream Output via Agent
        // Agent execute returns result directly in current impl, but we want streaming?
        // Our updated Agent trait supports streaming via mpsc.
        
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        
        let bus_clone = bus.clone();
        let session_id_clone = session_id;
        let worker_id_clone = self.profile.name.clone();

        // Spawn output forwarder
        tokio::spawn(async move {
            while let Some((line, is_stderr)) = rx.recv().await {
                 let _ = bus_clone.publish(Event {
                     id: Uuid::new_v4(),
                     session_id: session_id_clone,
                     trace_id: Uuid::new_v4(),
                     timestamp: Utc::now(),
                     worker_id: worker_id_clone.clone(),
                     event_type: "WorkerOutput".to_string(),
                     payload: serde_json::to_string(&WorkerOutputPayload {
                         content: line,
                         is_stderr, 
                     }).unwrap_or_default(),
                 }).await;
            }
        });

        // Execute Agent
        let out_str = self.agent.execute(&prompt, workspace_path, Some(tx)).await?;
        
        // Code Concierge Logic (Regex Parse)
        // ... (Keep existing logic)
        let status_success = true; // Agent executes throws error if failed? 
        // Agent execute logic returns Output if status success, or Error if fail.
        // So if we are here, success.

        if status_success {
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
            // Unreachable if agent.execute returns Err on failure
            anyhow::bail!("AI Worker Failed")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InMemoryEventBus;
    use crate::TaskStatus;
    use crate::AiCliAgent;
    use crate::agent::MockAgent;

    #[tokio::test]
    async fn test_ai_generic_worker_echo() {
        // Use 'echo' as a mock AI binary
        let agent = Box::new(AiCliAgent::new("echo".to_string(), None));
        let worker = AiGenericWorker::new(
            "Echo-Bot".to_string(),
            WorkerRole::Coder,
            agent,
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
        let agent = Box::new(AiCliAgent::new("echo".to_string(), Some("gpt-4".to_string())));
        let worker = AiGenericWorker::new(
            "Echo-Bot-Pro".to_string(),
            WorkerRole::Coder,
            agent,
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
        let agent = Box::new(AiCliAgent::new("echo".to_string(), None));
        let worker = AiGenericWorker::new(
            "Writer-Bot".to_string(),
            WorkerRole::Coder,
            agent,
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
    
    #[tokio::test]
    async fn test_ai_worker_with_mock_agent() {
        let mock_agent = MockAgent {
            id: "mock".to_string(),
            output_sequence: vec!["Mock Success".to_string()],
        };
        
        let worker = AiGenericWorker::new(
            "Test-Worker".to_string(),
            WorkerRole::Coder,
            Box::new(mock_agent),
        );
        
        let bus = Arc::new(InMemoryEventBus::new(10));
        let task = Task {
            id: Uuid::new_v4(),
            name: "Mock Task".to_string(),
            description: "Desc".to_string(),
            status: TaskStatus::Pending,
            assigned_worker: None,
        };
        let workspace = std::env::temp_dir();
        
        let result = worker.execute(bus, &task, workspace.to_str().unwrap(), Uuid::new_v4()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Mock Success");
    }
}
