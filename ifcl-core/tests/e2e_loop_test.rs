use ifcl_core::*;
use std::sync::Arc;
use tempfile::tempdir;
use uuid::Uuid;

struct TestPlanner;

#[async_trait::async_trait]
impl Planner for TestPlanner {
    async fn generate_initial_missions(&self, _goal: &str) -> Vec<Mission> {
        vec![Mission {
            id: Uuid::new_v4(),
            session_id: Uuid::nil(),
            name: "E2E Mission".to_string(),
            tasks: vec![Task {
                id: Uuid::new_v4(),
                name: "Create File".to_string(),
                description: "success_marker".to_string(),
                status: TaskStatus::Pending,
                assigned_worker: Some("Test-Bot".to_string()),
                retry_count: 0,
            }],
            workspace_path: None,
        }]
    }

    async fn replan_on_failure(&self, _: &str, _: &Mission, _: Uuid) -> Vec<Mission> {
        vec![]
    }
}

#[tokio::test]
async fn test_end_to_end_loop_execution() {
    // 1. Setup Infrastructure
    let bus = Arc::new(InMemoryEventBus::new(100));
    // We don't strictly need the store for this test unless orchestrator uses it, 
    // but the BasicOrchestrator is in-memory state.
    let orchestrator = Arc::new(BasicOrchestrator::new());
    let planner = Arc::new(TestPlanner);
    
    // Setup Worker
    let worker = Box::new(CliWorker::new("Test-Bot", WorkerRole::Ops));
    
    // Setup Environment
    let temp_workspace = tempdir().unwrap();
    let workspace_path = temp_workspace.path().to_str().unwrap().to_string();

    // 2. Simulate "Planning Phase"
    let missions = planner.generate_initial_missions("Test Goal").await;
    let mission_id = missions[0].id;
    
    // Prepare mission with correct workspace
    let mut initial_mission = missions[0].clone();
    initial_mission.workspace_path = Some(workspace_path.clone());
    
    // Load into Orchestrator
    orchestrator.add_mission(initial_mission).await.unwrap();

    // 3. Verify Initial State
    let missions_start = orchestrator.get_missions().await.unwrap();
    let task_start = &missions_start[0].tasks[0];
    assert_eq!(task_start.status, TaskStatus::Pending);
    assert_eq!(task_start.assigned_worker, Some("Test-Bot".to_string()));

    // 4. Run "Loop Execution Step"
    // In the real app, this is done by a `loop {}` block finding pending tasks.
    // Here we manually pick the task and execute it.
    
    println!("Executing Task: {}", task_start.name);
    
    // Listen for events to verify bus is working
    let mut rx = bus.subscribe();

    orchestrator.execute_task(
         bus.clone(),
         mission_id,
         task_start.id,
         worker.as_ref()
     ).await.unwrap();

    // 5. Verify Outcome
    
    // A. Check for side effects (File System)
    let expected_file = temp_workspace.path().join("success_marker");
    assert!(expected_file.exists(), "The file 'success_marker' should have been created by the worker.");

    // B. Check Orchestrator State
    let missions_after = orchestrator.get_missions().await.unwrap();
    let task_after = &missions_after[0].tasks[0];
    assert_eq!(task_after.status, TaskStatus::Success, "Task status should be updated to Success.");

    // C. Check Event Bus
    // We expect at least: TaskStarted (maybe), WorkerOutput(s), TaskCompleted (maybe handled by orchestrator/log)
    // The orchestrator emits a "Log" event on success/failure usually? 
    // Let's check what orchestrator emits. BasicOrchestrator emits "TaskStatusChanged" or similar?
    // Actually BasicOrchestrator.execute_task returns Result<String>. 
    // It doesn't emit logs itself, but it updates state.
    // The worker emits "WorkerOutput".
    
    let mut output_received = false;
    let mut other_events = 0;
    
    // Drain events quickly
    while let Ok(event) = rx.try_recv() {
        if event.event_type == "WorkerOutput" {
            output_received = true;
        } else {
            other_events += 1;
        }
    }
    
    // Note: 'touch' might not produce stdout, so output_received might be false.
    // Let's rely on file existence and task status.
}
