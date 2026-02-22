#![cfg(feature = "e2e")]

use anyhow::Result;
use pulpo_engine::agents::cli_client::{AiCliClient, ShellCliClient};
use std::path::PathBuf;

#[tokio::test]
async fn test_ai_cli_file_generation() -> Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let work_dir = manifest_dir.join("../../target/test-cli-gen");

    if work_dir.exists() {
        std::fs::remove_dir_all(&work_dir)?;
    }
    std::fs::create_dir_all(&work_dir)?;

    let client = ShellCliClient::new("gemini", work_dir.to_string_lossy().to_string())
        .with_yolo(true)
        .with_model("gemini-2.5-flash".to_string());

    // Prompt the AI to create a specific file in the current directory
    let prompt = "Create a file named 'hello.txt' containing the text 'Hello from AI CLI' in the current directory.";

    // We call prompt. Since we use --approval-mode yolo, the AI CLI (gemini) should execute the file creation.
    let response = client.prompt(prompt, Default::default()).await?;
    println!("AI Response: {}", response);

    // Verify the file exists in the work_dir
    let file_path = work_dir.join("hello.txt");
    assert!(
        file_path.exists(),
        "The file 'hello.txt' was not created in the work directory"
    );

    let content = std::fs::read_to_string(file_path)?;
    assert!(
        content.contains("Hello from AI CLI"),
        "File content is incorrect: {}",
        content
    );

    Ok(())
}
