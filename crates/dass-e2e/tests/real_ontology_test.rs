use anyhow::Result;
use async_trait::async_trait;
use dass_engine::agents::cli_client::AiCliClient;
use dass_engine::interaction::UserInteraction;
use dass_engine::orchestrator::Orchestrator;
use serde_json::json;
use std::path::PathBuf;

// Mock UI
struct TestUi;
#[async_trait]
impl UserInteraction for TestUi {
    async fn ask_for_feature(&self, _prompt: &str) -> Result<String> {
        Ok("add, subtract".to_string())
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

// Robust Mock Client
#[derive(Clone)]
struct MockSchemaCompliantClient;

impl MockSchemaCompliantClient {
    fn generate_artifact(kind: &str) -> String {
        match kind {
            "Requirement" => json!([{
                "name": "REQ-001",
                "kind": "Kind_Requirement",
                "req_type": "Functional",
                "priority": "Must Have",
                "user_story": "As a user, I want specific functionality.",
                "acceptance_criteria": ["Criteria 1"]
            }])
            .to_string(),
            "Code" => json!([{
                "name": "src/main.rs",
                "kind": "Kind_Code",
                "path": "src/main.rs",
                "language": "rust",
                "content": "fn main() {}"
            }])
            .to_string(),
            // Default for unknown artifacts or context dependencies
            _ => json!([{
                "name": format!("{}-001", kind),
                "kind": format!("Kind_{}", kind),
                "description": "Auto-generated mock artifact"
            }])
            .to_string(),
        }
    }

    fn generate_verification_result() -> String {
        json!({
            "score": 1.0,
            "feedback": "Looks good!",
            "passed": true
        })
        .to_string()
    }
}

#[async_trait]
impl AiCliClient for MockSchemaCompliantClient {
    async fn prompt(
        &self,
        prompt_text: &str,
        _options: dass_engine::graph::executor::ExecutionOptions,
    ) -> Result<String> {
        // println!("MOCK PROMPT: {}", prompt_text); // Debugging

        // 1. Check for Verification prompt (usually asks to verify X)
        if prompt_text.contains("verify the") || prompt_text.contains("Evaluate the") {
            return Ok(Self::generate_verification_result());
        }

        // 2. Check for Artifact Generation prompt using robust structure trigger
        // Pattern: "Please generate the [ArtifactName] artifact."
        if let Some(rest) = prompt_text.rsplit("Please generate the ").next() {
            if let Some(artifact_name) = rest.split(" artifact.").next() {
                let name = artifact_name.trim();
                return Ok(Self::generate_artifact(name));
            }
        }

        // 3. Fallback: Try to detect via context injection or legacy prompt phrasing
        if prompt_text.contains("Requirement") {
            return Ok(Self::generate_artifact("Requirement"));
        } else if prompt_text.contains("Code") {
            return Ok(Self::generate_artifact("Code"));
        }

        // Default empty JSON if nothing matches (should not happen in this test)
        Ok("{}".to_string())
    }
}

#[tokio::test]
async fn test_small_ontology_workflow() -> Result<()> {
    // Path to repo root
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.parent().unwrap().parent().unwrap();

    // We use ontology-software-engineering as base for prompts/schemas
    let base_ontology_path = root_dir.join("ontology-software-engineering");

    // We use mini_ontology for the graph (Small, Controlled)
    let mini_ontology_path = manifest_dir.join("fixtures/mini_ontology/ontology.json");
    let ontology_json = std::fs::read_to_string(&mini_ontology_path)
        .expect("Failed to read mini_ontology/ontology.json");

    // Work dir
    let work_dir = manifest_dir.join("../../target/test-work-dir-small");
    if work_dir.exists() {
        std::fs::remove_dir_all(&work_dir)?;
    }
    std::fs::create_dir_all(&work_dir)?;

    let client = MockSchemaCompliantClient;

    let mut orchestrator = Orchestrator::new_with_metamodel(
        client,
        "small-app".to_string(),
        "Small App".to_string(),
        work_dir,
        &ontology_json,
        Some(&base_ontology_path), // Use real prompts
    )
    .await?;

    // This runs the whole defined workflow:
    // 1. ProductManager creates Requirement
    // 2. QA verifies Requirement
    // 3. Engineer implements Code
    // 4. Engineer verifies Code
    orchestrator.run(&TestUi).await?;

    Ok(())
}
