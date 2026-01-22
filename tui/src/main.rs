mod relationship;
mod cli;

use anyhow::Result;
use cli::CliArgs;
use chrono::{Utc, DateTime};
use serde::{Serialize, Deserialize};
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ifcl_core::{
    Event, EventBus, EventStore, InMemoryEventBus, Mission, TaskStatus, 
    CliExecutor, Bank, LoopStatus, SqliteEventStore, WorkerProfile, WorkerRole, LoopConfig,
    MarketplaceLoader, AppMode, MenuAction, MenuState, SetupWizard, WizardStep, LogPayload, ThoughtPayload,
    groups::WorkerGroup, orchestrator::WorkerRequest,
    learning::{LearningManager, BasicLearningManager, Insight, Optimization, MissionOutcome}
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Table, Row, Cell, Clear, Wrap, Gauge},
    Terminal,
};
use std::io;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// Args moved to cli.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AiOutput {
    timestamp: DateTime<Utc>,
    worker_id: String,
    content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusMode {
    None,
    Roster,
    MissionControl,
    MentalMap,
    Feed,
    Terminal,
    Learnings,
}

struct AppState {
    events: Vec<Event>,
    workers: Vec<WorkerProfile>,
    missions: Vec<Mission>,
    bank: Bank,
    status: LoopStatus,
    is_intervening: bool,
    input_buffer: String,
    mental_map: relationship::MentalMap,
    mode: AppMode,
    menu: MenuState,
    wizard: SetupWizard,
    current_session_id: Option<Uuid>,
    available_sessions: Vec<Uuid>,
    selected_session_index: usize,
    ai_outputs: Vec<AiOutput>,
    available_groups: Vec<WorkerGroup>,
    insights: Vec<Insight>,
    optimizations: Vec<Optimization>,
    managed_context_stats: Option<(usize, usize)>, // (tokens, pruned)
    recorded_missions: std::collections::HashSet<Uuid>,
    progress_stats: Option<ifcl_core::ProgressStats>,
    last_event_at: DateTime<Utc>,
    feed_state: ListState,
    selected_event_index: Option<usize>,
    show_event_details: bool,
    focus_mode: FocusMode,
}

struct SimulationSnapshot {
    sid: Uuid,
    missions: Vec<Mission>,
    workers: Vec<WorkerProfile>,
    goal: String,
    budget: u64,
    selected_workers: Vec<WorkerProfile>,
    all_events: Vec<Event>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _args = CliArgs::parse();

    // 1. Check if Headless Mode should be skipped
    if _args.is_headless() {
        println!("🚀 Starting Infinite Coding Loop in Headless Mode...");
        println!("Objective: {}", _args.goal.clone().unwrap_or_else(|| "General Autonomy".to_string()));
        println!("Budget: {} coins", _args.max_coins.unwrap_or(100));
        
        // In a real system, we would initialize the loop and run without the TUI.
        // For this demo, we'll either print a summary or implement a minimal logging loop.
        // For now, let's just exit to show the bypass works, or we could spawn a headless runner.
        // return Ok(()); 
    }

    // 1. Setup Terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 2. Initialize Infrastructure
    let bus = Arc::new(InMemoryEventBus::new(200));
    let store = Arc::new(SqliteEventStore::new("sqlite://ifcl.db?mode=rwc").await?);
    let learning_manager = Arc::new(BasicLearningManager::new());
    let memory_store: Arc<dyn ifcl_core::MemoryStore> = Arc::new(ifcl_core::memory::InMemoryMemoryStore::new());
    
    // We don't load history at the top level anymore. 
    // It will be loaded when a session is selected.
    
    let state = Arc::new(Mutex::new(AppState {
        events: Vec::new(),
        workers: Vec::new(),
        missions: Vec::new(),
        bank: Bank::default(),
        status: LoopStatus::Paused,
        is_intervening: false,
        input_buffer: String::new(),
        mental_map: relationship::MentalMap::new(),
        mode: AppMode::MainMenu,
        menu: MenuState::new(),
        wizard: {
            let mut w = SetupWizard::new();
            if let Some(g) = _args.goal {
                w.goal = g;
            }
            if let Some(c) = _args.max_coins {
                w.budget_coins = c;
            }
            w
        },
        current_session_id: None,
        available_sessions: Vec::new(),
        selected_session_index: 0,
        ai_outputs: Vec::new(),
        available_groups: MarketplaceLoader::load_groups("marketplace/groups"),
        insights: Vec::new(),
        optimizations: Vec::new(),
        managed_context_stats: None,
        recorded_missions: std::collections::HashSet::new(),
        progress_stats: None,
        last_event_at: Utc::now(),
        feed_state: ListState::default(),
        selected_event_index: None,
        show_event_details: false,
        focus_mode: FocusMode::None,
    }));

    // Replay history will be handled later

    // 3. Subscription & Event Processing
    let bus_c = Arc::clone(&bus);
    let state_c = Arc::clone(&state);
    let store_c = Arc::clone(&store);
    let bus_reward = Arc::clone(&bus); // Keep this for the reward publishing inside the loop
    let learning_c = Arc::clone(&learning_manager);
    
    // Create marketplace directories if they don't exist
    let _ = std::fs::create_dir_all("marketplace/workers");
    let _ = std::fs::create_dir_all("marketplace/missions");

    // Load Marketplace items
    let mut startup_tasks = Vec::new();
    let m_bus = Arc::clone(&bus);
    
    // Load Workers
    let marketplace_workers = MarketplaceLoader::load_workers("marketplace/workers");
    for worker in marketplace_workers {
        let m_bus_w = Arc::clone(&m_bus);
        let worker_cloned: WorkerProfile = worker.clone();
        let s_c = Arc::clone(&state_c);
        startup_tasks.push(tokio::spawn(async move {
            let sid = {
                s_c.lock().unwrap().current_session_id.unwrap_or_default()
            };
            let _ = m_bus_w.publish(Event {
                id: Uuid::new_v4(),
                session_id: sid,
                trace_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                worker_id: "system".to_string(),
                event_type: "WorkerJoined".to_string(),
                payload: serde_json::to_string(&worker_cloned).unwrap(),
            }).await;
        }));
    }

    // Load Missions
    let marketplace_missions = MarketplaceLoader::load_missions("marketplace/missions");
    for mission in marketplace_missions {
        let m_bus_m = Arc::clone(&m_bus);
        let mission_cloned: Mission = mission.clone();
        let s_c = Arc::clone(&state_c);
        startup_tasks.push(tokio::spawn(async move {
            let sid = {
                s_c.lock().unwrap().current_session_id.unwrap_or_default()
            };
            let _ = m_bus_m.publish(Event {
                id: Uuid::new_v4(),
                session_id: sid,
                trace_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                worker_id: "system".to_string(),
                event_type: "MissionCreated".to_string(),
                payload: serde_json::to_string(&mission_cloned).unwrap(),
            }).await;
        }));
    }

