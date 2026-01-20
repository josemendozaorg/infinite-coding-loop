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
    Event, EventBus, EventStore, InMemoryEventBus, Mission, TaskStatus, Task, 
    CliExecutor, Bank, LoopStatus, SqliteEventStore, WorkerProfile, WorkerRole, LoopConfig,
    MarketplaceLoader, AppMode, MenuAction, MenuState, SetupWizard, WizardStep
};
use petgraph::visit::EdgeRef;
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
    }));

    // Replay history will be handled later

    // 3. Subscription & Event Processing
    let bus_c = Arc::clone(&bus);
    let state_c = Arc::clone(&state);
    let store_c = Arc::clone(&store);
    let bus_reward = Arc::clone(&bus); // Keep this for the reward publishing inside the loop
    
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
        // Wait for Wizard to complete
        check_pause_async(Arc::clone(&state_sim_monitor)).await;

        let (final_goal, final_budget, sid) = {
            let s = state_sim_monitor.lock().unwrap();
            (s.wizard.goal.clone(), s.wizard.budget_coins, s.current_session_id.unwrap_or_default())
        };

        // Step 1: Loop Started with Wizard Config
        let _ = bus_simulation.publish(Event {
            id: Uuid::new_v4(),
            session_id: sid,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            worker_id: "system".to_string(),
            event_type: "LoopStarted".to_string(),
            payload: serde_json::to_string(&LoopConfig { goal: final_goal.clone(), max_coins: Some(final_budget) }).unwrap(),
        }).await;

        // Step 2: Workers Joined
        let workers = vec![
            WorkerProfile { name: "Architect".to_string(), role: WorkerRole::Architect, model: Some("gemini".to_string()) },
            WorkerProfile { name: "Git-Bot".to_string(), role: WorkerRole::Git, model: None },
        ];
        for w in workers {
            check_pause_async(Arc::clone(&state_sim_monitor)).await;
            tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
            let _ = bus_simulation.publish(Event {
                id: Uuid::new_v4(),
                session_id: sid,
                trace_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                worker_id: "system".to_string(),
                event_type: "WorkerJoined".to_string(), payload: serde_json::to_string(&w).unwrap(),
            }).await;
        }

        // Step 3: Mission Created
        check_pause_async(Arc::clone(&state_sim_monitor)).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
        let t1_id = Uuid::new_v4();
        let t2_id = Uuid::new_v4();
        let mission = Mission {
            id: Uuid::new_v4(),
            name: "Phase 1: Analysis".to_string(),
            tasks: vec![
                Task { id: t1_id, name: "Consult Gemini".to_string(), description: "Ask for greeting".to_string(), status: TaskStatus::Pending, assigned_worker: Some("Architect".to_string()) },
                Task { id: t2_id, name: "Init Repo".to_string(), description: "Setup git".to_string(), status: TaskStatus::Pending, assigned_worker: Some("Git-Bot".to_string()) },
            ],
        };
        let mission_id = mission.id;
        let _ = bus_simulation.publish(Event {
            id: Uuid::new_v4(),
            session_id: sid,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            worker_id: "Architect".to_string(),
            event_type: "MissionCreated".to_string(), payload: serde_json::to_string(&mission).unwrap(),
        }).await;

        // Step 4: Execute Real Gemini Call
        check_pause_async(Arc::clone(&state_sim_monitor)).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        let _ = bus_simulation.publish(Event {
            id: Uuid::new_v4(),
            session_id: sid,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            worker_id: "Architect".to_string(),
            event_type: "TaskUpdated".to_string(),
            payload: format!(r#"{{"mission_id":"{}","task_id":"{}","status":"Running"}}"#, mission_id, t1_id),
        }).await;

        let _ = bus_simulation.publish(Event {
            id: Uuid::new_v4(),
            session_id: sid,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            worker_id: "Architect".to_string(),
            event_type: "Log".to_string(), payload: "Invoking Gemini CLI...".to_string(),
        }).await;

        let executor = CliExecutor::new("gemini".to_string());
        let prompt = format!("Explain the goal '{}' in 1 short sentence.", final_goal);
        
        match executor.execute(&prompt).await {
            Ok(response) => {
                let _ = bus_simulation.publish(Event {
                    id: Uuid::new_v4(),
                    session_id: sid,
                    trace_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    worker_id: "Architect".to_string(),
                    event_type: "AiResponse".to_string(), payload: response,
                }).await;
                let _ = bus_simulation.publish(Event {
                    id: Uuid::new_v4(),
                    session_id: sid,
                    trace_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    worker_id: "Architect".to_string(),
                    event_type: "TaskUpdated".to_string(),
                    payload: format!(r#"{{"mission_id":"{}","task_id":"{}","status":"Success"}}"#, mission_id, t1_id),
                }).await;
            }
            Err(e) => {
                let _ = bus_simulation.publish(Event {
                    id: Uuid::new_v4(),
                    session_id: sid,
                    trace_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    worker_id: "Architect".to_string(),
                    event_type: "WorkerError".to_string(), payload: e.to_string(),
                }).await;
            }
        }

        // Step 5: Git-Bot Completion
        check_pause_async(Arc::clone(&state_sim_monitor)).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
        let _ = bus_simulation.publish(Event {
            id: Uuid::new_v4(),
            session_id: sid,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            worker_id: "Git-Bot".to_string(),
            event_type: "TaskUpdated".to_string(),
            payload: format!(r#"{{"mission_id":"{}","task_id":"{}","status":"Running"}}"#, mission_id, t2_id),
        }).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        let _ = bus_simulation.publish(Event {
            id: Uuid::new_v4(),
            session_id: sid,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            worker_id: "Git-Bot".to_string(),
            event_type: "Log".to_string(), payload: "git init && git add . && git commit -m 'Initial commit'".to_string(),
        }).await;
        let _ = bus_simulation.publish(Event {
            id: Uuid::new_v4(),
            session_id: sid,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            worker_id: "Git-Bot".to_string(),
            event_type: "TaskUpdated".to_string(),
            payload: format!(r#"{{"mission_id":"{}","task_id":"{}","status":"Success"}}"#, mission_id, t2_id),
        }).await;
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

                    let (step, goal, stack, team, budget) = if let Ok(s) = state.lock() {
                        (s.wizard.current_step.clone(), s.wizard.goal.clone(), s.wizard.stack.clone(), s.wizard.team_size, s.wizard.budget_coins)
                    } else { (WizardStep::Goal, String::new(), String::new(), 0, 0) };

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
                        WizardStep::Team => format!("Desired Team Size:\n\n [ {} ] Workers", team),
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

                    let (xp, coins, loop_status, is_int, active_goal) = if let Ok(s) = state.lock() {
                        (s.bank.xp, s.bank.coins, s.status, s.is_intervening, s.wizard.goal.clone())
                    } else { (0, 0, LoopStatus::Running, false, String::new()) };

                    let header_content = format!(" OBJECTIVE: {:<35} | XP: {:<5} | COINS: {:<5} | STATUS: {:?}", active_goal, xp, coins, loop_status);
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
                            Constraint::Percentage(23), 
                            Constraint::Percentage(20), 
                            Constraint::Percentage(20),
                            Constraint::Percentage(25)
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
                        use relationship::NodeType;
                        for node_idx in s.mental_map.graph.node_indices() {
                            let node = &s.mental_map.graph[node_idx];
                            if let NodeType::Mission(name) = node {
                                map_items.push(ListItem::new(format!("󰚒 {}", name)).style(Style::default().fg(Color::Cyan)));
                                // Find tasks
                                for edge in s.mental_map.graph.edges(node_idx) {
                                    let target_idx = edge.target();
                                    if let NodeType::Task(t_name) = &s.mental_map.graph[target_idx] {
                                        map_items.push(ListItem::new(format!("  └─󰓅 {}", t_name)).style(Style::default().fg(Color::DarkGray)));
                                        // Find workers
                                        for w_edge in s.mental_map.graph.edges(target_idx) {
                                            let w_idx = w_edge.target();
                                            if let NodeType::Worker(w_name) = &s.mental_map.graph[w_idx] {
                                                map_items.push(ListItem::new(format!("      └─󰚩 {}", w_name)).style(Style::default().fg(Color::Yellow)));
                                            }
                                        }
                                    }
                                }
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
                                _ => Color::White 
                            };
                            let content = if e.event_type == "AiResponse" {
                                format!(" > AI: ...") 
                            } else if e.event_type == "RewardEarned" {
                                 format!(" + REWARD: {}", e.payload)
                            } else if e.event_type == "LoopStatusChanged" {
                                 format!(" # STATUS: {}", e.payload)
                            } else if e.event_type == "Log" {
                                 format!(" * LOG: {}", e.payload)
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
                                }
 else {
                                    let _ = s.wizard.next();
                                }
                            }
                            KeyCode::Backspace => s.wizard.prev(),
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
