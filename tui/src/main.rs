
use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ifcl_core::{
    Event, EventBus, EventStore, InMemoryEventBus, InMemoryEventStore, LoopConfig, 
    WorkerProfile, WorkerRole, Mission, TaskStatus, Task, CliExecutor, Bank, LoopStatus
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Table, Row, Cell},
    Terminal,
};
use std::io;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    goal: String,

    #[arg(short, long)]
    max_coins: Option<u64>,
}

struct AppState {
    events: Vec<Event>,
    workers: Vec<WorkerProfile>,
    missions: Vec<Mission>,
    bank: Bank,
    status: LoopStatus,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let goal_display = args.goal.clone();

    // 1. Setup Terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 2. Initialize Infrastructure
    let bus = Arc::new(InMemoryEventBus::new(200));
    let store = Arc::new(InMemoryEventStore::new());
    let state = Arc::new(Mutex::new(AppState {
        events: Vec::new(),
        workers: Vec::new(),
        missions: Vec::new(),
        bank: Bank::default(),
        status: LoopStatus::Running,
    }));

    // 3. Pipe Bus to State & Store
    let mut rx = bus.subscribe();
    let state_clone = Arc::clone(&state);
    let store_clone = Arc::clone(&store);
    let bus_reward = Arc::clone(&bus);
    
    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let _ = store_clone.append(event.clone()).await;
            
