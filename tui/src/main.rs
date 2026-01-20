
use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ifcl_core::{Event, EventBus, EventStore, InMemoryEventBus, InMemoryEventStore, LoopConfig, WorkerProfile, WorkerRole};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
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
    let bus = Arc::new(InMemoryEventBus::new(100));
    let store = Arc::new(InMemoryEventStore::new());
    let state = Arc::new(Mutex::new(AppState {
        events: Vec::new(),
        workers: Vec::new(),
    }));

    // 3. Pipe Bus to State & Store
    let mut rx = bus.subscribe();
    let state_clone = Arc::clone(&state);
    let store_clone = Arc::clone(&store);
    
    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let _ = store_clone.append(event.clone()).await;
            
            // Handle specific events
            if event.event_type == "WorkerJoined" {
                if let Ok(profile) = serde_json::from_str::<WorkerProfile>(&event.payload) {
                    if let Ok(mut s) = state_clone.lock() {
                        s.workers.push(profile);
                    }
                }
            }

            if let Ok(mut s) = state_clone.lock() {
                if s.events.len() > 100 {
                    s.events.remove(0);
                }
                s.events.push(event);
            }
        }
    });

    // 4. Emit Initial Simulation Events
    let bus_clone = Arc::clone(&bus);
    let goal_clone = args.goal.clone();
    let coins_clone = args.max_coins;
    
    tokio::spawn(async move {
        // Loop Started
        let _ = bus_clone.publish(Event {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            worker_id: "system".to_string(),
            event_type: "LoopStarted".to_string(),
            payload: serde_json::to_string(&LoopConfig {
                goal: goal_clone,
                max_coins: coins_clone,
            }).unwrap(),
        }).await;

        // Simulate Workers joining
        let initial_team = vec![
            WorkerProfile { name: "Git-Bot".to_string(), role: WorkerRole::Git, model: None },
            WorkerProfile { name: "Claude".to_string(), role: WorkerRole::Coder, model: Some("claude-3-5".to_string()) },
            WorkerProfile { name: "Search-1".to_string(), role: WorkerRole::Researcher, model: None },
        ];

        for w in initial_team {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let _ = bus_clone.publish(Event {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                worker_id: "system".to_string(),
                event_type: "WorkerJoined".to_string(),
                payload: serde_json::to_string(&w).unwrap(),
            }).await;
        }
    });

    // 5. Main TUI Loop
    loop {
        terminal.draw(|f| {
            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(0),
                        Constraint::Length(1),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            // Header - Cyber Blue
            let header = Paragraph::new(format!(" OBJECTIVE: {}", goal_display))
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .block(Block::default()
                    .title(" INFINITE CODING LOOP [v0.1.0] ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)));
            f.render_widget(header, main_layout[0]);

            // Middle Layout: Sidebar + Feed
            let middle_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(25), // Roster
                        Constraint::Percentage(75), // Feed
                    ]
                    .as_ref(),
                )
                .split(main_layout[1]);

            // ROSTER Panel
            let mut worker_items: Vec<ListItem> = Vec::new();
            if let Ok(state_lock) = state.lock() {
                worker_items = state_lock.workers.iter().map(|w| {
                    let symbol = match w.role {
                        WorkerRole::Git => "󰊢",
                        WorkerRole::Coder => "󰅩",
                        WorkerRole::Researcher => "󰍉",
                        _ => "󰚩",
                    };
                    ListItem::new(format!(" {} {} ({:?})", symbol, w.name, w.role))
                        .style(Style::default().fg(Color::Yellow))
                }).collect();
            }
            let roster = List::new(worker_items)
                .block(Block::default()
                    .title(" BARRACKS / ROSTER ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)));
            f.render_widget(roster, middle_chunks[0]);

            // FEED Panel
            let mut feed_items: Vec<ListItem> = Vec::new();
            if let Ok(state_lock) = state.lock() {
                feed_items = state_lock.events.iter().rev().map(|e| {
                    let color = match e.event_type.as_str() {
                        "LoopStarted" => Color::Green,
                        "WorkerJoined" => Color::Blue,
                        _ => Color::White,
                    };
                    
                    ListItem::new(format!(
                        " {:<10} | {:<15} | {}",
                        e.timestamp.format("%H:%M:%S"),
                        e.event_type,
                        e.payload
                    )).style(Style::default().fg(color))
                }).collect();
            }
            let list = List::new(feed_items)
                .block(Block::default()
                    .title(" LIVE ACTIVITY FEED ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)));
            f.render_widget(list, middle_chunks[1]);

            // Footer
            let footer = Paragraph::new(" [Q] Quit | [SPACE] Pause (N/A) | System Status: ONLINE")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(footer, main_layout[2]);
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let CEvent::Key(key) = event::read()? {
                if let KeyCode::Char('q') = key.code {
                    break;
                }
            }
        }
    }

    // 6. Cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
