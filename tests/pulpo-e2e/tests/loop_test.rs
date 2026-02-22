#![cfg(feature = "e2e")]
use anyhow::Result;
use async_trait::async_trait;
use pulpo_engine::agents::cli_client::ShellCliClient;
use pulpo_engine::interaction::UserInteraction;
use pulpo_engine::orchestrator::Orchestrator;
use std::path::PathBuf;

// 1. Mock UI (To drive the Orchestrator)
// 1. Mock UI (To drive the Orchestrator)
struct TestUi {
    start_time: std::time::Instant,
}

impl TestUi {
    fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
        }
    }

    fn elapsed(&self) -> String {
        format!("{:.2?}", self.start_time.elapsed())
    }
}

#[async_trait]
impl UserInteraction for TestUi {
    async fn ask_for_feature(&self, _prompt: &str) -> Result<String> {
        println!("[{}] ASK_FOR_FEATURE: {}", self.elapsed(), _prompt);
        Ok("Build a simple CLI tool that prints Hello World in Rust".to_string())
    }
    async fn ask_user(&self, prompt: &str) -> Result<String> {
        println!("[{}] ASK_USER: {}", self.elapsed(), prompt);
        Ok("yes".to_string())
    }
    async fn confirm(&self, prompt: &str) -> Result<bool> {
        println!("[{}] CONFIRM: {}", self.elapsed(), prompt);
        Ok(true)
    }
    async fn select_option(&self, prompt: &str, options: &[String]) -> Result<usize> {
        println!(
            "[{}] SELECT_OPTION: {} {:?}",
            self.elapsed(),
            prompt,
            options
        );
        Ok(0)
    }

    // Logging stubs
    fn log_info(&self, msg: &str) {
        println!("[{}] [INFO] {}", self.elapsed(), msg);
    }
    fn log_error(&self, msg: &str) {
        println!("[{}] [ERROR] {}", self.elapsed(), msg);
    }

    fn start_step(&self, msg: &str) {
        println!("\n[{}] [STEP] {}", self.elapsed(), msg);
    }
    fn end_step(&self, msg: &str) {
        println!("[{}] [DONE] {}", self.elapsed(), msg);
    }

    // Artifact rendering stubs
    fn render_artifact(&self, kind: &str, _data: &serde_json::Value) {
        println!("[{}] [RENDER] Artifact: {}", self.elapsed(), kind);
    }
}

#[tokio::test]
async fn test_end_to_end_execution() -> Result<()> {
    // 1. Setup paths & content
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures_dir = manifest_dir.join("fixtures/mini_pulpo_ontology");
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
    .await?
    .with_max_iterations(15);

    // 3. Run the Loop
    // This will actually call gemini!
    orchestrator.run(&TestUi::new()).await?;

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
