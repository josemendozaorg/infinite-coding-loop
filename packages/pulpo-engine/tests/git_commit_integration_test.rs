use anyhow::Result;
use async_trait::async_trait;
use pulpo_engine::agents::cli_client::AiCliClient;
use pulpo_engine::interaction::UserInteraction;
use pulpo_engine::orchestrator::Orchestrator;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

struct MockUi;

#[async_trait]
impl UserInteraction for MockUi {
    async fn ask_user(&self, _prompt: &str) -> Result<String> {
        Ok("yes".to_string())
    }
    async fn ask_for_feature(&self, _prompt: &str) -> Result<String> {
        Ok("Test Feature".to_string())
    }
    async fn confirm(&self, _prompt: &str) -> Result<bool> {
        Ok(true)
    }
    async fn select_option(&self, _prompt: &str, _options: &[String]) -> Result<usize> {
        Ok(0)
    }
    fn start_step(&self, _msg: &str) {}
    fn end_step(&self, _msg: &str) {}
    fn render_artifact(&self, _kind: &str, _data: &Value) {}
    fn log_info(&self, _msg: &str) {}
    fn log_error(&self, _msg: &str) {}
}

#[derive(Clone)]
struct TrackingMockCliClient {
    pub prompts_received: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl AiCliClient for TrackingMockCliClient {
    async fn prompt(
        &self,
        prompt: &str,
        _options: pulpo_engine::graph::executor::ExecutionOptions,
    ) -> Result<String> {
        self.prompts_received
            .lock()
            .unwrap()
            .push(prompt.to_string());

        if prompt.contains("commit") || prompt.contains("git add .") {
            Ok("Git commit successful".to_string())
        } else {
            Ok(r#"{"result": "mocked artifact content", "name": "Feature"}"#.to_string())
        }
    }
}

#[tokio::test]
async fn test_git_commit_after_action() -> Result<()> {
    let tmp_dir = tempdir()?;
    let work_dir = tmp_dir.path().to_path_buf();

    let ontology_root = work_dir.join("ontology");
    let schema_dir = ontology_root.join("artifact/schema");
    let agent_dir = ontology_root.join("agent/system_prompt");
    let prompt_dir = ontology_root.join("relationship/prompt");

    tokio::fs::create_dir_all(&schema_dir).await?;
    tokio::fs::create_dir_all(&agent_dir).await?;
    tokio::fs::create_dir_all(&prompt_dir).await?;

    tokio::fs::write(
        schema_dir.join("Feature.schema.json"),
        r#"{
            "$id": "https://pulpo.dev/Feature",
            "type": "object"
        }"#,
    )
    .await?;

    tokio::fs::write(agent_dir.join("ProductManager.md"), "You are a PM.").await?;

    let metamodel_json = r#"[
        {
             "source": { "name": "ProductManager", "type": "Agent" },
             "target": { "name": "Feature", "type": "Other" },
             "type": { "name": "creates", "verbType": "Creation" }
        }
    ]"#;

    let prompts_received = Arc::new(Mutex::new(Vec::new()));
    let client = TrackingMockCliClient {
        prompts_received: prompts_received.clone(),
    };

    let mut orchestrator = Orchestrator::new_with_metamodel(
        client,
        "test-app-id".to_string(),
        "Test App".to_string(),
        work_dir.clone(),
        metamodel_json,
        Some(&ontology_root),
    )
    .await?;

    orchestrator = orchestrator.with_max_iterations(1);
    orchestrator.run(&MockUi).await?;

    let prompts = prompts_received.lock().unwrap();

    let commit_prompt_found = prompts
        .iter()
        .any(|p| p.contains("git add .") && p.contains("commit"));

    assert!(
        commit_prompt_found,
        "Expected AI CLI to receive a prompt containing git commit instructions. Prompts received: {:?}",
        prompts
    );

    Ok(())
}