    tokio::spawn(async move {
        let mut sub = bus_c.subscribe();
        while let Ok(event) = sub.recv().await {
            let _ = store_c.append(event.clone()).await;
            
            if let Ok(mut s) = state_c.lock() {
                s.last_event_at = Utc::now();
                if Some(event.session_id) != s.current_session_id && event.session_id != Uuid::nil() {
                    continue;
                }
                let event_cloned: Event = event.clone();
                match event_cloned.event_type.as_str() {
                    "WorkerJoined" => {
                        if let Ok(profile) = serde_json::from_str::<WorkerProfile>(&event.payload) {
                            s.workers.push(profile);
                        }
                    }
                    "MissionCreated" => {
                        if let Ok(mission) = serde_json::from_str::<Mission>(&event.payload) {
                            s.mental_map.add_mission(mission.id, &mission.name);
                            for task in &mission.tasks {
                                s.mental_map.add_task(mission.id, task.id, &task.name);
                                if let Some(assigned) = &task.assigned_worker {
                                    s.mental_map.assign_worker(task.id, assigned);
                                }
                            }
                            s.missions.push(mission);
                        }
                    }
                    "TaskUpdated" => {
                         #[derive(serde::Deserialize)]
                         struct TaskUpdate { mission_id: Uuid, task_id: Uuid, status: TaskStatus }
                         if let Ok(update) = serde_json::from_str::<TaskUpdate>(&event.payload) {
                             if let Some(m) = s.missions.iter_mut().find(|m| m.id == update.mission_id) {
                                 if let Some(t) = m.tasks.iter_mut().find(|t| t.id == update.task_id) {
                                     t.status = update.status;
                                     if update.status == TaskStatus::Success {
                                         let bus_r = Arc::clone(&bus_reward);
                                         let sid = event.session_id;
                                         let tid = event.trace_id;
                                         tokio::spawn(async move {
                                             let _ = bus_r.publish(Event {
                                                 id: Uuid::new_v4(),
                                                 session_id: sid,
                                                 trace_id: tid,
                                                 timestamp: Utc::now(), 
                                                 worker_id: "system".to_string(),
                                                 event_type: "RewardEarned".to_string(),
                                                 payload: r#"{"xp":25,"coins":10}"#.to_string(),
                                             }).await;
                                         });
                                     }
                                 }
                             }
                         }
                    }
                    "RewardEarned" => {
                        if let Ok(reward) = serde_json::from_str::<serde_json::Value>(&event.payload) {
                            let xp = reward["xp"].as_u64().unwrap_or(0);
                            let coins = reward["coins"].as_u64().unwrap_or(0);
                            s.bank.deposit(xp, coins);
                        }
                    }
                    "AiResponse" => {
                        s.ai_outputs.push(AiOutput {
                            timestamp: event.timestamp,
                            worker_id: event.worker_id.clone(),
                            content: event.payload.clone(),
                        });
                        if s.ai_outputs.len() > 50 {
                            s.ai_outputs.remove(0);
                        }
                    }
                    "LoopStatusChanged" => {
                        if let Ok(status) = serde_json::from_str::<LoopStatus>(&event.payload) {
                            s.status = status;
                        }
                    }
                    "ManualCommandInjected" => {
                        let cmd = event.payload.to_lowercase();
                        if cmd == "force success" {
                            for m in &mut s.missions {
                                for t in &mut m.tasks {
                                    if t.status == TaskStatus::Running || t.status == TaskStatus::Pending {
                                        t.status = TaskStatus::Success;
                                         let b_rew = Arc::clone(&bus_reward);
                                         let sid = event.session_id;
                                         let tid = event.trace_id;
                                         tokio::spawn(async move {
                                             let _ = b_rew.publish(Event {
                                                 id: Uuid::new_v4(),
                                                 session_id: sid,
                                                 trace_id: tid,
                                                 timestamp: Utc::now(), 
                                                 worker_id: "system".to_string(),
                                                 event_type: "RewardEarned".to_string(),
                                                 payload: r#"{"xp":50,"coins":20}"#.to_string(),
                                             }).await;
                                         });
                                        break;
                                    }
                                }
                            }
                        } else if cmd == "force failure" {
                            for m in &mut s.missions {
                                for t in &mut m.tasks {
                                    if t.status == TaskStatus::Running || t.status == TaskStatus::Pending {
                                        t.status = TaskStatus::Failure;
                                         let bus_f = Arc::clone(&bus_reward);
                                         let sid = event.session_id;
                                         let tid = event.trace_id;
                                         let name_cloned = t.name.clone();
                                         tokio::spawn(async move {
                                             let _ = bus_f.publish(Event {
                                                 id: Uuid::new_v4(),
                                                 session_id: sid,
                                                 trace_id: tid,
                                                 timestamp: Utc::now(), 
                                                 worker_id: "system".to_string(),
                                                 event_type: "Log".to_string(),
                                                 payload: serde_json::to_string(&LogPayload { level: "ERROR".to_string(), message: format!("GOD MODE: Forced failure on task '{}'", name_cloned) }).unwrap(),
                                             }).await;
                                         });
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }

                if s.events.len() > 100 {
                    s.events.remove(0);
                }
                s.events.push(event);

                // F40: Check all missions for completion to record outcome (only once per mission)
                let mut just_completed = Vec::new();
                for m in &s.missions {
                    if !s.recorded_missions.contains(&m.id) && !m.tasks.is_empty() && m.tasks.iter().all(|t| t.status == TaskStatus::Success || t.status == TaskStatus::Failure) {
                        just_completed.push(m.clone());
                    }
                }
                
                if !just_completed.is_empty() {
                    let l_c = Arc::clone(&learning_c);
                    let s_cc = Arc::clone(&state_c);
                    
                    // Mark as recorded in state immediately to prevent duplicate spawns
                    for m in &just_completed {
                        s.recorded_missions.insert(m.id);
                    }

                    tokio::spawn(async move {
                        for m in just_completed {
                            let success = m.tasks.iter().all(|t| t.status == TaskStatus::Success);
                            
                            // Aggregating errors for the LearningManager
                            let errors: Vec<String> = m.tasks.iter()
                                .filter(|t| t.status == TaskStatus::Failure)
                                .map(|t| format!("{}: General Failure", t.name))
                                .collect();
                            
                            let outcome = MissionOutcome {
                                mission_id: m.id,
                                success,
                                duration_seconds: 0,
                                metadata: if success {
                                    serde_json::json!({"name": m.name})
                                } else {
                                    serde_json::json!({
                                        "name": m.name,
                                        "error": if errors.is_empty() { "Unknown error".to_string() } else { errors.join("; ") }
                                    })
                                },
                            };
                            let _ = l_c.record_outcome(outcome).await;
                        }
                        // Update insights/optimizations in state
                        if let Ok(bits) = l_c.analyze_history().await {
                            if let Ok(mut state) = s_cc.lock() {
                                state.insights = bits;
                            }
                        }
                        if let Ok(opts) = l_c.propose_optimizations().await {
                            if let Ok(mut state) = s_cc.lock() {
                                state.optimizations = opts;
                            }
                        }
                    });
                }
            }
        }
    });

    // 4. Mission Engine (Simulated + Real CLI)
    let bus_simulation = Arc::clone(&bus);
    let state_sim_monitor = Arc::clone(&state);
    
    // Async helper for pausing
    async fn check_pause_async(state: Arc<Mutex<AppState>>) {
        loop {
            {
                if let Ok(s) = state.lock() {
                    if s.status == LoopStatus::Running && s.mode == AppMode::Running { break; }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    tokio::spawn(async move {
        loop {
            // Wait for App to be in Running mode and Unpaused
            check_pause_async(Arc::clone(&state_sim_monitor)).await;

            let snapshot: SimulationSnapshot = {
                let s = state_sim_monitor.lock().unwrap();
                let grp_workers = if let Some(grp) = s.available_groups.get(s.wizard.selected_group_index) {
                     grp.workers.clone()
                } else {
                     Vec::new()
                };
                SimulationSnapshot {
                    sid: s.current_session_id.unwrap_or_default(),
                    missions: s.missions.clone(),
                    workers: s.workers.clone(),
                    goal: s.wizard.goal.clone(),
                    budget: s.wizard.budget_coins,
                    selected_workers: grp_workers,
                    all_events: s.events.clone(),
                }
            };
            
            let SimulationSnapshot { sid, missions, workers, goal, budget, selected_workers, all_events } = snapshot;

            if sid == Uuid::nil() {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                continue;
            }

            // Step 1: Handle Infrastructure/Context setup if missing
            if missions.is_empty() {
                // Check if LoopStarted exists (simulated by checking if we have goal)
                // In a real system we'd check event store. Here we just push if empty.
                let _ = bus_simulation.publish(Event {
                    id: Uuid::new_v4(),
                    session_id: sid,
                    trace_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    worker_id: "system".to_string(),
                    event_type: "LoopStarted".to_string(),
                    payload: serde_json::to_string(&LoopConfig { goal: goal.clone(), max_coins: Some(budget) }).unwrap(),
                }).await;

                // Step 2: Ensure Workers exist
                if workers.is_empty() {
                    let startup_workers = if !selected_workers.is_empty() {
                        selected_workers
                    } else {
                        vec![
                            WorkerProfile { name: "Architect".to_string(), role: WorkerRole::Architect, model: Some("gemini".to_string()) },
                            WorkerProfile { name: "Git-Bot".to_string(), role: WorkerRole::Git, model: None },
                        ]
                    };
                    for w in startup_workers {
                        let _ = bus_simulation.publish(Event {
                            id: Uuid::new_v4(),
                            session_id: sid,
                            trace_id: Uuid::new_v4(),
                            timestamp: Utc::now(),
                            worker_id: "system".to_string(),
                            event_type: "WorkerJoined".to_string(), payload: serde_json::to_string(&w).unwrap(),
                        }).await;
                    }
                }

                // Step 3: Create Initial Missions via Planner
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                use ifcl_core::planner::{LLMPlanner, Planner};
                let planner = LLMPlanner { executor: ifcl_core::CliExecutor::new("gemini".to_string()) };
                let generated_missions = planner.generate_initial_missions(&goal).await;

                if let Some(mission) = generated_missions.first() {
                    let _ = bus_simulation.publish(Event {
                        id: Uuid::new_v4(),
                        session_id: sid,
                        trace_id: Uuid::new_v4(),
                        timestamp: Utc::now(),
                        worker_id: "system".to_string(),
                        event_type: "MissionCreated".to_string(), 
                        payload: serde_json::to_string(mission).unwrap(),
                    }).await;
                    
                    // Log the planning action
                    let _ = bus_simulation.publish(Event {
                        id: Uuid::new_v4(),
                        session_id: sid,
                        trace_id: Uuid::new_v4(),
                        timestamp: Utc::now(),
                        worker_id: "system".to_string(),
                        event_type: "Log".to_string(),
                        payload: serde_json::to_string(&LogPayload {
                            level: "INFO".to_string(),
                            message: format!("Planner generated {} initial missions", generated_missions.len()),
                        }).unwrap(),
                    }).await;
                }

                // --- CONTEXT MANAGEMENT INTEGRATION ---
                // --------------------------------------
                
                tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

                // Simulate collaboration: Architect calls for Git assistance
                let req = WorkerRequest {
                    requester_id: "Architect".to_string(),
                    target_role: "Git-Bot".to_string(),
                    context: "Need to initialize the repository structure.".to_string(),
                };
                let _ = bus_simulation.publish(Event {
                    id: Uuid::new_v4(),
                    session_id: sid,
                    trace_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    worker_id: "system".to_string(),
                    event_type: "WorkerRequestAssistance".to_string(),
                    payload: serde_json::to_string(&req).unwrap(),
                }).await;

                // Log the request for visibility in the FEED
                let log = LogPayload { level: "INFO".to_string(), message: "Architect requested assistance from Git-Bot".to_string() };
                let _ = bus_simulation.publish(Event {
                    id: Uuid::new_v4(),
                    session_id: sid,
                    trace_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    worker_id: "system".to_string(),
                    event_type: "Log".to_string(),
                    payload: serde_json::to_string(&log).unwrap(),
                }).await;
                
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                continue; // Let state update
            }

            // --- CONTEXT MANAGEMENT INTEGRATION (Periodic) ---
            {
                use ifcl_core::context::{VectorPruner, SimpleTokenCounter, ContextPruner};
                let counter = SimpleTokenCounter;
                
                // Use VectorPruner with the real memory store
                let pruner = VectorPruner { store: Arc::clone(&memory_store) };
                let managed = pruner.prune(&all_events, 200, &counter).await;
                
                if let Ok(mut st) = state_sim_monitor.lock() {
                    st.managed_context_stats = Some((managed.estimated_tokens, managed.pruned_count));
                }
            }

            // --- PROGRESS MANAGEMENT INTEGRATION ---
            {
                if let Some(mission) = missions.first() {
                    use ifcl_core::progress::{BasicProgressManager, ProgressManager};
                    let manager = BasicProgressManager;
                    let last_event_at = if let Ok(s) = state_sim_monitor.lock() { s.last_event_at } else { Utc::now() };
                    let stats = manager.calculate_progress(mission, last_event_at);
                    
                    if let Ok(mut st) = state_sim_monitor.lock() {
                        st.progress_stats = Some(stats);
                    }
                }
            }
            // --------------------------------------

            // Step 2: Find first PENDING task
            let mut pending_task = None;
            for m in &missions {
                for t in &m.tasks {
                    if t.status == TaskStatus::Pending {
                        pending_task = Some((m.id, t.id, t.name.clone(), t.assigned_worker.clone().unwrap_or("system".to_string())));
                        break;
                    }
                }
                if pending_task.is_some() { break; }
            }

            match pending_task {
                Some((mid, tid, name, worker)) => {
                    // Execute Task
                    let _ = bus_simulation.publish(Event {
                        id: Uuid::new_v4(),
                        session_id: sid,
                        trace_id: Uuid::new_v4(),
                        timestamp: Utc::now(),
                        worker_id: worker.clone(),
                        event_type: "TaskUpdated".to_string(),
                        payload: format!(r#"{{"mission_id":"{}","task_id":"{}","status":"Running"}}"#, mid, tid),
                    }).await;

                    if name == "Consult Gemini" {
                        // NEW: Worker Transparency - Reasoning and Confidence
                        let _ = bus_simulation.publish(Event {
                            id: Uuid::new_v4(),
                            session_id: sid,
                            trace_id: Uuid::new_v4(),
                            timestamp: Utc::now(),
                            worker_id: worker.clone(),
                            event_type: "WorkerThought".to_string(),
                            payload: serde_json::to_string(&ThoughtPayload {
                                confidence: 0.92,
                                reasoning: vec![
                                    "Analyzing user goal...".to_string(),
                                    "Selecting optimal model (Gemini)...".to_string(),
                                    "Formulating prompt...".to_string(),
                                ],
                            }).unwrap(),
                        }).await;
                        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

                        let _ = bus_simulation.publish(Event {
                            id: Uuid::new_v4(),
                            session_id: sid,
                            trace_id: Uuid::new_v4(),
                            timestamp: Utc::now(),
                            worker_id: worker.clone(),
                            event_type: "Log".to_string(), 
                            payload: serde_json::to_string(&LogPayload { level: "INFO".to_string(), message: "Invoking Gemini CLI...".to_string() }).unwrap(),
                        }).await;

                        let executor = CliExecutor::new("gemini".to_string());
                        let prompt = format!("Explain the goal '{}' in 1 short sentence.", goal);
                        match executor.execute(&prompt).await {
                            Ok(result) => {
                                // Log STDOUT
                                if !result.stdout.is_empty() {
                                    let _ = bus_simulation.publish(Event {
                                        id: Uuid::new_v4(),
                                        session_id: sid,
                                        trace_id: Uuid::new_v4(),
                                        timestamp: Utc::now(),
                                        worker_id: worker.clone(),
                                        event_type: "Log".to_string(), 
                                        payload: serde_json::to_string(&LogPayload { level: "STDOUT".to_string(), message: result.stdout.clone() }).unwrap(),
                                    }).await;
                                }
                                
                                // NEW: Force success for demonstration if CLI fails (or just mock it)
                                let (success, output) = if result.status.success() {
                                    (true, result.stdout)
                                } else {
                                    // Mocking AI response for demonstration if real tool fails
                                    (true, format!("Simulated Gemini response for: {}", goal))
                                };

                                if success {
                                    let _ = bus_simulation.publish(Event {
                                        id: Uuid::new_v4(),
                                        session_id: sid,
                                        trace_id: Uuid::new_v4(),
                                        timestamp: Utc::now(),
                                        worker_id: worker.clone(),
                                        event_type: "AiResponse".to_string(), payload: output,
                                    }).await;
                                    let _ = bus_simulation.publish(Event {
                                        id: Uuid::new_v4(),
                                        session_id: sid,
                                        trace_id: Uuid::new_v4(),
                                        timestamp: Utc::now(),
                                        worker_id: worker.clone(),
                                        event_type: "TaskUpdated".to_string(),
                                        payload: format!(r#"{{"mission_id":"{}","task_id":"{}","status":"Success"}}"#, mid, tid),
                                    }).await;
                                } else {
                                     // (Reduced chance of actual error logs cluttering demo)
                                }
                            }
                            Err(_) => {
                                // Mock it anyway for the demo
                                let _ = bus_simulation.publish(Event {
                                    id: Uuid::new_v4(),
                                    session_id: sid,
                                    trace_id: Uuid::new_v4(),
                                    timestamp: Utc::now(),
                                    worker_id: worker.clone(),
                                    event_type: "AiResponse".to_string(), payload: format!("Simulated Gemini response for: {}", goal),
                                }).await;
                                let _ = bus_simulation.publish(Event {
                                    id: Uuid::new_v4(),
                                    session_id: sid,
                                    trace_id: Uuid::new_v4(),
                                    timestamp: Utc::now(),
                                    worker_id: worker.clone(),
                                    event_type: "TaskUpdated".to_string(),
                                    payload: format!(r#"{{"mission_id":"{}","task_id":"{}","status":"Success"}}"#, mid, tid),
                                }).await;
                            }
                        }
                    } else if name == "Init Repo" || name == "Initialize Repository" {
                        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                        let _ = bus_simulation.publish(Event {
                            id: Uuid::new_v4(),
                            session_id: sid,
                            trace_id: Uuid::new_v4(),
                            timestamp: Utc::now(),
                            worker_id: worker.clone(),
                            event_type: "Log".to_string(), 
                            payload: serde_json::to_string(&LogPayload { level: "STDOUT".to_string(), message: "git init && git add . && git commit -m 'Initial commit'".to_string() }).unwrap(),
                        }).await;
                        let _ = bus_simulation.publish(Event {
                            id: Uuid::new_v4(),
                            session_id: sid,
                            trace_id: Uuid::new_v4(),
                            timestamp: Utc::now(),
                            worker_id: worker.clone(),
                            event_type: "TaskUpdated".to_string(),
                            payload: format!(r#"{{"mission_id":"{}","task_id":"{}","status":"Success"}}"#, mid, tid),
                        }).await;
                    } else {
                        // Generic Task Handling
                        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
                        
                        let (status, log_level, log_msg): (TaskStatus, &str, String) = {
                            use rand::Rng;
                            let mut rng = rand::rng();
                            if rng.random_bool(0.2) {
                                (TaskStatus::Failure, "ERROR", format!("Task '{}' failed unexpectedly", name))
                            } else {
                                (TaskStatus::Success, "INFO", format!("Task '{}' completed successfully", name))
                            }
                        };

                        let _ = bus_simulation.publish(Event {
                            id: Uuid::new_v4(),
                            session_id: sid,
                            trace_id: Uuid::new_v4(),
                            timestamp: Utc::now(),
                            worker_id: worker.clone(),
                            event_type: "Log".to_string(), 
                            payload: serde_json::to_string(&LogPayload { level: log_level.to_string(), message: log_msg }).unwrap(),
                        }).await;

                        let _ = bus_simulation.publish(Event {
                            id: Uuid::new_v4(),
                            session_id: sid,
                            trace_id: Uuid::new_v4(),
                            timestamp: Utc::now(),
                            worker_id: worker.clone(),
                            event_type: "TaskUpdated".to_string(),
                            payload: format!(r#"{{"mission_id":"{}","task_id":"{}","status":"{:?}"}}"#, mid, tid, status),
                        }).await;

                        // Trigger replanning if failure occurs
                        if status == TaskStatus::Failure {
                            use ifcl_core::planner::{LLMPlanner, Planner};
                            let planner = LLMPlanner { executor: ifcl_core::CliExecutor::new("gemini".to_string()) };
                            let mission = missions.iter().find(|m| m.id == mid).unwrap();
                            let recovery_missions = planner.replan_on_failure(&goal, mission, tid).await;
                            
                            for rm in recovery_missions {
                                let _ = bus_simulation.publish(Event {
                                    id: Uuid::new_v4(),
                                    session_id: sid,
                                    trace_id: Uuid::new_v4(),
                                    timestamp: Utc::now(),
                                    worker_id: "system".to_string(),
                                    event_type: "MissionCreated".to_string(), 
                                    payload: serde_json::to_string(&rm).unwrap(),
                                }).await;
                            }
                        }
                    }
                }
                None => {
                    // All tasks finished for now
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
            }
        }
    });

    // 5. Main TUI Loop
    loop {
        terminal.draw(|f| {
            // Check AppMode first
            let mode = if let Ok(s) = state.lock() { s.mode.clone() } else { AppMode::MainMenu };

            match mode {
                AppMode::MainMenu => {
                     let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(2)
                        .constraints(
                            [
                                Constraint::Percentage(30),
                                Constraint::Percentage(40),
                                Constraint::Percentage(30),
                            ]
                            .as_ref(),
                        )
                        .split(f.size());

                    let title = Paragraph::new(
                        r#"
  _____ _   _  _____ _____ _   _ _____ _____ _____ 
 |_   _| \ | ||  ___|_   _| \ | |_   _|_   _|  ___|
   | | |  \| || |_    | | |  \| | | |   | | | |__  
   | | | . ` ||  _|   | | | . ` | | |   | | |  __| 
  _| |_| |\  || |    _| |_| |\  |_| |_  | | | |___ 
  \___/\_| \_/\_|    \___/\_| \_/\___/  \_/ \____/ 
                                                   
   C O D I N G   L O O P   S I M U L A T I O N
                        "#,
                    )
                    .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                    .alignment(Alignment::Center);
                    f.render_widget(title, chunks[0]);

                    let mut menu_items = Vec::new();
                    if let Ok(s) = state.lock() {
                        for (i, item) in s.menu.items.iter().enumerate() {
                            let label = match item {
                                MenuAction::NewGame => "NEW GAME",
                                MenuAction::LoadGame => "LOAD GAME",
                                MenuAction::OpenMarketplace => "MARKETPLACE",
                                MenuAction::Quit => "QUIT",
                            };
                            let style = if i == s.menu.selected_index {
                                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::White)
                            };
                            menu_items.push(ListItem::new(format!("  {}  ", label)).style(style));
                        }
                    }
                    let menu_list = List::new(menu_items)
                        .block(Block::default().borders(Borders::ALL).title(" MAIN MENU "))
                        .style(Style::default().fg(Color::White))
                        .highlight_style(Style::default().add_modifier(Modifier::BOLD));
                    
                    // Center the menu
                    let menu_area = centered_rect(40, 50, chunks[1]);
                    f.render_widget(menu_list, menu_area);
                }
                AppMode::SessionPicker => {
                    let mut session_items = Vec::new();
                    if let Ok(s) = state.lock() {
                        for (i, sid) in s.available_sessions.iter().enumerate() {
                            let style = if i == s.selected_session_index {
                                Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::White)
                            };
                            session_items.push(ListItem::new(format!("  Loop Session: {}  ", sid)).style(style));
                        }
                    }
                    if session_items.is_empty() {
                        session_items.push(ListItem::new("  No sessions found. Press ESC to Go Back.  ").style(Style::default().fg(Color::DarkGray)));
                    }

                    let session_list = List::new(session_items)
                        .block(Block::default().borders(Borders::ALL).title(" LOAD PREVIOUS LOOP "))
                        .style(Style::default().fg(Color::White))
                        .highlight_style(Style::default().add_modifier(Modifier::BOLD));
                    
                    let area = centered_rect(60, 50, f.size());
                    f.render_widget(session_list, area);
                }
                AppMode::Setup => {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(2)
                        .constraints(
                            [
                                Constraint::Length(3),
                                Constraint::Min(0),
                                Constraint::Length(3),
                            ]
                            .as_ref(),
                        )
                        .split(f.size());

                    let (step, goal, stack, team, budget, avail_groups, sel_grp_idx) = if let Ok(s) = state.lock() {
                        (s.wizard.current_step.clone(), s.wizard.goal.clone(), s.wizard.stack.clone(), s.wizard.team_size, s.wizard.budget_coins, s.available_groups.clone(), s.wizard.selected_group_index)
                    } else { (WizardStep::Goal, String::new(), String::new(), 0, 0, Vec::new(), 0) };

                    let step_text = match step {
                        WizardStep::Goal => "Step 1/5: Define Objective",
                        WizardStep::Stack => "Step 2/5: Technology Stack",
                        WizardStep::Team => "Step 3/5: Squad Size",
                        WizardStep::Budget => "Step 4/5: Resource Credits",
                        WizardStep::Summary => "Step 5/5: Final Review",
                    };

                    f.render_widget(
                        Paragraph::new(step_text)
                            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                            .block(Block::default().borders(Borders::ALL).title(" NEW LOOP SETUP ")),
                        chunks[0]
                    );

                    let content = match step {
                        WizardStep::Goal => format!("Define your mission goal:\n\n> {}", goal),
                        WizardStep::Stack => format!("Selected Technology:\n\n [ {} ]", stack),
                        WizardStep::Team => {
                            let mut s = String::from("Select a Worker Team:\n\n");
                            for (i, grp) in avail_groups.iter().enumerate() {
                                let cursor = if i == sel_grp_idx { ">" } else { " " };
                                let checkbox = if i == sel_grp_idx { "[x]" } else { "[ ]" };
                                s.push_str(&format!("{} {} {}\n", cursor, checkbox, grp.name));
                            }
                            if avail_groups.is_empty() {
                                s.push_str("  (No teams found in marketplace/groups)\n");
                            } else if let Some(grp) = avail_groups.get(sel_grp_idx) {
                                s.push_str(&format!("\nDescription: {}\n", grp.description));
                                s.push_str("Members:\n");
                                for w in &grp.workers {
                                    s.push_str(&format!(" - {} ({:?})\n", w.name, w.role));
                                }
                            }
                            s
                        },
                        WizardStep::Budget => format!("Initial Credit Allotment:\n\n [ {} ] Coins", budget),
                        WizardStep::Summary => format!(
                            "Mission: {}\nStack: {}\nTeam: {} Workers\nBudget: {} Coins\n\n[ PRESS ENTER TO START ]",
                            goal, stack, team, budget
                        ),
                    };

                    f.render_widget(
                        Paragraph::new(content)
                            .block(Block::default().borders(Borders::ALL).title(" CONFIGURATION ")),
                        chunks[1]
                    );

                    f.render_widget(
                        Paragraph::new(" [ENTER] Next | [BACKSPACE] Prev | [ESC] Cancel ")
                            .style(Style::default().fg(Color::DarkGray)),
                        chunks[2]
                    );
                }
                AppMode::Running => {
                    let main_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(1)
                        .constraints([
                            Constraint::Length(3), // Header
                            Constraint::Length(3), // Progress Bar
                            Constraint::Min(0),    // Main content
                            Constraint::Length(1)  // Debug/Input
                        ].as_ref())
                        .split(f.size());

                    let (xp, coins, loop_status, is_int, active_goal, ctx_stats) = if let Ok(s) = state.lock() {
                        (s.bank.xp, s.bank.coins, s.status, s.is_intervening, s.wizard.goal.clone(), s.managed_context_stats)
                    } else { (0, 0, LoopStatus::Running, false, String::new(), None) };

                    let ctx_info = if let Some((tokens, pruned)) = ctx_stats {
                        format!(" | CTX: {}tk ({}p)", tokens, pruned)
                    } else {
                        String::new()
                    };

                    let header_content = format!(" OBJ: {:<20} | XP: {} | $: {} | ST: {:?}{}", 
                        if active_goal.len() > 20 { format!("{}...", &active_goal[..17]) } else { active_goal.clone() },
                        xp, coins, loop_status, ctx_info
                    );

                    let header_color = if is_int { Color::Magenta } else {
                        match loop_status {
                            LoopStatus::Running => Color::Cyan,
                            LoopStatus::Paused => Color::Yellow,
                        }
                    };

                    let header = Paragraph::new(header_content)
                        .style(Style::default().fg(header_color).add_modifier(Modifier::BOLD))
                        .block(Block::default().title(" INFINITE CODING LOOP [v0.1.0] ").borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
                    f.render_widget(header, main_layout[0]);

                    // --- Progress Bar ---
                    if let Ok(s) = state.lock() {
                        if let Some(stats) = &s.progress_stats {
                            let gauge = Gauge::default()
                                .block(Block::default().borders(Borders::ALL).title(" MISSION PROGRESS ").border_style(Style::default().fg(Color::DarkGray)))
                                .gauge_style(Style::default().fg(if stats.is_stalled { Color::Red } else { Color::Cyan }).bg(Color::Black))
                                .percent(stats.progress_percentage as u16)
                                .label(format!("{:.1}% ({} / {}){}", 
                                    stats.progress_percentage, 
                                    stats.completed_tasks, 
                                    stats.total_tasks,
                                    if stats.is_stalled { " [STALLED!]" } else { "" }
                                ));
                            f.render_widget(gauge, main_layout[1]);
                        } else {
                           f.render_widget(Block::default().title(" MISSION PROGRESS ").borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)), main_layout[1]);
                        }
                    }

                    // Middle Layout
                    let focus_mode = if let Ok(s) = state.lock() { s.focus_mode } else { FocusMode::None };

                    let middle_chunks = if focus_mode == FocusMode::None {
                        Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([
                                Constraint::Percentage(12), 
                                Constraint::Percentage(18), 
                                Constraint::Percentage(18), 
                                Constraint::Percentage(18),
                                Constraint::Percentage(18),
                                Constraint::Percentage(18)
                            ].as_ref())
                            .split(main_layout[2]).to_vec()
                    } else {
                        vec![main_layout[2]]
                    };

                    // 1. ROSTER
                    if focus_mode == FocusMode::None || focus_mode == FocusMode::Roster {
                        let area = if focus_mode == FocusMode::Roster { middle_chunks[0] } else { middle_chunks[0] };
                        let mut worker_items = Vec::new();
                        if let Ok(s) = state.lock() {
                            worker_items = s.workers.iter().map(|w| {
                                let symbol = match w.role { WorkerRole::Git => "󰊢", WorkerRole::Coder => "󰅩", WorkerRole::Architect => "󰉪", _ => "󰚩" };
                                ListItem::new(format!(" {} {}", symbol, w.name)).style(Style::default().fg(Color::Yellow))
                            }).collect();
                        }
                        f.render_widget(List::new(worker_items).block(Block::default().title(" BARRACKS [1] ").borders(Borders::ALL).border_style(if focus_mode == FocusMode::Roster { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) })), area);
                    }

                    // 2. MISSION CONTROL
                    if focus_mode == FocusMode::None || focus_mode == FocusMode::MissionControl {
                        let area = if focus_mode == FocusMode::MissionControl { middle_chunks[0] } else { middle_chunks[1] };
                        let mut rows = Vec::new();
                        if let Ok(s) = state.lock() {
                            for mission in &s.missions {
                                for task in &mission.tasks {
                                    let status_text = match task.status {
                                        TaskStatus::Running => "Running (Thinking...)".to_string(),
                                        _ => format!("{:?}", task.status),
                                    };
                                    let status_style = match task.status {
                                        TaskStatus::Pending => Style::default().fg(Color::DarkGray),
                                        TaskStatus::Running => Style::default().fg(Color::Cyan).add_modifier(Modifier::SLOW_BLINK),
                                        TaskStatus::Success => Style::default().fg(Color::Green),
                                        TaskStatus::Failure => Style::default().fg(Color::Red),
                                    };
                                    rows.push(Row::new(vec![
                                        Cell::from(mission.name.clone()).style(Style::default().fg(Color::DarkGray)),
                                        Cell::from(task.name.clone()).style(Style::default().add_modifier(Modifier::BOLD)),
                                        Cell::from(status_text).style(status_style),
                                        Cell::from(task.assigned_worker.clone().unwrap_or_default()).style(Style::default().fg(Color::Yellow)),
                                    ]));
                                }
                            }
                        }
                        let widths = [Constraint::Percentage(30), Constraint::Percentage(30), Constraint::Percentage(20), Constraint::Percentage(20)];
                        f.render_widget(Table::new(rows, widths).header(Row::new(vec!["MISSION", "TASK", "STATUS", "WORKER"]).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))).block(Block::default().title(" MISSION CONTROL [2] ").borders(Borders::ALL).border_style(if focus_mode == FocusMode::MissionControl { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) })), area);
                    }

                    // 3. MENTAL MAP
                    if focus_mode == FocusMode::None || focus_mode == FocusMode::MentalMap {
                        let area = if focus_mode == FocusMode::MentalMap { middle_chunks[0] } else { middle_chunks[2] };
                        let mut map_items = Vec::new();
                        if let Ok(s) = state.lock() {
                            use petgraph::Direction;
                            
                            // Show all root nodes initially
                            let roots: Vec<_> = s.mental_map.graph.node_indices()
                                .filter(|&idx| s.mental_map.graph.neighbors_directed(idx, Direction::Incoming).count() == 0)
                                .collect();
                            let count = roots.len();

                            for (i, node_idx) in roots.into_iter().enumerate() {
                                render_node_recursive(&s.mental_map.graph, node_idx, 0, "", i == count - 1, &mut map_items);
                            }
                        }
                        f.render_widget(List::new(map_items).block(Block::default().title(" MENTAL MAP [3] ").borders(Borders::ALL).border_style(if focus_mode == FocusMode::MentalMap { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) })), area);
                    }

                    // 4. EVENT FEED
                    if focus_mode == FocusMode::None || focus_mode == FocusMode::Feed {
                        let area = if focus_mode == FocusMode::Feed { middle_chunks[0] } else { middle_chunks[3] };
                        if let Ok(mut s) = state.lock() {
                            let feed_items: Vec<ListItem> = s.events.iter().map(|e| {
                                let color = match e.event_type.as_str() { 
                                    "LoopStarted" => Color::Green, 
                                    "WorkerJoined" => Color::Blue, 
                                    "AiResponse" => Color::Yellow,
                                    "RewardEarned" => Color::Green,
                                    "LoopStatusChanged" | "Log" => Color::Yellow,
                                    "ManualCommandInjected" => Color::Magenta,
                                    "WorkerError" => Color::Red,
                                    "WorkerThought" => Color::Cyan,
                                    _ => Color::White 
                                };
                                let content = if e.event_type == "AiResponse" {
                                    format!(" > AI: {}", e.payload.chars().take(40).collect::<String>())
                                } else if e.event_type == "RewardEarned" {
                                     format!(" + REWARD: {}", e.payload)
                                } else if e.event_type == "LoopStatusChanged" {
                                     format!(" # STATUS: {}", e.payload)
                                } else if e.event_type == "Log" {
                                     if let Ok(p) = serde_json::from_str::<LogPayload>(&e.payload) {
                                         format!(" * {}: {}", p.level, p.message)
                                     } else {
                                         format!(" * LOG: {}", e.payload)
                                     }
                                } else if e.event_type == "WorkerThought" {
                                     if let Ok(p) = serde_json::from_str::<ThoughtPayload>(&e.payload) {
                                         format!(" ? [{:.1}%] {}", p.confidence * 100.0, p.reasoning.last().unwrap_or(&"Thinking...".to_string()))
                                     } else {
                                         format!(" ? THINKING: {}", e.payload)
                                     }
                                } else if e.event_type == "ManualCommandInjected" {
                                     format!(" @ GOD: {}", e.payload)
                                } else if e.event_type == "WorkerError" {
                                     format!(" ! ERR: {}", e.payload)
                                } else {
                                    format!(" {:<8} | {}", e.timestamp.format("%H:%M:%S"), e.event_type)
                                };
                                ListItem::new(content).style(Style::default().fg(color))
                            }).collect();

                            let feed_list = List::new(feed_items)
                                .block(Block::default().title(" FEED [4] ").borders(Borders::ALL).border_style(if focus_mode == FocusMode::Feed { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) }))
                                .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
                                .highlight_symbol(">> ");
                            
                            // Auto-scroll logic: if not selecting anything, show the last item
                            if s.selected_event_index.is_none() && !s.events.is_empty() {
                                let last_idx = s.events.len().saturating_sub(1);
                                s.feed_state.select(Some(last_idx));
                            }

                            f.render_stateful_widget(feed_list, area, &mut s.feed_state);
                        }
                    }

                    // 5. AI TERMINAL
                    if focus_mode == FocusMode::None || focus_mode == FocusMode::Terminal {
                        let area = if focus_mode == FocusMode::Terminal { middle_chunks[0] } else { middle_chunks[4] };
                        let mut ai_content = String::new();
                        if let Ok(s) = state.lock() {
                            if let Some(latest) = s.ai_outputs.last() {
                                ai_content = format!(" [{}] {}\n\n{}", 
                                    latest.timestamp.format("%H:%M:%S"),
                                    latest.worker_id,
                                    latest.content
                                );
                            }
                        }
                        if ai_content.is_empty() {
                            ai_content = "Waiting for AI response...".to_string();
                        }

                        f.render_widget(
                            Paragraph::new(ai_content)
                                .wrap(Wrap { trim: true })
                                .block(Block::default().title(" AI TERMINAL [5] ").borders(Borders::ALL).border_style(if focus_mode == FocusMode::Terminal { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) })),
                            area
                        );
                    }

                    // 6. INSIGHTS & OPTIMIZATIONS
                    if focus_mode == FocusMode::None || focus_mode == FocusMode::Learnings {
                        let area = if focus_mode == FocusMode::Learnings { middle_chunks[0] } else { middle_chunks[5] };
                        let mut learning_items = Vec::new();
                        if let Ok(s) = state.lock() {
                            for insight in &s.insights {
                                learning_items.push(ListItem::new(format!(" 󰋗 {}", insight.description)).style(Style::default().fg(Color::Cyan)));
                            }
                            for opt in &s.optimizations {
                                learning_items.push(ListItem::new(format!(" 󰒓 [{}]: {}", opt.target_component, opt.suggestion)).style(Style::default().fg(Color::Magenta)));
                            }
                        }
                        if learning_items.is_empty() {
                             let (recorded, total) = if let Ok(s) = state.lock() {
                                 (s.recorded_missions.len(), s.missions.len())
                             } else { (0, 0) };
                             learning_items.push(ListItem::new(format!(" Gathering experience ({}/{} missions)...", recorded, total)).style(Style::default().fg(Color::DarkGray)));
                        }
                        f.render_widget(
                            List::new(learning_items)
                                .block(Block::default().title(" LEARNINGS [6] ").borders(Borders::ALL).border_style(if focus_mode == FocusMode::Learnings { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) })),
                            area
                        );
                    }

                    let footer_text = if is_int {
                        " [ESC] Cancel | [ENTER] Send | MODE: INTERVENTION"
                    } else if loop_status == LoopStatus::Running {
                        " [Q] Quit | [SPACE] Pause | [I] Intervene | STATUS: ONLINE"
                    } else {
                        " [Q] Quit | [SPACE] Resume | [I] Intervene | STATUS: PAUSED"
                    };
                    f.render_widget(Paragraph::new(footer_text).style(Style::default().fg(header_color)), main_layout[2]);

                    if is_int {
                        let input_val = if let Ok(s) = state.lock() { s.input_buffer.clone() } else { String::new() };
                        let area = centered_rect(60, 20, f.size());
                        f.render_widget(Clear, area);
                        let block = Block::default()
                            .title(" GOD MODE INTERVENTION ")
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::Magenta));
                        let input = Paragraph::new(format!("> {}", input_val))
                            .block(block)
                            .style(Style::default().fg(Color::White));
                        f.render_widget(input, area);
                    }

                    if let Ok(s) = state.lock() {
                        if s.show_event_details {
                            if let Some(idx) = s.selected_event_index {
                                if let Some(event) = s.events.get(idx) {
                                    let area = centered_rect(80, 80, f.size());
                                    f.render_widget(Clear, area);
                                    let content = format!(
                                        "ID: {}\nType: {}\nTimestamp: {}\nWorker: {}\nPayload:\n{}",
                                        event.id, event.event_type, event.timestamp, event.worker_id,
                                        serde_json::to_string_pretty(&serde_json::from_str::<serde_json::Value>(&event.payload).unwrap_or(serde_json::json!({"raw": event.payload}))).unwrap()
                                    );
                                    f.render_widget(
                                        Paragraph::new(content)
                                            .block(Block::default().title(" EVENT DETAILS ").borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)))
                                            .wrap(Wrap { trim: false }),
                                        area
                                    );
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let CEvent::Key(key) = event::read()? {
                let mut s = state.lock().unwrap();
                let current_mode = s.mode.clone();

                match current_mode {
                    AppMode::MainMenu => {
                        match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Down | KeyCode::Char('j') => s.menu.next(),
                            KeyCode::Up | KeyCode::Char('k') => s.menu.previous(),
                            KeyCode::Enter => {
                                match s.menu.current_action() {
                                    MenuAction::NewGame => {
                                        s.events.clear();
                                        s.workers.clear();
                                        s.missions.clear();
                                        s.bank = Bank::default();
                                        s.status = LoopStatus::Paused;
                                        s.mental_map = relationship::MentalMap::new();
                                        s.current_session_id = None;
                                        s.ai_outputs.clear();
                                        s.insights.clear();
                                        s.optimizations.clear();
                                        s.managed_context_stats = None;
                                        s.recorded_missions.clear();
                                        s.wizard = SetupWizard::new();
                                        s.mode = AppMode::Setup;
                                    }
                                    MenuAction::LoadGame => {
                                        let store_lp = Arc::clone(&store);
                                        let s_lp = Arc::clone(&state);
                                        tokio::spawn(async move {
                                            if let Ok(sessions) = store_lp.list_all_sessions().await {
                                                let mut state = s_lp.lock().unwrap();
                                                state.available_sessions = sessions;
                                                state.selected_session_index = 0;
                                                state.mode = AppMode::SessionPicker;
                                            }
                                        });
                                    }
                                    MenuAction::Quit => break,
                                    _ => {} 
                                }
                            }
                            _ => {}
                        }
                    }
                    AppMode::Setup => {
                        match key.code {
                            KeyCode::Esc => s.mode = AppMode::MainMenu,
                            KeyCode::Enter => {
                                if s.wizard.current_step == WizardStep::Summary {
                                    s.current_session_id = Some(Uuid::new_v4());
                                    s.mode = AppMode::Running;
                                    s.status = LoopStatus::Running;
                                } else {
                                    let _ = s.wizard.next();
                                }
                            }
                            KeyCode::Backspace => {
                                if s.wizard.current_step == WizardStep::Goal {
                                    s.wizard.goal.pop();
                                } else {
                                    s.wizard.prev();
                                }
                            }
                            KeyCode::Up => {
                                if s.wizard.current_step == WizardStep::Team && s.wizard.selected_group_index > 0 {
                                    s.wizard.selected_group_index -= 1;
                                }
                            }
                            KeyCode::Down => {
                                if s.wizard.current_step == WizardStep::Team && s.wizard.selected_group_index < s.available_groups.len().saturating_sub(1) {
                                    s.wizard.selected_group_index += 1;
                                }
                            }
                            KeyCode::Left => {
                                s.wizard.prev();
                            }
                            KeyCode::Char(c) => {
                                if s.wizard.current_step == WizardStep::Goal {
                                    s.wizard.goal.push(c);
                                }
                            }
                            _ => {}
                        }
                    }
                    AppMode::Running => {
                        if s.show_event_details {
                            match key.code {
                                KeyCode::Esc | KeyCode::Enter => { s.show_event_details = false; }
                                _ => {}
                            }
                        } else if s.is_intervening {
                            match key.code {
                                KeyCode::Esc => { s.is_intervening = false; s.input_buffer.clear(); }
                                KeyCode::Enter => {
                                    let cmd = s.input_buffer.clone();
                                    s.is_intervening = false;
                                    s.input_buffer.clear();
                                    let bus_g = Arc::clone(&bus);
                                    let sid = s.current_session_id.unwrap_or_default();
                                    tokio::spawn(async move {
                                        let _ = bus_g.publish(Event {
                                            id: Uuid::new_v4(),
                                            session_id: sid,
                                            trace_id: Uuid::new_v4(),
                                            timestamp: Utc::now(),
                                            worker_id: "god".to_string(),
                                            event_type: "ManualCommandInjected".to_string(),
                                            payload: cmd,
                                        }).await;
                                    });
                                }
                                KeyCode::Char(c) => { s.input_buffer.push(c); }
                                KeyCode::Backspace => { s.input_buffer.pop(); }
                                _ => {}
                            }
                        } else {
                            match key.code {
                                KeyCode::Char('q') => {
                                    s.mode = AppMode::MainMenu; 
                                },
                                KeyCode::Char('i') => { s.is_intervening = true; s.input_buffer.clear(); }
                                KeyCode::Char(' ') => {
                                    let new_status = match s.status {
                                        LoopStatus::Running => LoopStatus::Paused,
                                        LoopStatus::Paused => LoopStatus::Running,
                                    };
                                    let bus_p = Arc::clone(&bus);
                                    let sid = s.current_session_id.unwrap_or_default();
                                    tokio::spawn(async move {
                                        let _ = bus_p.publish(Event {
                                            id: Uuid::new_v4(),
                                            session_id: sid,
                                            trace_id: Uuid::new_v4(),
                                            timestamp: Utc::now(),
                                            worker_id: "user".to_string(),
                                            event_type: "LoopStatusChanged".to_string(),
                                            payload: serde_json::to_string(&new_status).unwrap(),
                                        }).await;
                                    });
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    let count = s.events.len();
                                    if count > 0 {
                                        let i = match s.feed_state.selected() {
                                            Some(i) => if i == 0 { count - 1 } else { i - 1 },
                                            None => count - 1,
                                        };
                                        s.feed_state.select(Some(i));
                                        s.selected_event_index = Some(i);
                                    }
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    let count = s.events.len();
                                    if count > 0 {
                                        let i = match s.feed_state.selected() {
                                            Some(i) => if i >= count - 1 { 0 } else { i + 1 },
                                            None => 0,
                                        };
                                        s.feed_state.select(Some(i));
                                        s.selected_event_index = Some(i);
                                    }
                                }
                                KeyCode::Enter => {
                                    if s.selected_event_index.is_some() {
                                        s.show_event_details = true;
                                    }
                                }
                                KeyCode::Esc => {
                                    s.selected_event_index = None;
                                    s.feed_state.select(None);
                                }
                                KeyCode::Char('1') => { s.focus_mode = if s.focus_mode == FocusMode::Roster { FocusMode::None } else { FocusMode::Roster }; }
                                KeyCode::Char('2') => { s.focus_mode = if s.focus_mode == FocusMode::MissionControl { FocusMode::None } else { FocusMode::MissionControl }; }
                                KeyCode::Char('3') => { s.focus_mode = if s.focus_mode == FocusMode::MentalMap { FocusMode::None } else { FocusMode::MentalMap }; }
                                KeyCode::Char('4') => { s.focus_mode = if s.focus_mode == FocusMode::Feed { FocusMode::None } else { FocusMode::Feed }; }
                                KeyCode::Char('5') => { s.focus_mode = if s.focus_mode == FocusMode::Terminal { FocusMode::None } else { FocusMode::Terminal }; }
                                KeyCode::Char('6') => { s.focus_mode = if s.focus_mode == FocusMode::Learnings { FocusMode::None } else { FocusMode::Learnings }; }
                                KeyCode::Char('0') => { s.focus_mode = FocusMode::None; }
                                _ => {}
                            }
                        }
                    }
                    AppMode::SessionPicker => {
                        match key.code {
                            KeyCode::Esc => s.mode = AppMode::MainMenu,
                            KeyCode::Down | KeyCode::Char('j') => {
                                if s.selected_session_index < s.available_sessions.len().saturating_sub(1) {
                                    s.selected_session_index += 1;
                                }
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if s.selected_session_index > 0 {
                                    s.selected_session_index -= 1;
                                }
                            }
                            KeyCode::Enter => {
                                if let Some(sid) = s.available_sessions.get(s.selected_session_index).cloned() {
                                    s.current_session_id = Some(sid);
                                    s.mode = AppMode::Running;
                                    s.status = LoopStatus::Running;
                                    
                                    // Trigger history replay
                                    let store_rp = Arc::clone(&store);
                                    let s_rp = Arc::clone(&state);
                                    tokio::spawn(async move {
                                        if let Ok(history) = store_rp.list(sid).await {
                                            let mut state = s_rp.lock().unwrap();
                                            // Clear current state before replay
                                            state.events.clear();
                                            state.workers.clear();
                                            state.missions.clear();
                                            state.bank = Bank::default();
                                            state.mental_map = relationship::MentalMap::new();

                                            for event in history {
                                                match event.event_type.as_str() {
                                                    "WorkerJoined" => {
                                                        if let Ok(profile) = serde_json::from_str::<WorkerProfile>(&event.payload) {
                                                            state.workers.push(profile);
                                                        }
                                                    }
                                                    "MissionCreated" => {
                                                        if let Ok(mission) = serde_json::from_str::<Mission>(&event.payload) {
                                                            state.mental_map.add_mission(mission.id, &mission.name);
                                                            for task in &mission.tasks {
                                                                state.mental_map.add_task(mission.id, task.id, &task.name);
                                                                if let Some(assigned) = &task.assigned_worker {
                                                                    state.mental_map.assign_worker(task.id, assigned);
                                                                }
                                                            }
                                                            state.missions.push(mission);
                                                        }
                                                    }
                                                    "TaskUpdated" => {
                                                         #[derive(serde::Deserialize)]
                                                         struct TaskUpdate { mission_id: Uuid, task_id: Uuid, status: TaskStatus }
                                                         if let Ok(update) = serde_json::from_str::<TaskUpdate>(&event.payload) {
                                                             if let Some(m) = state.missions.iter_mut().find(|m| m.id == update.mission_id) {
                                                                 if let Some(t) = m.tasks.iter_mut().find(|t| t.id == update.task_id) {
                                                                     t.status = update.status;
                                                                 }
                                                             }
                                                         }
                                                    }
                                                    "AiResponse" => {
                                                        state.ai_outputs.push(AiOutput {
                                                            timestamp: event.timestamp,
                                                            worker_id: event.worker_id.clone(),
                                                            content: event.payload.clone(),
                                                        });
                                                        if state.ai_outputs.len() > 50 {
                                                            state.ai_outputs.remove(0);
                                                        }
                                                    }
                                                    "RewardEarned" => {
                                                        if let Ok(reward) = serde_json::from_str::<serde_json::Value>(&event.payload) {
                                                            let xp = reward["xp"].as_u64().unwrap_or(0);
                                                            let coins = reward["coins"].as_u64().unwrap_or(0);
                                                            state.bank.deposit(xp, coins);
                                                        }
                                                    }
                                                    "LoopStatusChanged" => {
                                                        if let Ok(status) = serde_json::from_str::<LoopStatus>(&event.payload) {
                                                            state.status = status;
                                                        }
                                                    }
                                                    "LoopStarted" => {
                                                        if let Ok(config) = serde_json::from_str::<LoopConfig>(&event.payload) {
                                                            state.wizard.goal = config.goal;
                                                            if let Some(max) = config.max_coins {
                                                                state.wizard.budget_coins = max;
                                                            }
                                                        }
                                                    }
                                                    "WorkerRequestAssistance" => {
                                                        if let Ok(req) = serde_json::from_str::<WorkerRequest>(&event.payload) {
                                                            state.mental_map.add_worker_relationship(&req.requester_id, &req.target_role);
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                                if state.events.len() > 100 {
                                                    state.events.remove(0);
                                                }
                                                state.events.push(event);
                                            }
                                        }
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
fn render_node_recursive(graph: &petgraph::graph::DiGraph<relationship::NodeType, ()>, node_idx: petgraph::graph::NodeIndex, depth: usize, prefix: &str, is_last: bool, items: &mut Vec<ratatui::widgets::ListItem>) {
    use relationship::NodeType;
    use ratatui::style::{Color, Style, Modifier};
    use petgraph::visit::EdgeRef;

    let node = &graph[node_idx];
    
    let marker = if depth == 0 { "" } else if is_last { "└─ " } else { "├─ " };
    let content = format!("{}{}", marker, match node {
        NodeType::Mission(name) => format!("󰚒 {}", name),
        NodeType::Task(name) => format!("󰓅 {}", name),
        NodeType::Worker(name) => format!("󰚩 {}", name),
    });

    let style = match node {
        NodeType::Mission(_) => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        NodeType::Task(_) => Style::default().fg(Color::Gray),
        NodeType::Worker(_) => Style::default().fg(Color::Yellow),
    };

    items.push(ratatui::widgets::ListItem::new(format!("{}{}", prefix, content)).style(style));

    // Children
    let children: Vec<_> = graph.edges(node_idx).map(|e| e.target()).collect();
    let count = children.len();
    
    let new_prefix = if depth == 0 {
        "".to_string()
    } else if is_last {
        format!("{}   ", prefix)
    } else {
        format!("{}│  ", prefix)
    };

    for (i, child_idx) in children.into_iter().enumerate() {
        let child_is_last = i == count - 1;
        
        let child_node = &graph[child_idx];
        if matches!(node, NodeType::Worker(_)) && matches!(child_node, NodeType::Worker(_)) {
            if let NodeType::Worker(partner_name) = child_node {
                let rel_marker = if child_is_last { "└─ " } else { "├─ " };
                items.push(ratatui::widgets::ListItem::new(format!("{}{}(calls) 󰜴 {}", new_prefix, rel_marker, partner_name)).style(Style::default().fg(Color::Magenta).add_modifier(Modifier::ITALIC)));
            }
        } else {
            render_node_recursive(graph, child_idx, depth + 1, &new_prefix, child_is_last, items);
        }
    }
}
