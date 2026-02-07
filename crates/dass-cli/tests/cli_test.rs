use anyhow::Result;
use async_trait::async_trait;
use dass_engine::{
    agents::cli_client::ShellCliClient, interaction::UserInteraction, orchestrator::Orchestrator,
};
use serde_json::Value;
use tempfile::tempdir;

struct TestInteraction;

#[async_trait]
impl UserInteraction for TestInteraction {
    async fn ask_user(&self, _prompt: &str) -> Result<String> {
        Ok("test input".to_string())
    }
    async fn ask_for_feature(&self, _prompt: &str) -> Result<String> {
        Ok("Create a calculator app".to_string())
    }
    async fn confirm(&self, _prompt: &str) -> Result<bool> {
        Ok(true)
    }
    async fn select_option(&self, _prompt: &str, _options: &[String]) -> Result<usize> {
        Ok(0)
    }
    fn render_artifact(&self, _kind: &str, _data: &Value) {}
    fn start_step(&self, _name: &str) {}
    fn end_step(&self, _name: &str) {}
    fn log_info(&self, _msg: &str) {}
    fn log_error(&self, _msg: &str) {}
}

#[tokio::test]
async fn test_orchestration_cycle() -> Result<()> {
    let dir = tempdir()?;
    // Use gemini provider but it won't be called if we don't have actions or if we mock them
    // Actually, we want to see if it identifies the first action.
    let client = ShellCliClient::new("gemini", dir.path().to_string_lossy().to_string());

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let ontology_path = workspace_root.join("ontology-software-engineering");

    let mut orchestrator = Orchestrator::new(
        client,
        "test-app-id".to_string(),
        "TestApp".to_string(),
        dir.path().to_path_buf(),
    )
    .await?;

    // Override the graph with one loaded from the correct path
    let metamodel_json = std::fs::read_to_string(ontology_path.join("ontology.json"))?;
    orchestrator.executor.graph = dass_engine::graph::DependencyGraph::load_from_metamodel(
        &metamodel_json,
        Some(&ontology_path),
    )?;

    // Seed SoftwareApplication
    orchestrator.artifacts.insert(
        "SoftwareApplication".to_string(),
        serde_json::json!({ "name": "TestApp", "goal": "Create a calculator app" }),
    );

    let ui = TestInteraction;

    // We expect the first iteration to identify ProductManager -> creates -> Requirement
    // and attempt to call the AI. Since we don't want to call real AI in tests,
    // we might just want to verify state or run it in a way that doesn't call it.

    // For now, let's just see if it identifies actions.
    let actions = orchestrator.identify_next_actions();
    assert!(
        !actions.is_empty(),
        "Should identify at least one starting action (ProductManager creating Requirement)"
    );
    assert_eq!(actions[0].agent, "ProductManager");
    assert_eq!(actions[0].target, "Requirement");
    assert_eq!(actions[0].relation, "creates");

    Ok(())
}