            if let Ok(mut s) = state_clone.lock() {
                match event.event_type.as_str() {
                    "WorkerJoined" => {
                        if let Ok(profile) = serde_json::from_str::<WorkerProfile>(&event.payload) {
                            s.workers.push(profile);
                        }
                    }
                    "MissionCreated" => {
                        if let Ok(mission) = serde_json::from_str::<Mission>(&event.payload) {
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
                                         tokio::spawn(async move {
                                             let _ = bus_r.publish(Event {
                                                 id: Uuid::new_v4(), timestamp: Utc::now(), worker_id: "system".to_string(),
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
                    "LoopStatusChanged" => {
                        if let Ok(status) = serde_json::from_str::<LoopStatus>(&event.payload) {
                            s.status = status;
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
    let goal_simulation = args.goal.clone();
    let coins_simulation = args.max_coins;
    let state_sim_monitor = Arc::clone(&state);
    
    tokio::spawn(async move {
        // Helper to check pause state
        let check_pause = || {
            loop {
                if let Ok(s) = state_sim_monitor.lock() {
                    if s.status == LoopStatus::Running { break; }
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        };

        // Step 1: Loop Started
        let _ = bus_simulation.publish(Event {
            id: Uuid::new_v4(), timestamp: Utc::now(), worker_id: "system".to_string(),
            event_type: "LoopStarted".to_string(),
            payload: serde_json::to_string(&LoopConfig { goal: goal_simulation, max_coins: coins_simulation }).unwrap(),
        }).await;

        // Step 2: Workers Joined
        let workers = vec![
            WorkerProfile { name: "Architect".to_string(), role: WorkerRole::Architect, model: Some("gemini".to_string()) },
            WorkerProfile { name: "Git-Bot".to_string(), role: WorkerRole::Git, model: None },
        ];
        for w in workers {
            check_pause();
            tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
            let _ = bus_simulation.publish(Event {
                id: Uuid::new_v4(), timestamp: Utc::now(), worker_id: "system".to_string(),
                event_type: "WorkerJoined".to_string(), payload: serde_json::to_string(&w).unwrap(),
            }).await;
        }

        // Step 3: Mission Created
        check_pause();
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
            id: Uuid::new_v4(), timestamp: Utc::now(), worker_id: "Architect".to_string(),
            event_type: "MissionCreated".to_string(), payload: serde_json::to_string(&mission).unwrap(),
        }).await;

        // Step 4: Execute Real Gemini Call
        check_pause();
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        let _ = bus_simulation.publish(Event {
            id: Uuid::new_v4(), timestamp: Utc::now(), worker_id: "Architect".to_string(),
            event_type: "TaskUpdated".to_string(),
            payload: format!(r#"{{"mission_id":"{}","task_id":"{}","status":"Running"}}"#, mission_id, t1_id),
        }).await;

        let _ = bus_simulation.publish(Event {
            id: Uuid::new_v4(), timestamp: Utc::now(), worker_id: "Architect".to_string(),
            event_type: "Log".to_string(), payload: "Invoking Gemini CLI...".to_string(),
        }).await;

        let executor = CliExecutor::new("gemini".to_string());
        let prompt = "Explain Rust ownership in 1 short sentence.";
        
        match executor.execute(prompt).await {
            Ok(response) => {
                check_pause();
                let _ = bus_simulation.publish(Event {
                    id: Uuid::new_v4(), timestamp: Utc::now(), worker_id: "Architect".to_string(),
                    event_type: "AiResponse".to_string(), payload: response,
                }).await;
                let _ = bus_simulation.publish(Event {
                    id: Uuid::new_v4(), timestamp: Utc::now(), worker_id: "Architect".to_string(),
                    event_type: "TaskUpdated".to_string(),
                    payload: format!(r#"{{"mission_id":"{}","task_id":"{}","status":"Success"}}"#, mission_id, t1_id),
                }).await;
            }
            Err(e) => {
                let _ = bus_simulation.publish(Event {
                    id: Uuid::new_v4(), timestamp: Utc::now(), worker_id: "Architect".to_string(),
                    event_type: "WorkerError".to_string(), payload: e.to_string(),
                }).await;
            }
        }
    });

    // 5. Main TUI Loop
    loop {
        terminal.draw(|f| {
            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)].as_ref())
                .split(f.size());

            // Header with Bank Info
            let (xp, coins, loop_status) = if let Ok(s) = state.lock() {
                (s.bank.xp, s.bank.coins, s.status)
            } else { (0, 0, LoopStatus::Running) };

            let header_content = format!(" OBJECTIVE: {:<35} | XP: {:<5} | COINS: {:<5} | STATUS: {:?}", goal_display, xp, coins, loop_status);
            let header_color = match loop_status {
                LoopStatus::Running => Color::Cyan,
                LoopStatus::Paused => Color::Yellow,
            };

            let header = Paragraph::new(header_content)
                .style(Style::default().fg(header_color).add_modifier(Modifier::BOLD))
                .block(Block::default().title(" INFINITE CODING LOOP [v0.1.0] ").borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
            f.render_widget(header, main_layout[0]);

            // Middle Layout
            let middle_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(20), Constraint::Percentage(45), Constraint::Percentage(35)].as_ref())
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

            // 3. EVENT FEED
            let mut feed_items = Vec::new();
            if let Ok(s) = state.lock() {
                feed_items = s.events.iter().rev().map(|e| {
                    let color = match e.event_type.as_str() { 
                        "LoopStarted" => Color::Green, 
                        "WorkerJoined" => Color::Blue, 
                        "AiResponse" => Color::Yellow,
                        "RewardEarned" => Color::Green,
                        "LoopStatusChanged" => Color::Yellow,
                        "WorkerError" => Color::Red,
                        _ => Color::White 
                    };
                    let content = if e.event_type == "AiResponse" {
                        format!(" > AI: {}", e.payload)
                    } else if e.event_type == "RewardEarned" {
                         format!(" + REWARD: {}", e.payload)
                    } else if e.event_type == "LoopStatusChanged" {
                         format!(" # STATUS: {}", e.payload)
                    } else if e.event_type == "WorkerError" {
                         format!(" ! ERR: {}", e.payload)
                    } else {
                        format!(" {:<8} | {}", e.timestamp.format("%H:%M:%S"), e.event_type)
                    };
                    ListItem::new(content).style(Style::default().fg(color))
                }).collect();
            }
            f.render_widget(List::new(feed_items).block(Block::default().title(" ACTIVITY FEED ").borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray))), middle_chunks[2]);

            // Footer
            let footer_text = if loop_status == LoopStatus::Running {
                " [Q] Quit | [SPACE] Pause | System Status: ONLINE"
            } else {
                " [Q] Quit | [SPACE] Resume | System Status: PAUSED"
            };
            f.render_widget(Paragraph::new(footer_text).style(Style::default().fg(header_color)), main_layout[2]);
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let CEvent::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char(' ') => {
                        if let Ok(s) = state.lock() {
                            let new_status = match s.status {
                                LoopStatus::Running => LoopStatus::Paused,
                                LoopStatus::Paused => LoopStatus::Running,
                            };
                            let bus_p = Arc::clone(&bus);
                            tokio::spawn(async move {
                                let _ = bus_p.publish(Event {
                                    id: Uuid::new_v4(), timestamp: Utc::now(), worker_id: "user".to_string(),
                                    event_type: "LoopStatusChanged".to_string(),
                                    payload: serde_json::to_string(&new_status).unwrap(),
                                }).await;
                            });
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
