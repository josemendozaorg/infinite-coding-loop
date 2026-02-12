use anyhow::Result;
use dass_engine::{agents::cli_client::ShellCliClient, orchestrator::Orchestrator};
use tempfile::tempdir;

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

    // We expect one of the identified actions to be ProductManager -> creates -> Requirement
    let actions = orchestrator.identify_next_actions();
    assert!(
        !actions.is_empty(),
        "Should identify at least one starting action"
    );

    let pm_action = actions
        .iter()
        .find(|a| a.agent == "ProductManager" && a.target == "Requirement");
    assert!(
        pm_action.is_some(),
        "ProductManager should be ready to create Requirement. Identified actions: {:?}",
        actions
    );
    let pm_action = pm_action.unwrap();
    assert_eq!(pm_action.relation, "creates");

    Ok(())
}
