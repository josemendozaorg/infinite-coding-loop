use crossterm::event::KeyCode; // FIXED IMPORT (crossterm is direct dep, not via ratatui)
use ifcl_core::{
    learning::BasicLearningManager,
    planner::{LLMPlanner, Planner},
    AiCliAgent,
    AiGenericWorker,
    AppMode,
    BasicOrchestrator,
    CliExecutor,
    CliWorker,
    InMemoryEventBus,
    Orchestrator, // ADDED TRAIT
    SetupWizard,
    SqliteEventStore,
    TaskStatus,
    WorkerRole,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tui::app::App;
use tui::state::AppState;

#[tokio::test]
async fn test_happy_path_calculator_generation() {
    // 0. Configuration
    let test_goal = "Build a Rust Calculator";

    // Create temp dir
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    // We need the directory name as string for the TUI input
    // The TUI wizard usually expects a relative path but explicit path works too if we type it?
    // Actually, TUI wizard expects a simple name which it might use relative to CWD.
    // BUT we want to direct it to our temp dir.
    // If we type the absolute path into the wizard, it should work if the system supports it.
    let test_workspace_path = temp_dir.path().to_str().expect("path not valid utf8");
    println!("Test Workspace: {}", test_workspace_path);

    // 1. Setup Internals
    let bus: Arc<dyn ifcl_core::EventBus> = Arc::new(InMemoryEventBus::new(100)); // FIXED TYPE
                                                                                  // Use in-memory sqlite
    let store_result = SqliteEventStore::new("sqlite::memory:").await;
    let store = Arc::new(store_result.expect("Failed to create in-memory sqlite store"));

    let learning_manager = Arc::new(BasicLearningManager::new());
    let orchestrator: Arc<dyn Orchestrator> = Arc::new(BasicOrchestrator::new()); // FIXED TYPE
    let wizard = SetupWizard::new();
    let available_groups = vec![];

    let state = Arc::new(Mutex::new(AppState::new(available_groups, wizard)));

    let app = App::new(
        Arc::clone(&state),
        Arc::clone(&bus),
        store,
        Arc::clone(&orchestrator),
        learning_manager,
    );

    // 2. Drive UI: Main Menu -> New Game
    {
        let s = state.lock().unwrap();
        assert_eq!(s.mode, AppMode::MainMenu);
    }
    app.handle_input(KeyCode::Enter).await; // Select "New Game"
    {
        let s = state.lock().unwrap();
        assert_eq!(s.mode, AppMode::Setup);
    }

    // 3. Drive UI: Setup Wizard
    // Helper to type string
    async fn type_string(app: &App, text: &str) {
        for c in text.chars() {
            app.handle_input(KeyCode::Char(c)).await;
        }
        app.handle_input(KeyCode::Enter).await;
    }

    // Step 1: Goal
    type_string(&app, test_goal).await;

    // Step 2: Stack
    type_string(&app, "Rust").await;

    // Step 3: Workspace
    // INPUT THE FULL ABSOLUTE PATH OF TEMP DIR
    type_string(&app, test_workspace_path).await;

    // Step 4: Provider (Select Gemini)
    // tui/app.rs logic check:
    // Default is Basic.
    // Basic --(Down)--> Gemini
    app.handle_input(KeyCode::Down).await;
    app.handle_input(KeyCode::Enter).await; // Confirm Provider

    // Step 5: Team
    app.handle_input(KeyCode::Enter).await; // Default team

    // Step 6: Budget
    app.handle_input(KeyCode::Enter).await; // Default budget

    // Step 7: Summary
    app.handle_input(KeyCode::Enter).await; // Start!

    // 4. Verify Running State & AI Initialization
    let sid;
    {
        let s = state.lock().unwrap();
        assert_eq!(s.mode, AppMode::Running);
        assert!(s.current_session_id.is_some());
        sid = s.current_session_id.unwrap();
    }

    // 5. Replicate The Execution Loop (simplified version of main.rs)
    // We need to run this for reasonable amount of time or until completion.

    // Initialize Planner based on Wizard selection (Gemini)
    let planner: Arc<dyn Planner> = Arc::new(LLMPlanner {
        executor: CliExecutor::new(
            "gemini".to_string(),
            vec![
                "--approval-mode".to_string(),
                "yolo".to_string(),
                "--allowed-tools".to_string(),
                "run_shell_command".to_string(),
            ],
        ),
    });

    println!("Generating initial missions...");
    let mut missions = planner.generate_initial_missions(test_goal).await;
    for m in &mut missions {
        m.session_id = sid;
        m.workspace_path = Some(test_workspace_path.to_string());
        orchestrator.add_mission(m.clone()).await.unwrap();
    }

    // Execution Loop
    let max_iterations = 50; // Safety break
    let mut loop_count = 0;

    loop {
        if loop_count > max_iterations {
            break;
        }
        loop_count += 1;

        let missions = orchestrator.get_missions().await.unwrap();
        let mut pending_task = None;
        let mut all_done = true;

        for m in missions {
            for t in m.tasks {
                if t.status == TaskStatus::Pending {
                    let w_name = t
                        .assigned_worker
                        .clone()
                        .unwrap_or_else(|| "Bot".to_string());
                    pending_task = Some((m.id, t.id, w_name));
                    all_done = false;
                    break;
                }
                if t.status == TaskStatus::Running {
                    all_done = false;
                }
            }
            if pending_task.is_some() {
                break;
            }
        }

        if let Some((mid, tid, worker_name)) = pending_task {
            println!("Executing Task {} with {}", tid, worker_name);
            let w_lower = worker_name.to_lowercase();

            // Instantiate real worker
            let worker: Box<dyn ifcl_core::Worker> = if w_lower.contains("gemini")
                || w_lower.contains("planner")
                || w_lower.contains("git")
            {
                // Note: Git-Bot usually handled by CliWorker in main.rs, but we want to force AI here?
                // Or stick to main.rs logic?
                // main.rs logic:
                // if w_lower.contains("gemini") ... AiGenericWorker
                // else CliWorker

                // If the planner returns "Git-Bot", main.rs uses CliWorker.
                // If the planner returns "Gemini-Coder", main.rs uses AiGenericWorker.

                if w_lower.contains("gemini") {
                    Box::new(AiGenericWorker::new(
                        worker_name.clone(),
                        WorkerRole::Coder,
                        Box::new(AiCliAgent::new(
                            "gemini".to_string(),
                            None,
                            vec![
                                "--approval-mode".to_string(),
                                "yolo".to_string(),
                                "--allowed-tools".to_string(),
                                "run_shell_command".to_string(),
                            ],
                        )),
                    ))
                } else {
                    Box::new(CliWorker::new(&worker_name, WorkerRole::Coder))
                }
            } else {
                Box::new(CliWorker::new(&worker_name, WorkerRole::Coder))
            };

            let _ = orchestrator
                .execute_task(Arc::clone(&bus), mid, tid, worker.as_ref())
                .await;
        } else if all_done {
            println!("All missions completed.");
            break;
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // 6. Verification

    // Specialized search that looks for a cargo project root (has Cargo.toml and src/main.rs)
    fn find_cargo_project(
        dir: &std::path::Path,
    ) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
        if dir.is_dir() {
            let cargo = dir.join("Cargo.toml");
            let main = dir.join("src/main.rs");
            if cargo.exists() && main.exists() {
                return Some((cargo, main));
            }

            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(found) = find_cargo_project(&path) {
                            return Some(found);
                        }
                    }
                }
            }
        }
        None
    }

    let (cargo_toml_path, main_rs_path) =
        find_cargo_project(std::path::Path::new(test_workspace_path))
            .expect("Could not find a valid Cargo project (Cargo.toml + src/main.rs) in workspace");

    let mut found_main = false;
    let mut found_cargo = false;

    // Poll for files (they might be created async if we didn't wait enough, but loop should cover it)
    if main_rs_path.exists() {
        found_main = true;
    }
    if cargo_toml_path.exists() {
        found_cargo = true;
    }

    if !found_main {
        println!("WARNING: src/main.rs not found immediately. Listing workspace:");
        let _ = std::process::Command::new("ls")
            .arg("-R")
            .arg(test_workspace_path)
            .status();
    }

    assert!(found_cargo, "Cargo.toml was not created");
    assert!(found_main, "src/main.rs was not created");

    let main_content = std::fs::read_to_string(main_rs_path).unwrap();
    assert!(
        main_content.contains("fn main"),
        "main.rs does not contain 'fn main'"
    );

    // Cleanup handled by tempfile crate
}
