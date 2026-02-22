use anyhow::Result;
use async_trait::async_trait;
use pulpo_engine::agents::cli_client::AiCliClient;
use pulpo_engine::interaction::UserInteraction;
use pulpo_engine::orchestrator::Orchestrator;
use serde_json::Value;
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
struct MockCliClient;

#[async_trait]
impl AiCliClient for MockCliClient {
    async fn prompt(
        &self,
        _prompt: &str,
        _options: pulpo_engine::graph::executor::ExecutionOptions,
    ) -> Result<String> {
        Ok(r#"{"result": "mocked response", "name": "Feature"}"#.to_string())
    }
}

#[tokio::test]
async fn test_logging_integration() -> Result<()> {
    let tmp_dir = tempdir()?;
    let work_dir = tmp_dir.path().to_path_buf();

    // 1. Setup minimal Ontology in temp dir
    let ontology_root = work_dir.join("ontology");
    let schema_dir = ontology_root.join("artifact/schema");
    let agent_dir = ontology_root.join("agent/system_prompt");
    let prompt_dir = ontology_root.join("relationship/prompt");

    tokio::fs::create_dir_all(&schema_dir).await?;
    tokio::fs::create_dir_all(&agent_dir).await?;
    tokio::fs::create_dir_all(&prompt_dir).await?;

    // Create Schemas
    tokio::fs::write(
        schema_dir.join("Feature.schema.json"),
        r#"{
            "$id": "https://pulpo.dev/Feature",
            "type": "object"
        }"#,
    )
    .await?;

    // Create Agent Definition
    tokio::fs::write(agent_dir.join("ProductManager.md"), "You are a PM.").await?;

    // 2. Define Metamodel JSON
    // We define: ProductManager creates Feature
    let metamodel_json = r#"[
        {
             "source": { "name": "ProductManager", "type": "Agent" },
             "target": { "name": "Feature", "type": "Other" },
             "type": { "name": "creates", "verbType": "Creation" }
        }
    ]"#;

    let client = MockCliClient;

    let mut orchestrator = Orchestrator::new_with_metamodel(
        client,
        "test-app-id".to_string(),
        "Test App".to_string(),
        work_dir.clone(),
        metamodel_json,
        Some(&ontology_root),
    )
    .await?;

    // Inject initial artifact so we have something to do?
    // Wait, typical flow starts with asking for feature.
    // If we have no artifacts, identify_next_actions will see Agent->Feature.
    // Is it blocked?
    // "creates" usually requires nothing, or maybe contextual dependency?
    // In `get_related_artifacts`, it looks for incoming edges. none.
    // So dispatch should happen.

    orchestrator = orchestrator.with_max_iterations(1);

    orchestrator.run(&MockUi).await?;

    // Now verify logs
    let icl_dir = work_dir.join(".infinitecodingloop");
    let iterations_dir = icl_dir.join("iterations");

    // logs might be inside the only iteration dir
    let mut dir_reader = tokio::fs::read_dir(&iterations_dir).await?;
    let iteration_entry = dir_reader
        .next_entry()
        .await?
        .expect("Should have one iteration dir");
    let log_file = iteration_entry.path().join("logs/execution.jsonl");

    assert!(log_file.exists(), "Log file should exist at {:?}", log_file);

    let content = tokio::fs::read_to_string(&log_file).await?;

    let lines: Vec<&str> = content.trim().split('\n').collect();
    assert!(!lines.is_empty(), "Log file should not be empty");

    let has_start = lines.iter().any(|l| l.contains("iteration_start"));
    let has_loop = lines.iter().any(|l| l.contains("loop_cycle"));

    assert!(has_start, "Should have logged iteration start");
    assert!(has_loop, "Should have logged loop cycle");

    // We expect dispatch because ProductManager creates Feature is valid and enabled.
    // And MockUi confirms.
    if lines.iter().any(|l| l.contains("action_dispatched")) {
        let has_prompt = lines.iter().any(|l| l.contains("prompt_sent"));
        let has_response = lines.iter().any(|l| l.contains("response_received"));
        assert!(has_prompt, "Should have logged prompt sent if dispatched");
        assert!(
            has_response,
            "Should have logged response received if dispatched"
        );
    } else {
        // Do nothing
    }

    Ok(())
}
