mod relationship;

use anyhow::Result;
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
    widgets::{Block, Borders, List, ListItem, Paragraph, Table, Row, Cell, Clear, Wrap},
    Terminal,
};
use std::io;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    goal: Option<String>,

    #[arg(short, long)]
    max_coins: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AiOutput {
    timestamp: DateTime<Utc>,
    worker_id: String,
    content: String,
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
    let _args = Args::parse();

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
                        }
                    }
                    _ => {}
                }

                if s.events.len() > 100 {
                    s.events.remove(0);
                }
                s.events.push(event);

                // F40: Check all missions for completion to record outcome
                let mut completed_missions = Vec::new();
                for m in &s.missions {
                    if !m.tasks.is_empty() && m.tasks.iter().all(|t| t.status == TaskStatus::Success || t.status == TaskStatus::Failure) {
                        completed_missions.push(m.clone());
                    }
                }
                
                if !completed_missions.is_empty() {
                    let l_c = Arc::clone(&learning_c);
                    let s_cc = Arc::clone(&state_c);
                    tokio::spawn(async move {
                        for m in completed_missions {
                            let success = m.tasks.iter().all(|t| t.status == TaskStatus::Success);
                            let outcome = MissionOutcome {
                                mission_id: m.id,
                                success,
                                duration_seconds: 0,
                                metadata: serde_json::json!({"name": m.name}),
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
                use ifcl_core::planner::{BasicPlanner, Planner};
                let planner = BasicPlanner;
                let generated_missions = planner.generate_initial_missions(&goal);

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
                use ifcl_core::context::{SlidingWindowPruner, SimpleTokenCounter, ContextPruner};
                let pruner = SlidingWindowPruner;
                let counter = SimpleTokenCounter;
                
                // Prune to a very small limit (e.g. 100 tokens) to demonstrate pruning
                let managed = pruner.prune(&all_events, 200, &counter);
                
                if let Ok(mut st) = state_sim_monitor.lock() {
                    st.managed_context_stats = Some((managed.estimated_tokens, managed.pruned_count));
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
                                // Log STDERR
                                if !result.stderr.is_empty() {
                                    let _ = bus_simulation.publish(Event {
                                        id: Uuid::new_v4(),
                                        session_id: sid,
                                        trace_id: Uuid::new_v4(),
                                        timestamp: Utc::now(),
                                        worker_id: worker.clone(),
                                        event_type: "Log".to_string(), 
                                        payload: serde_json::to_string(&LogPayload { level: "STDERR".to_string(), message: result.stderr.clone() }).unwrap(),
                                    }).await;
                                }

                                if result.status.success() {
                                    let _ = bus_simulation.publish(Event {
                                        id: Uuid::new_v4(),
                                        session_id: sid,
                                        trace_id: Uuid::new_v4(),
                                        timestamp: Utc::now(),
                                        worker_id: worker.clone(),
                                        event_type: "AiResponse".to_string(), payload: result.stdout,
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
                                    let _ = bus_simulation.publish(Event {
                                        id: Uuid::new_v4(),
                                        session_id: sid,
                                        trace_id: Uuid::new_v4(),
                                        timestamp: Utc::now(),
                                        worker_id: worker.clone(),
                                        event_type: "WorkerError".to_string(), 
                                        payload: format!("CLI exited with non-zero: {}", result.status),
                                    }).await;
                                }
                            }
                            Err(e) => {
                                let _ = bus_simulation.publish(Event {
                                    id: Uuid::new_v4(),
                                    session_id: sid,
                                    trace_id: Uuid::new_v4(),
                                    timestamp: Utc::now(),
                                    worker_id: worker.clone(),
                                    event_type: "WorkerError".to_string(), payload: e.to_string(),
                                }).await;
                            }
                        }
                    } else if name == "Init Repo" {
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
                        // Generic success for other tasks
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
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
                        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)].as_ref())
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

                    // Middle Layout
                    let middle_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(12), 
                            Constraint::Percentage(18), 
                            Constraint::Percentage(18), 
                            Constraint::Percentage(18),
                            Constraint::Percentage(18),
                            Constraint::Percentage(18)
                        ].as_ref())
                        .split(main_layout[1]);

                    // 1. ROSTER
                    let mut worker_items = Vec::new();
                    if let Ok(s) = state.lock() {
                        worker_items = s.workers.iter().map(|w| {
                            let symbol = match w.role { WorkerRole::Git => "󰊢", WorkerRole::Coder => "󰅩", WorkerRole::Architect => "󰉪", _ => "󰚩" };
                            ListItem::new(format!(" {} {}", symbol, w.name)).style(Style::default().fg(Color::Yellow))
                        }).collect();
                    }
                    f.render_widget(List::new(worker_items).block(Block::default().title(" BARRACKS ").borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray))), middle_chunks[0]);

                    // 2. MISSION CONTROL
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
                    f.render_widget(Table::new(rows, widths).header(Row::new(vec!["MISSION", "TASK", "STATUS", "WORKER"]).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))).block(Block::default().title(" MISSION CONTROL ").borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray))), middle_chunks[1]);

                    // 3. MENTAL MAP
                    let mut map_items = Vec::new();
                    if let Ok(s) = state.lock() {
                        use petgraph::Direction;
                        
                        // Show all root nodes initially
                        for node_idx in s.mental_map.graph.node_indices() {
                            let in_degree = s.mental_map.graph.neighbors_directed(node_idx, Direction::Incoming).count();
                            if in_degree == 0 {
                                render_node_recursive(&s.mental_map.graph, node_idx, 0, &mut map_items);
                            }
                        }
                    }
                    f.render_widget(List::new(map_items).block(Block::default().title(" MENTAL MAP ").borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray))), middle_chunks[2]);

                    // 4. EVENT FEED
                    let mut feed_items = Vec::new();
                    if let Ok(s) = state.lock() {
                        feed_items = s.events.iter().rev().map(|e| {
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
                                " > AI: ...".to_string()
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
                    }
                    f.render_widget(List::new(feed_items).block(Block::default().title(" FEED ").borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray))), middle_chunks[3]);

                    // 5. AI TERMINAL
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
                            .block(Block::default().title(" AI TERMINAL ").borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow))),
                        middle_chunks[4]
                    );

                    // 6. INSIGHTS & OPTIMIZATIONS
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
                         learning_items.push(ListItem::new(" Gathering experience...").style(Style::default().fg(Color::DarkGray)));
                    }
                    f.render_widget(
                        List::new(learning_items)
                            .block(Block::default().title(" LEARNINGS ").borders(Borders::ALL).border_style(Style::default().fg(Color::Magenta))),
                        middle_chunks[5]
                    );

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
                                        s.wizard = SetupWizard::new(); // Reset
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
                        if s.is_intervening {
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
                                    // Return to Main Menu instead of quitting immediately
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
fn render_node_recursive(graph: &petgraph::graph::DiGraph<relationship::NodeType, ()>, node_idx: petgraph::graph::NodeIndex, depth: usize, items: &mut Vec<ratatui::widgets::ListItem>) {
    use relationship::NodeType;
    use ratatui::style::{Color, Style, Modifier};
    use petgraph::visit::EdgeRef;

    let node = &graph[node_idx];
    let indent = "  ".repeat(depth);
    
    match node {
        NodeType::Mission(name) => {
            items.push(ratatui::widgets::ListItem::new(format!("{}󰚒 {}", indent, name)).style(Style::default().fg(Color::Cyan)));
        }
        NodeType::Task(name) => {
            items.push(ratatui::widgets::ListItem::new(format!("{}└─󰓅 {}", indent, name)).style(Style::default().fg(Color::DarkGray)));
        }
        NodeType::Worker(name) => {
            items.push(ratatui::widgets::ListItem::new(format!("{}└─󰚩 {}", indent, name)).style(Style::default().fg(Color::Yellow)));
        }
    }

    // Children
    for edge in graph.edges(node_idx) {
        let child_idx = edge.target();
        // Special case for worker-to-worker relationships to keep them indented but distinct in label
        let child_node = &graph[child_idx];
        if matches!(node, NodeType::Worker(_)) && matches!(child_node, NodeType::Worker(_)) {
            if let NodeType::Worker(partner_name) = child_node {
                let rel_indent = "  ".repeat(depth + 1);
                items.push(ratatui::widgets::ListItem::new(format!("{} (calls) -> {}", rel_indent, partner_name)).style(Style::default().fg(Color::Magenta).add_modifier(Modifier::ITALIC)));
            }
        } else {
            render_node_recursive(graph, child_idx, depth + 1, items);
        }
    }
}
