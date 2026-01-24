mod relationship;
mod cli;
mod state;
mod ui;
mod app;

use state::AppState;
use app::App;

use anyhow::Result;
use cli::CliArgs;
use chrono::Utc;
use clap::Parser;
use crossterm::{
    event::{self, Event as CEvent},
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ifcl_core::{
    Event, EventStore, InMemoryEventBus, SqliteEventStore, TaskStatus, 
    LoopStatus, WorkerRole,
    MarketplaceLoader, SetupWizard,
    WorkerOutputPayload, CliWorker, CliExecutor, AiGenericWorker, AiCliAgent,
    learning::{LearningManager, BasicLearningManager},
    planner::{Planner, BasicPlanner, LLMPlanner},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let _args = CliArgs::parse();

    // 1. Initialize Infrastructure (Shared)
    let bus: Arc<dyn ifcl_core::EventBus> = Arc::new(InMemoryEventBus::new(200));
    let store: Arc<dyn EventStore> = Arc::new(SqliteEventStore::new("sqlite://ifcl.db?mode=rwc").await?);
    let learning_manager: Arc<dyn LearningManager> = Arc::new(BasicLearningManager::new());
    let _memory_store: Arc<dyn ifcl_core::MemoryStore> = Arc::new(ifcl_core::memory::InMemoryMemoryStore::new());
    let orchestrator: Arc<dyn ifcl_core::Orchestrator> = Arc::new(ifcl_core::BasicOrchestrator::new());

    // 2. Check for Headless Mode
    if _args.is_headless() {
        println!("🚀 Starting Infinite Coding Loop in Headless Mode...");
        let goal = _args.goal.clone().unwrap_or_else(|| "General Autonomy".to_string());
        println!("Objective: {}", goal);
        
        let workspace = _args.workspace.clone();
        if let Some(ws) = &workspace {
             println!("Workspace: {}", ws);
        }

        let provider = _args.provider.clone();
        let planner: Arc<dyn Planner> = match provider.as_deref() {
            Some("gemini") => Arc::new(LLMPlanner { executor: CliExecutor::new("gemini".to_string()) }),
            Some("claude") => Arc::new(LLMPlanner { executor: CliExecutor::new("claude".to_string()) }),
            Some("opencode") => Arc::new(LLMPlanner { executor: CliExecutor::new("opencode".to_string()) }),
            _ => Arc::new(BasicPlanner),
        };
        
        let mut missions = planner.generate_initial_missions(&goal).await;
        let sid = Uuid::new_v4();
        for m in &mut missions {
            m.session_id = sid;
            m.workspace_path = workspace.clone();
            orchestrator.add_mission(m.clone()).await?;
            println!("Mission Created: {} (ID: {})", m.name, m.id);
        }

        let mut rx = bus.subscribe();
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                if event.event_type == "WorkerOutput" {
                    if let Ok(payload) = serde_json::from_str::<WorkerOutputPayload>(&event.payload) {
                        print!("{}", payload.content); 
                    }
                }
            }
        });

        loop {
            let missions = orchestrator.get_missions().await?;
            let mut pending_task = None;
            let mut all_done = true;

            for m in missions {
                for t in m.tasks {
                    if t.status == TaskStatus::Pending {
                        let w_name = t.assigned_worker.clone().unwrap_or_else(|| "Headless-Bot".to_string());
                        pending_task = Some((m.id, t.id, w_name));
                        all_done = false;
                        break;
                    }
                    if t.status == TaskStatus::Running {
                        all_done = false; 
                    }
                    if t.status == TaskStatus::Failure {
                        println!("❌ Task Failed: {}", t.name);
                        return Ok(());
                    }
                }
                if pending_task.is_some() { break; }
            }

            if let Some((mid, tid, worker_name)) = pending_task {
                let w_lower = worker_name.to_lowercase();
                let worker: Box<dyn ifcl_core::Worker> = if w_lower.contains("gemini") {
                    Box::new(AiGenericWorker::new(worker_name.clone(), WorkerRole::Coder, Box::new(AiCliAgent::new("gemini".to_string(), None))))
                } else if w_lower.contains("claude") {
                    Box::new(AiGenericWorker::new(worker_name.clone(), WorkerRole::Coder, Box::new(AiCliAgent::new("claude".to_string(), None))))
                } else if w_lower.contains("opencode") {
                    Box::new(AiGenericWorker::new(worker_name.clone(), WorkerRole::Coder, Box::new(AiCliAgent::new("opencode".to_string(), None))))
                } else {
                    Box::new(CliWorker::new(&worker_name, WorkerRole::Coder))
                };

                match orchestrator.execute_task(Arc::clone(&bus), mid, tid, worker.as_ref()).await {
                    Ok(out) => println!("✅ Task Success:\n{}", out),
                    Err(e) => println!("❌ Task Execution Failed: {}", e),
                }
            } else if all_done {
                println!("✨ All tasks completed successfully.");
                break;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        return Ok(());
    }

    // 3. Setup Terminal (GUI Mode)
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    let wizard = {
        let mut w = SetupWizard::new();
        if let Some(g) = _args.goal { w.goal = g; }
        if let Some(c) = _args.max_coins { w.budget_coins = c; }
        if let Some(ws) = _args.workspace { w.workspace_path = ws; }
        w
    };

    let available_groups = MarketplaceLoader::load_groups("marketplace/groups");
    let state = Arc::new(Mutex::new(AppState::new(available_groups, wizard)));

    let app = Arc::new(App::new(
        Arc::clone(&state),
        Arc::clone(&bus),
        Arc::clone(&store),
        Arc::clone(&orchestrator),
        Arc::clone(&learning_manager),
    ));
    
    // 4. Load Marketplace items (Async)
    let m_bus: Arc<dyn ifcl_core::EventBus> = Arc::clone(&bus);
    let s_c_marketplace = Arc::clone(&state);
    tokio::spawn(async move {
        let _ = std::fs::create_dir_all("marketplace/workers");
        let _ = std::fs::create_dir_all("marketplace/missions");

        for worker in MarketplaceLoader::load_workers("marketplace/workers") {
            let m_bus_w = Arc::clone(&m_bus);
            let s_c = Arc::clone(&s_c_marketplace);
            tokio::spawn(async move {
                let sid = s_c.lock().unwrap().current_session_id.unwrap_or_default();
                let _ = m_bus_w.publish(Event {
                    id: Uuid::new_v4(),
                    session_id: sid,
                    trace_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    worker_id: "system".to_string(),
                    event_type: "WorkerJoined".to_string(),
                    payload: serde_json::to_string(&worker).unwrap(),
                }).await;
            });
        }

        for mission in MarketplaceLoader::load_missions("marketplace/missions") {
            let m_bus_m: Arc<dyn ifcl_core::EventBus> = Arc::clone(&m_bus);
            let s_c = Arc::clone(&s_c_marketplace);
            tokio::spawn(async move {
                let sid = s_c.lock().unwrap().current_session_id.unwrap_or_default();
                let _ = m_bus_m.publish(Event {
                    id: Uuid::new_v4(),
                    session_id: sid,
                    trace_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    worker_id: "system".to_string(),
                    event_type: "MissionCreated".to_string(),
                    payload: serde_json::to_string(&mission).unwrap(),
                }).await;
            });
        }
    });

    let app_event = Arc::clone(&app);
    let bus_sub: Arc<dyn ifcl_core::EventBus> = Arc::clone(&bus);
    tokio::spawn(async move {
        let mut sub = bus_sub.subscribe();
        while let Ok(event) = sub.recv().await {
            app_event.process_event(event).await;
        }
    });

    // 5. Background Task Runner (The Real Loop Engine)
    let bus_runner: Arc<dyn ifcl_core::EventBus> = Arc::clone(&bus);
    let state_runner = Arc::clone(&state);
    let orch_runner = Arc::clone(&orchestrator);
    tokio::spawn(async move {
        loop {
            let target = {
                let s = state_runner.lock().unwrap();
                if s.status != LoopStatus::Running { 
                    None 
                } else {
                    let mut found = None;
                    for mission in &s.missions {
                        for task in &mission.tasks {
                            if task.status == TaskStatus::Pending {
                                let worker_name = task.assigned_worker.clone().unwrap_or_else(|| "Loop-Bot".to_string());
                                found = Some((mission.id, task.id, worker_name));
                                break;
                            }
                        }
                        if found.is_some() { break; }
                    }
                    found
                }
            };

            if let Some((mid, tid, worker_name)) = target {
                let bus_exec = Arc::clone(&bus_runner);
                let orch_exec = Arc::clone(&orch_runner);
                let w_lower = worker_name.to_lowercase();
                
                let worker: Box<dyn ifcl_core::Worker> = if w_lower.contains("gemini") {
                    Box::new(AiGenericWorker::new(worker_name.clone(), WorkerRole::Coder, Box::new(AiCliAgent::new("gemini".to_string(), None))))
                } else if w_lower.contains("claude") {
                    Box::new(AiGenericWorker::new(worker_name.clone(), WorkerRole::Coder, Box::new(AiCliAgent::new("claude".to_string(), None))))
                } else if w_lower.contains("opencode") {
                    Box::new(AiGenericWorker::new(worker_name.clone(), WorkerRole::Coder, Box::new(AiCliAgent::new("opencode".to_string(), None))))
                } else {
                    Box::new(CliWorker::new(&worker_name, WorkerRole::Coder))
                };
                
                let _ = orch_exec.execute_task(bus_exec.clone(), mid, tid, worker.as_ref()).await;
                
                let all_done = {
                    let s = state_runner.lock().unwrap();
                    !s.missions.is_empty() && s.missions.iter().all(|m| 
                        m.tasks.iter().all(|t| t.status == TaskStatus::Success || t.status == TaskStatus::Failure)
                    )
                };
                
                if all_done {
                    let session_id = state_runner.lock().unwrap().current_session_id.unwrap_or_default();
                    let _ = bus_exec.publish(Event {
                        id: Uuid::new_v4(),
                        session_id,
                        trace_id: Uuid::new_v4(),
                        timestamp: Utc::now(),
                        worker_id: "system".to_string(),
                        event_type: "GoalCompleted".to_string(),
                        payload: "All missions completed".to_string(),
                    }).await;
                }
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
        }
    });

    // 6. TUI Rendering Loop
    loop {
        terminal.draw(|f| {
            if let Ok(mut s) = state.lock() {
                ui::draw(f, &mut s);
            }
        })?;

        if let Ok(mut s) = state.lock() {
            s.frame_count = s.frame_count.wrapping_add(1);
        }

        if event::poll(std::time::Duration::from_millis(50))? {
            if let CEvent::Key(key) = event::read()? {
                if app.handle_input(key.code).await {
                    break;
                }
            }
        }
    }
    
    crossterm::terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::style::ResetColor
    )?;
    terminal.show_cursor()?;
    
    Ok(())
}
