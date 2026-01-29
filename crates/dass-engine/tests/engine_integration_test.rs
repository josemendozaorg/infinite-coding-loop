use anyhow::Result;
use dass_engine::agents::cli_client::AiCliClient;
use dass_engine::orchestrator::Orchestrator;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// Local MockCliClient definition to avoid visibility issues
#[derive(Clone)]
pub struct MockCliClient {
    pub responses: Arc<Mutex<VecDeque<String>>>,
}

impl MockCliClient {
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses.into())),
        }
    }
}

impl AiCliClient for MockCliClient {
    fn prompt(&self, _prompt: &str) -> Result<String> {
        let mut guard = self.responses.lock().unwrap();
        if let Some(res) = guard.pop_front() {
            Ok(res)
        } else {
            Ok("MOCK_RESPONSE".to_string())
        }
    }
}

#[tokio::test]
async fn test_engine_end_to_end_flow() -> Result<()> {
    // 1. Setup Mock Client with canned responses for agents
    let mock_responses = vec![
        // Product Manager Response (Requirement)
        r#"
- id: 00000000-0000-0000-0000-000000000001
  name: "Spinner Requirement"
  user_story: 'As a user I want to see a spinner'
  acceptance_criteria: ['Spinner visible within 100ms']
  kind: 'Requirement'
  req_type: 'Functional'
  priority: 'Must Have'
"#
        .to_string(),
        // Architect Response (Design Spec / Feature)
        r#"
{
  "id": "00000000-0000-0000-0000-000000000002",
  "name": "Spinner Feature",
  "description": "A spinner component",
  "req_ids": ["00000000-0000-0000-0000-000000000001"],
  "priority": "High",
  "status": "Proposed",
  "business_value": "Medium",
  "ui_spec": "Blue spinner",
  "logic_spec": "Rotate 360",
  "data_spec": "None",
  "verification_plan": "Check rotation",
  "metadata": {},
  "kind": "Feature"
}
"#
        .to_string(),
        // Architect Response (Project Structure)
        r#"
{
  "id": "00000000-0000-0000-0000-000000000003",
  "name": "Spinner Structure",
  "layout": [
      {
          "name": "src",
          "type": "Directory",
          "children": [
              {
                  "name": "components",
                  "type": "Directory",
                  "children": [
                      {
                          "name": "Spinner.rs",
                          "type": "File"
                      }
                  ]
              }
          ]
      }
  ],
  "kind": "ProjectStructure",
  "metadata": {}
}
"#
        .to_string(),
        // Engineer Response (Plan)
        r#"
{
  "id": "00000000-0000-0000-0000-000000000004",
  "name": "Spinner Plan",
  "kind": "Plan",
  "tasks": [
    {
      "id": "task-1",
      "description": "Create file",
      "dependencies": [],
      "estimated_duration": "1h"
    }
  ],
  "metadata": {}
}
"#
        .to_string(),
    ];

    let client = MockCliClient::new(mock_responses);

    // 2. Initialize Orchestrator
    let app_id = Uuid::new_v4().to_string();
    let work_dir = PathBuf::from("./tmp/test_integration");
    let _ = tokio::fs::remove_dir_all(&work_dir).await; // Clean start
    tokio::fs::create_dir_all(&work_dir).await?;

    let mut orchestrator =
        Orchestrator::new(client, app_id.clone(), "Test App".to_string(), work_dir).await?;

    // 3. Setup Mock UI
    use async_trait::async_trait;
    use dass_engine::interaction::UserInteraction;
    use serde_json::Value;

    struct MockUI;
    #[async_trait]
    impl UserInteraction for MockUI {
        fn log_info(&self, msg: &str) {
            println!("[INFO] {}", msg);
        }
        fn log_error(&self, msg: &str) {
            println!("[ERROR] {}", msg);
        }
        fn start_step(&self, msg: &str) {
            println!("[START] {}", msg);
        }
        fn end_step(&self, msg: &str) {
            println!("[END] {}", msg);
        }
        async fn ask_for_feature(&self, _prompt: &str) -> Result<String> {
            Ok("I want a spinner".to_string())
        }
        async fn ask_user(&self, _prompt: &str) -> Result<String> {
            Ok("yes".to_string())
        }
        async fn confirm(&self, _prompt: &str) -> Result<bool> {
            Ok(true)
        }
        async fn select_option(&self, _prompt: &str, _options: &[String]) -> Result<usize> {
            Ok(0)
        }

        // Stubs for legacy types (still present in trait)
        fn render_requirements(&self, _reqs: &[Value]) {}
        fn render_spec(&self, _spec: &Value) {}
        fn render_plan(&self, _plan: &Value) {}
    }

    let mock_ui = MockUI;

    // 4. Execute Flow
    orchestrator.run(&mock_ui).await?;

    Ok(())
}
