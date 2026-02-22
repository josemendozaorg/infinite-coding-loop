use anyhow::Result;
use async_trait::async_trait;
use pulpo_engine::agents::cli_client::mocks::MockCliClient;
use pulpo_engine::interaction::UserInteraction;
use pulpo_engine::orchestrator::Orchestrator;
use std::path::PathBuf;

struct TestUi {
    _start_time: std::time::Instant,
}

impl TestUi {
    fn new() -> Self {
        Self {
            _start_time: std::time::Instant::now(),
        }
    }
}

#[async_trait]
impl UserInteraction for TestUi {
    async fn ask_for_feature(&self, _prompt: &str) -> Result<String> {
        Ok("Build a simple CLI tool that prints Hello World in Rust".to_string())
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

    fn log_info(&self, _msg: &str) {}
    fn log_error(&self, _msg: &str) {}
    fn start_step(&self, _msg: &str) {}
    fn end_step(&self, _msg: &str) {}
    fn render_artifact(&self, _kind: &str, _data: &serde_json::Value) {}
}

#[tokio::test]
async fn test_mock_orchestrator_loop() -> Result<()> {
    // 1. Setup paths
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures_dir = manifest_dir.join("fixtures/mini_pulpo_ontology");
    let ontology_path = fixtures_dir.join("ontology.json");
    let ontology_json = std::fs::read_to_string(&ontology_path)?;

    let work_dir = manifest_dir.join("../../target/test-work-dir-mocked-ai");
    if work_dir.exists() {
        std::fs::remove_dir_all(&work_dir)?;
    }
    std::fs::create_dir_all(&work_dir)?;

    // 2. Initialize the Mock Client
    let mock_client = MockCliClient::new();

    // We expect the LLM to output some generic responses for requirements and architecture.
    // For the code step, we simulate creating a code.json file.
    let work_dir_clone = work_dir.clone();
    mock_client.add_action(|_prompt| {
        Ok(r#"{ "content": "Mock Requirement Document Generated" }"#.to_string())
    });
    mock_client.add_action(|_prompt| {
        Ok(r#"{ "content": "Mock Architecture Spec Generated" }"#.to_string())
    });

    mock_client.add_action(move |_prompt| {
        // Simulate code generation by writing to code.json
        let file_path = work_dir_clone.join("code.json");
        let content = serde_json::json!({
            "files": [
                {
                    "path": "src/main.rs",
                    "content": "fn main() { println!(\"Hello World\"); }"
                }
            ]
        });
        std::fs::write(file_path, content.to_string())?;
        Ok("MOCK_CODE_GENERATED".to_string())
    });

    for _ in 0..10 {
        mock_client.add_response("MOCK_RESPONSE_OK".to_string());
    }

    // 3. Initialize Orchestrator
    let mut orchestrator = Orchestrator::<MockCliClient>::new_with_metamodel(
        mock_client.clone(),
        "test-app-mock".to_string(),
        "Test App Mock".to_string(),
        work_dir.clone(),
        &ontology_json,
        Some(&fixtures_dir),
    )
    .await?
    .with_max_iterations(3); // Just enough to trigger some agent actions

    // 4. Run the Loop
    orchestrator.run(&TestUi::new()).await?;

    // 5. Verify the mocked code.json exists and contains our mocked files!
    let code_artifact_path = work_dir.join("code.json");
    assert!(
        code_artifact_path.exists(),
        "code.json should have been created by the mocked closure"
    );

    let content = std::fs::read_to_string(&code_artifact_path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;

    assert!(
        json.get("files").is_some() && json["files"].is_array(),
        "code.json should have the mocked files array"
    );

    Ok(())
}
