mod relationship;
mod cli;

use anyhow::Result;
use cli::CliArgs;
use chrono::{Utc, DateTime};
use serde::{Serialize, Deserialize};
use clap::Parser;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ifcl_core::{
    Event, EventStore, InMemoryEventBus, Mission, TaskStatus, 
    Bank, LoopStatus, SqliteEventStore, WorkerProfile, WorkerRole, LoopConfig,
    MarketplaceLoader, AppMode, MenuAction, MenuState, SetupWizard, WizardStep, LogPayload, ThoughtPayload,
    WorkerOutputPayload, CliWorker,
    groups::WorkerGroup, orchestrator::WorkerRequest,
    learning::{LearningManager, BasicLearningManager, Insight, Optimization, MissionOutcome},
    planner::{Planner, BasicPlanner},
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
    last_event_type: String,
    pulse: bool,
    feed_state: ListState,
    selected_event_index: Option<usize>,
    show_event_details: bool,
    focus_mode: FocusMode,
    frame_count: u64,
}


#[tokio::main]
async fn main() -> Result<()> {
    let _args = CliArgs::parse();

    // 1. Initialize Infrastructure (Shared)
    let bus: Arc<dyn ifcl_core::EventBus> = Arc::new(InMemoryEventBus::new(200));
    let store = Arc::new(SqliteEventStore::new("sqlite://ifcl.db?mode=rwc").await?);
    let learning_manager = Arc::new(BasicLearningManager::new());
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

        // Create initial mission
        let mission = orchestrator.create_mission(
            Uuid::new_v4(), // New session ID for headless
            &goal,
            vec![
                ("Initialize Project".to_string(), "Initialize workspace and create basic structure".to_string()),
                ("Run Command".to_string(), "echo '# Infinite Coding Loop Benchmark' > README.md".to_string())
            ],
            workspace.clone()
        ).await?;

        println!("Mission Created: {} (ID: {})", mission.name, mission.id);

        // Headless Execution Loop
        let worker = CliWorker::new("Headless-Bot", WorkerRole::Coder);
        let bus_c = Arc::clone(&bus);
        
        // Subscribe to print output
        let mut rx = bus.subscribe();
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                if event.event_type == "WorkerOutput" {
                    if let Ok(payload) = serde_json::from_str::<WorkerOutputPayload>(&event.payload) {
                        print!("{}", payload.content); // Stream to stdout
                    }
                }
            }
        });

        loop {
            // Check for pending tasks
            let missions = orchestrator.get_missions().await?;
            let mut pending_task = None;
            let mut all_done = true;

            for m in missions {
                for t in m.tasks {
                    if t.status == TaskStatus::Pending {
                        pending_task = Some((m.id, t.id));
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

            if let Some((mid, tid)) = pending_task {
                println!("▶ Executing Task...");
                match orchestrator.execute_task(Arc::clone(&bus_c), mid, tid, &worker).await {
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
            if let Some(ws) = _args.workspace {
                w.workspace_path = ws;
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
        last_event_type: "Waiting...".to_string(),
        pulse: false,
        feed_state: ListState::default(),
        selected_event_index: None,
        show_event_details: false,
        focus_mode: FocusMode::None,
        frame_count: 0,
    }));

    // Replay history will be handled later

    // 3. Subscription & Event Processing
    let bus_c = Arc::clone(&bus);
    let state_c = Arc::clone(&state);
    let store_c = Arc::clone(&store);
    let bus_reward = Arc::clone(&bus); // Keep this for the reward publishing inside the loop
    let learning_c = Arc::clone(&learning_manager);
    
    // 4. Load Marketplace items (Async)
    let m_bus = Arc::clone(&bus);
    let s_c_marketplace = Arc::clone(&state_c);
    
    tokio::spawn(async move {
        // Create marketplace directories if they don't exist
        let _ = std::fs::create_dir_all("marketplace/workers");
        let _ = std::fs::create_dir_all("marketplace/missions");

        // Load Workers
        let marketplace_workers = MarketplaceLoader::load_workers("marketplace/workers");
        for worker in marketplace_workers {
            let m_bus_w = Arc::clone(&m_bus);
            let worker_cloned: WorkerProfile = worker.clone();
            let s_c = Arc::clone(&s_c_marketplace);
            tokio::spawn(async move {
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
            });
        }

        // Load Missions
        let marketplace_missions = MarketplaceLoader::load_missions("marketplace/missions");
        for mission in marketplace_missions {
            let m_bus_m = Arc::clone(&m_bus);
            let mission_cloned: Mission = mission.clone();
            let s_c = Arc::clone(&s_c_marketplace);
            tokio::spawn(async move {
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
            });
        }
    });

    tokio::spawn(async move {
        let mut sub = bus_c.subscribe();
        while let Ok(event) = sub.recv().await {
            let _ = store_c.append(event.clone()).await;
            
            if let Ok(mut s) = state_c.lock() {
                s.last_event_at = Utc::now();
                s.last_event_type = event.event_type.clone();
                s.pulse = !s.pulse;

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
                    "WorkerOutput" => {
                        if let Ok(payload) = serde_json::from_str::<WorkerOutputPayload>(&event.payload) {
                            if let Some(latest) = s.ai_outputs.last_mut() {
                                if latest.worker_id == event.worker_id {
                                    latest.content.push_str(&payload.content);
                                    latest.content.push('\n');
                                } else {
                                    s.ai_outputs.push(AiOutput {
                                        timestamp: event.timestamp,
                                        worker_id: event.worker_id.clone(),
                                        content: payload.content,
                                    });
                                }
                            } else {
                                s.ai_outputs.push(AiOutput {
                                    timestamp: event.timestamp,
                                    worker_id: event.worker_id.clone(),
                                    content: payload.content,
                                });
                            }
                            if s.ai_outputs.len() > 50 {
                                s.ai_outputs.remove(0);
                            }
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
                    "Log" => {
                         if let Ok(payload) = serde_json::from_str::<LogPayload>(&event.payload) {
                             s.last_event_type = format!("{}: {}", payload.level, payload.message).chars().take(40).collect();
                             // Also push to AI terminal for visibility
                             s.ai_outputs.push(AiOutput {
                                 timestamp: event.timestamp,
                                 worker_id: event.worker_id.clone(),
                                 content: format!("LOG [{}]: {}", payload.level, payload.message),
                             });
                             if s.ai_outputs.len() > 50 { s.ai_outputs.remove(0); }
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


    // 5. Background Task Runner (The Real Loop Engine)
    let bus_runner = Arc::clone(&bus);
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
                                // Capture assigned_worker for worker selection
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
                // Execute task with the assigned worker
                let bus_exec = Arc::clone(&bus_runner);
                let orch_exec = Arc::clone(&orch_runner);
                let worker = CliWorker::new(&worker_name, WorkerRole::Coder);
                
                let _ = orch_exec.execute_task(bus_exec.clone(), mid, tid, &worker).await;
                
                // Check for goal completion after task execution
                let all_done = {
                    let s = state_runner.lock().unwrap();
                    !s.missions.is_empty() && s.missions.iter().all(|m| 
                        m.tasks.iter().all(|t| t.status == TaskStatus::Success || t.status == TaskStatus::Failure)
                    )
                };
                
                if all_done {
                    // Get session_id for the event
                    let session_id = {
                        state_runner.lock().unwrap().current_session_id.unwrap_or_default()
                    };
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
                // Idle: short poll
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
        }
    });


    // 6. TUI Rendering Loop
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
                                                   
   A U T O N O M O U S   C O D I N G   L O O P
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

                    let (step, goal, stack, workspace, team, budget, avail_groups, sel_grp_idx) = if let Ok(s) = state.lock() {
                        (s.wizard.current_step.clone(), s.wizard.goal.clone(), s.wizard.stack.clone(), s.wizard.workspace_path.clone(), s.wizard.team_size, s.wizard.budget_coins, s.available_groups.clone(), s.wizard.selected_group_index)
                    } else { (WizardStep::Goal, String::new(), String::new(), String::new(), 0, 0, Vec::new(), 0) };

                    let step_text = match step {
                        WizardStep::Goal => "Step 1/6: Define Objective",
                        WizardStep::Stack => "Step 2/6: Technology Stack",
                        WizardStep::Workspace => "Step 3/6: Project Workspace",
                        WizardStep::Team => "Step 4/6: Squad Size",
                        WizardStep::Budget => "Step 5/6: Resource Credits",
                        WizardStep::Summary => "Step 6/6: Final Review",
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
                        WizardStep::Workspace => format!("Project output directory:\n (Where files will be built)\n\n> {}", workspace),
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
                            "Mission: {}\nStack: {}\nWorkspace: {}\nTeam: {} Workers\nBudget: {} Coins\n\n[ PRESS ENTER TO START ]",
                            goal, stack, workspace, team, budget
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
                    let mut state_guard = state.lock().unwrap();
                    let s = &mut *state_guard;
                    let focus_mode = s.focus_mode;

                    let main_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(3), // Header
                            Constraint::Length(3), // Progress Bar
                            Constraint::Min(0),    // Main content
                            Constraint::Length(1), // Activity Bar
                            Constraint::Length(1), // Debug/Footer
                        ].as_ref())
                        .split(f.size());

                    // --- Header ---
                    let header_color = if s.is_intervening { Color::Magenta } else {
                        match s.status {
                            LoopStatus::Running => Color::Cyan,
                            LoopStatus::Paused => Color::Yellow,
                        }
                    };
                    let ctx_info = if let Some((tokens, pruned)) = s.managed_context_stats {
                        format!(" | CTX: {}tk ({}p)", tokens, pruned)
                    } else {
                        String::new()
                    };

                    let pulse_indicator = if s.status == LoopStatus::Running {
                        // Animated braille spinner when running
                        const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                        SPINNER_FRAMES[(s.frame_count as usize) % SPINNER_FRAMES.len()]
                    } else {
                        "⏸"  // Paused indicator
                    };
                    let last_activity_secs = (Utc::now() - s.last_event_at).num_seconds();
                    let activity_timer = format!(" ({}s ago)", last_activity_secs);

                    let header = Paragraph::new(format!(" {} OBJ: {:<20} | XP: {} | $: {} | ST: {:?}{}{}", 
                        pulse_indicator,
                        if s.wizard.goal.len() > 20 { format!("{}...", &s.wizard.goal[..17]) } else { s.wizard.goal.clone() },
                        s.bank.xp, s.bank.coins, s.status, ctx_info, activity_timer))
                        .style(Style::default().fg(header_color).add_modifier(Modifier::BOLD))
                        .block(Block::default().title(" INFINITE CODING LOOP [v0.1.0] ").borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
                    f.render_widget(header, main_layout[0]);

                    // --- Progress Bar ---
                    // ... (unchanged gauge logic) ...
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

                    // Middle Layout
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
                    
                    // ... (panels 1-6 unchanged) ...
                    // 1. ROSTER
                    if focus_mode == FocusMode::None || focus_mode == FocusMode::Roster {
                        let area = middle_chunks[0];
                        // Find workers assigned to running tasks
                        let active_workers: std::collections::HashSet<String> = s.missions.iter()
                            .flat_map(|m| m.tasks.iter())
                            .filter(|t| t.status == TaskStatus::Running)
                            .filter_map(|t| t.assigned_worker.clone())
                            .collect();
                        
                        const WORKER_SPINNER: [&str; 4] = ["⣾", "⣽", "⣻", "⢿"];
                        let worker_spinner = WORKER_SPINNER[(s.frame_count as usize) % WORKER_SPINNER.len()];
                        
                        let worker_items: Vec<_> = s.workers.iter().map(|w| {
                            let symbol = match w.role { WorkerRole::Git => "󰊢", WorkerRole::Coder => "󰅩", WorkerRole::Architect => "󰉪", _ => "󰚩" };
                            let is_active = active_workers.contains(&w.name);
                            let activity_indicator = if is_active { format!(" {}", worker_spinner) } else { String::new() };
                            let style = if is_active {
                                let blink_phase = (s.frame_count / 5).is_multiple_of(2);
                                if blink_phase {
                                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)
                                }
                            } else {
                                Style::default().fg(Color::Yellow)
                            };
                            ListItem::new(format!(" {} {}{}", symbol, w.name, activity_indicator)).style(style)
                        }).collect();
                        f.render_widget(List::new(worker_items).block(Block::default().title(" BARRACKS [1] ").borders(Borders::ALL).border_style(if focus_mode == FocusMode::Roster { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) })), area);
                    }

                    // 2. MISSION CONTROL
                    if focus_mode == FocusMode::None || focus_mode == FocusMode::MissionControl {
                        let area = middle_chunks[if focus_mode == FocusMode::MissionControl { 0 } else { 1 }];
                        let mut rows = Vec::new();
                        // Spinner frames for running tasks
                        const TASK_SPINNER: [&str; 4] = ["◐", "◓", "◑", "◒"];
                        let spinner_frame = TASK_SPINNER[(s.frame_count as usize) % TASK_SPINNER.len()];
                        let blink_phase = (s.frame_count / 5).is_multiple_of(2); // Alternate every 5 frames
                        
                        for mission in &s.missions {
                            for task in &mission.tasks {
                                let status_text = match task.status {
                                    TaskStatus::Running => format!("{} EXECUTING...", spinner_frame),
                                    _ => format!("{:?}", task.status),
                                };
                                let status_style = match task.status {
                                    TaskStatus::Pending => Style::default().fg(Color::DarkGray),
                                    TaskStatus::Running => {
                                        // High contrast blinking - very visible alternation
                                        if blink_phase {
                                            Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
                                        } else {
                                            Style::default().fg(Color::Yellow).bg(Color::Black).add_modifier(Modifier::BOLD)
                                        }
                                    }
                                    TaskStatus::Success => Style::default().fg(Color::Green),
                                    TaskStatus::Failure => Style::default().fg(Color::Red),
                                };
                                let task_name_style = if task.status == TaskStatus::Running {
                                    // Blink the task name too with high contrast
                                    if blink_phase {
                                        Style::default().fg(Color::Yellow).bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                                    } else {
                                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                                    }
                                } else {
                                    Style::default().add_modifier(Modifier::BOLD)
                                };
                                rows.push(Row::new(vec![
                                    Cell::from(mission.name.clone()).style(Style::default().fg(Color::DarkGray)),
                                    Cell::from(task.name.clone()).style(task_name_style),
                                    Cell::from(status_text).style(status_style),
                                    Cell::from(task.assigned_worker.clone().unwrap_or_default()).style(Style::default().fg(Color::Yellow)),
                                ]));
                            }
                        }
                        let widths = [Constraint::Percentage(30), Constraint::Percentage(30), Constraint::Percentage(20), Constraint::Percentage(20)];
                        f.render_widget(Table::new(rows, widths).header(Row::new(vec!["MISSION", "TASK", "STATUS", "WORKER"]).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))).block(Block::default().title(" MISSION CONTROL [2] ").borders(Borders::ALL).border_style(if focus_mode == FocusMode::MissionControl { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) })), area);
                    }

                    // 3. MENTAL MAP
                    if focus_mode == FocusMode::None || focus_mode == FocusMode::MentalMap {
                        let area = middle_chunks[if focus_mode == FocusMode::MentalMap { 0 } else { 2 }];
                        let mut map_items = Vec::new();
                        use petgraph::Direction;
                        let roots: Vec<_> = s.mental_map.graph.node_indices()
                            .filter(|&idx| s.mental_map.graph.neighbors_directed(idx, Direction::Incoming).count() == 0)
                            .collect();
                        let count = roots.len();
                        let mut visited = std::collections::HashSet::new();

                        for (i, node_idx) in roots.into_iter().enumerate() {
                            render_node_recursive(&s.mental_map.graph, node_idx, 0, "", i == count - 1, &mut map_items, &mut visited);
                        }
                        f.render_widget(List::new(map_items).block(Block::default().title(" MENTAL MAP [3] ").borders(Borders::ALL).border_style(if focus_mode == FocusMode::MentalMap { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) })), area);
                    }

                    // 4. EVENT FEED
                    if focus_mode == FocusMode::None || focus_mode == FocusMode::Feed {
                        let area = middle_chunks[if focus_mode == FocusMode::Feed { 0 } else { 3 }];
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

                    // 5. AI TERMINAL
                    if focus_mode == FocusMode::None || focus_mode == FocusMode::Terminal {
                        let area = middle_chunks[if focus_mode == FocusMode::Terminal { 0 } else { 4 }];
                        let mut ai_content = String::new();
                        for output in &s.ai_outputs {
                            ai_content.push_str(&format!("[{}] {}>\n{}\n", 
                                output.timestamp.format("%H:%M:%S"),
                                output.worker_id,
                                output.content
                            ));
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
                        let area = middle_chunks[if focus_mode == FocusMode::Learnings { 0 } else { 5 }];
                        let mut learning_items = Vec::new();
                        for insight in &s.insights {
                            learning_items.push(ListItem::new(format!(" 󰋗 {}", insight.description)).style(Style::default().fg(Color::Cyan)));
                        }
                        for opt in &s.optimizations {
                            learning_items.push(ListItem::new(format!(" 󰒓 [{}]: {}", opt.target_component, opt.suggestion)).style(Style::default().fg(Color::Magenta)));
                        }
                        if learning_items.is_empty() {
                             let recorded = s.recorded_missions.len();
                             let total = s.missions.len();
                             learning_items.push(ListItem::new(format!(" Gathering experience ({}/{} missions)...", recorded, total)).style(Style::default().fg(Color::DarkGray)));
                        }
                        f.render_widget(
                            List::new(learning_items)
                                .block(Block::default().title(" LEARNINGS [6] ").borders(Borders::ALL).border_style(if focus_mode == FocusMode::Learnings { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) })),
                            area
                        );
                    }

                    // --- Activity Bar ---
                    let has_running_task = s.missions.iter()
                        .flat_map(|m| m.tasks.iter())
                        .any(|t| t.status == TaskStatus::Running);
                    let current_task_name = s.missions.iter()
                        .flat_map(|m| m.tasks.iter())
                        .find(|t| t.status == TaskStatus::Running)
                        .map(|t| t.name.clone())
                        .unwrap_or_else(|| "Idle".to_string());
                    
                    // Pulsing activity indicator for running tasks
                    let activity_prefix = if has_running_task {
                        const ACTIVITY_FRAMES: [&str; 4] = ["●", "◔", "◑", "◕"];
                        format!(" {} ", ACTIVITY_FRAMES[(s.frame_count as usize) % ACTIVITY_FRAMES.len()])
                    } else {
                        " ○ ".to_string()
                    };
                    
                    let last_activity_text = format!("{}[ACTIVITY] Task: {} | Last Event: {} ({}s ago)", activity_prefix, current_task_name, s.last_event_type, last_activity_secs);
                    
                    // Pulsing color when actively processing
                    let activity_color = if has_running_task {
                        let blink_phase = (s.frame_count / 5).is_multiple_of(2);
                        if blink_phase { Color::Cyan } else { Color::LightCyan }
                    } else if last_activity_secs > 30 { 
                        Color::Red 
                    } else if last_activity_secs > 10 { 
                        Color::Yellow 
                    } else { 
                        Color::Green 
                    };
                    
                    let activity_bar = Paragraph::new(last_activity_text)
                        .style(Style::default().fg(activity_color).add_modifier(if has_running_task { Modifier::BOLD } else { Modifier::empty() }))
                        .alignment(Alignment::Left);
                    f.render_widget(activity_bar, main_layout[3]);

                    let footer_text = if s.is_intervening {
                        " [ESC] Cancel | [ENTER] Send | MODE: INTERVENTION"
                    } else if s.show_event_details {
                        " [ESC] Close | MODE: DETAIL VIEW"
                    } else {
                        " [Q] Quit | [SPACE] Pause | [I] Intervene | [1-6] Focus | [J/K] Feed Scroll | [ENTER] Event Details"
                    };
                    let footer = Paragraph::new(footer_text)
                        .style(Style::default().fg(Color::DarkGray))
                        .alignment(Alignment::Center);
                    f.render_widget(footer, main_layout[4]);

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
                _ => {}
            }
        })?;

        // Increment frame counter for animations
        if let Ok(mut s) = state.lock() {
            s.frame_count = s.frame_count.wrapping_add(1);
        }

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
                                    let sid = Uuid::new_v4();
                                    s.current_session_id = Some(sid);
                                    s.mode = AppMode::Running;
                                    s.status = LoopStatus::Running;
                                    
                                    // Use planner to generate missions based on the goal
                                    let bus_m = Arc::clone(&bus);
                                    let orch_m = Arc::clone(&orchestrator);
                                    let goal = s.wizard.goal.clone();
                                    let workspace = if s.wizard.workspace_path.is_empty() { None } else { Some(s.wizard.workspace_path.clone()) };
                                    
                                    tokio::spawn(async move {
                                        // Use planner to dynamically generate missions
                                        let planner = BasicPlanner;
                                        let mut missions = planner.generate_initial_missions(&goal).await;
                                        
                                        // Set session_id and workspace on all generated missions
                                        for mission in &mut missions {
                                            mission.session_id = sid;
                                            mission.workspace_path = workspace.clone();
                                        }
                                        
                                        // Add to orchestrator and publish each mission
                                        for mission in missions {
                                            // Add to orchestrator so background loop can execute
                                            let _ = orch_m.add_mission(mission.clone()).await;
                                            
                                            // Publish event so TUI state updates
                                            let _ = bus_m.publish(Event {
                                                id: Uuid::new_v4(),
                                                session_id: sid,
                                                trace_id: Uuid::new_v4(),
                                                timestamp: Utc::now(),
                                                worker_id: "system".to_string(),
                                                event_type: "MissionCreated".to_string(),
                                                payload: serde_json::to_string(&mission).unwrap(),
                                            }).await;
                                        }
                                    });

                                } else {
                                    let _ = s.wizard.next();
                                }
                            }
                            KeyCode::Backspace => {
                                if s.wizard.current_step == WizardStep::Goal {
                                    s.wizard.goal.pop();
                                } else if s.wizard.current_step == WizardStep::Workspace {
                                    s.wizard.workspace_path.pop();
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
                                } else if s.wizard.current_step == WizardStep::Workspace {
                                    s.wizard.workspace_path.push(c);
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
    
    // Cleanup: Restore terminal to normal state
    crossterm::terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::style::ResetColor
    )?;
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
fn render_node_recursive(graph: &petgraph::graph::DiGraph<relationship::NodeType, ()>, node_idx: petgraph::graph::NodeIndex, depth: usize, prefix: &str, is_last: bool, items: &mut Vec<ratatui::widgets::ListItem>, visited: &mut std::collections::HashSet<petgraph::graph::NodeIndex>) {
    use relationship::NodeType;
    use ratatui::style::{Color, Style, Modifier};
    use petgraph::visit::EdgeRef;

    if visited.contains(&node_idx) {
        return;
    }
    visited.insert(node_idx);

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
            render_node_recursive(graph, child_idx, depth + 1, &new_prefix, child_is_last, items, visited);
        }
    }
}
