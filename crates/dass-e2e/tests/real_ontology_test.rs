use anyhow::Result;
use async_trait::async_trait;
use dass_engine::agents::cli_client::AiCliClient;
use dass_engine::interaction::UserInteraction;
use dass_engine::orchestrator::Orchestrator;
use std::path::PathBuf;

// Mock UI
struct TestUi;
#[async_trait]
impl UserInteraction for TestUi {
    async fn ask_for_feature(&self, _prompt: &str) -> Result<String> {
        Ok("add, subtract".to_string()) // Matches user query
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

// Mock Client that returns the specific JSON
#[derive(Clone)]
struct MockPanicClient;

#[async_trait]
impl AiCliClient for MockPanicClient {
    async fn prompt(&self, prompt_text: &str) -> Result<String> {
        //println!("MOCK PROMPT: {}", prompt_text); // Comment out debug
        if prompt_text.contains("Requirement") {
            Ok(r#"[
  {
    "name": "REQ-001",
    "kind": "Kind_Requirement",
    "req_type": "Functional",
    "priority": "Must Have",
    "user_story": "As a user, I want to add two numbers, so that I can get their sum.",
    "acceptance_criteria": [
      "Given two positive integers, the calculator should return their correct sum."
    ]
  }
]"#
            .to_string())
        } else if prompt_text.contains("DesignSpec") {
            Ok(r#"[
  {
    "name": "DS-001",
    "kind": "Kind_DesignSpec",
    "description": "Design specification for the calculator."
  }
]"#
            .to_string())
        } else {
            Ok("{}".to_string())
        }
    }
}

#[tokio::test]
async fn test_real_ontology_panic_repro() -> Result<()> {
    // Path to REAL ontology
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.parent().unwrap().parent().unwrap();
    let ontology_path = root_dir.join("ontology-software-engineering"); // Real path

    // Work dir
    let work_dir = manifest_dir.join("../../target/test-work-dir-panic");
    if work_dir.exists() {
        std::fs::remove_dir_all(&work_dir)?;
    }
    std::fs::create_dir_all(&work_dir)?;

    let ontology_json_path = ontology_path.join("ontology.json");
    let ontology_json =
        std::fs::read_to_string(&ontology_json_path).expect("Failed to read real ontology.json");

    let client = MockPanicClient;

    let mut orchestrator = Orchestrator::new_with_metamodel(
        client,
        "panic-app".to_string(),
        "Panic App".to_string(),
        work_dir,
        &ontology_json,
        Some(&ontology_path),
    )
    .await?;

    // This should NOT panic ungracefully. It SHOULD return an error because the mock JSON is invalid against schema.
    orchestrator.run(&TestUi).await?;

    Ok(())
}
