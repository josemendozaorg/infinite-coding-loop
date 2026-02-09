use anyhow::Result;
use async_trait::async_trait;
use dass_engine::agents::cli_client::ShellCliClient;
use dass_engine::interaction::UserInteraction;
use dass_engine::orchestrator::Orchestrator;
use std::path::PathBuf;

// 1. Mock UI (To drive the Orchestrator)
struct TestUi;

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

    // Logging stubs
    fn log_info(&self, msg: &str) {
        println!("[INFO] {}", msg);
    }
    fn log_error(&self, msg: &str) {
        println!("[ERROR] {}", msg);
    }

    fn start_step(&self, msg: &str) {
        println!("\n[STEP] {}", msg);
    }
    fn end_step(&self, msg: &str) {
        println!("[DONE] {}", msg);
    }

    // Artifact rendering stubs
    fn render_artifact(&self, _kind: &str, _data: &serde_json::Value) {}
}

#[tokio::test]
async fn test_end_to_end_execution() -> Result<()> {
    // 1. Setup paths & content
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures_dir = manifest_dir.join("fixtures/mini_ontology");
    let ontology_path = fixtures_dir.join("ontology.json");
    let ontology_json = std::fs::read_to_string(&ontology_path)?;

    // Use a unique work dir for this test run inside the repo to avoid workspace validation issues
    let work_dir = manifest_dir.join("../../target/test-work-dir-real-ai");
    if work_dir.exists() {
        std::fs::remove_dir_all(&work_dir)?;
    }
    std::fs::create_dir_all(&work_dir)?;

    // 2. Initialize Orchestrator with Real Shell Client (gemini)
    // We point to the gemini executable and set the work_dir
    let client = ShellCliClient::new("gemini", work_dir.to_string_lossy().to_string())
        .with_yolo(true)
        .with_yolo(true)
        .with_model("gemini-2.5-flash".to_string());

    let mut orchestrator = Orchestrator::new_with_metamodel(
        client.clone(),
        "test-app-real".to_string(),
        "Test App Real".to_string(),
        work_dir.clone(),
        &ontology_json,
        Some(&fixtures_dir),
    )
    .await?;

    // 3. Run the Loop
    // This will actually call gemini!
    orchestrator.run(&TestUi).await?;

    // 4. Verification: Check for artifact existence
    // The agents should have created files in the work_dir
    // We expect at least the plan and requirements to be there if the orchestration finished.

    let entries = std::fs::read_dir(&work_dir)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()?;

    println!("Created Files in {:?}:", work_dir);
    for entry in &entries {
        println!("  - {:?}", entry.file_name().unwrap());
    }

    assert!(
        !entries.is_empty(),
        "No artifacts were created by the agents"
    );

    // We check for some common names the agents might use (or we can just check if NOT empty)
    // Since it's a real LLM, filenames might vary, but they should be there.

    // 5. Specific Verification for Scalable Code Artifact
    let code_artifact_path = work_dir.join("code.json");
    if code_artifact_path.exists() {
        let content = std::fs::read_to_string(&code_artifact_path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;

        if let Some(files) = json.get("files") {
            assert!(
                files.is_array(),
                "code.json 'files' should be an array (Reference-Based)"
            );
        } else {
            panic!(
                "code.json exists but is missing 'files' array. Content: {}",
                content
            );
        }
    }

    Ok(())
}
