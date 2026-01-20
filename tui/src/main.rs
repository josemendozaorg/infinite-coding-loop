
use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ifcl_core::{Event, EventBus, EventStore, InMemoryEventBus, InMemoryEventStore, LoopConfig};
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
    }));

    // 3. Pipe Bus to State & Store
    let mut rx = bus.subscribe();
    let state_clone = Arc::clone(&state);
    let store_clone = Arc::clone(&store);
    
    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let _ = store_clone.append(event.clone()).await;
            if let Ok(mut s) = state_clone.lock() {
                if s.events.len() > 100 {
                    s.events.remove(0);
                }
                s.events.push(event);
            }
        }
    });

    // 4. Emit Initial Event
    let start_event = Event {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        worker_id: "system".to_string(),
        event_type: "LoopStarted".to_string(),
        payload: serde_json::to_string(&LoopConfig {
            goal: args.goal,
            max_coins: args.max_coins,
        })?,
    };
    bus.publish(start_event).await?;

    // 5. Main TUI Loop
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
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
            f.render_widget(header, chunks[0]);

            // Events List
            let mut items: Vec<ListItem> = Vec::new();
            if let Ok(state_lock) = state.lock() {
                items = state_lock.events.iter().rev().map(|e| {
                    let color = match e.event_type.as_str() {
                        "LoopStarted" => Color::Green,
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

            let list = List::new(items)
                .block(Block::default()
                    .title(" LIVE ACTIVITY FEED ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)));
            f.render_widget(list, chunks[1]);

            // Footer
            let footer = Paragraph::new(" [Q] Quit | [SPACE] Pause (N/A) | System Status: ONLINE")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(footer, chunks[2]);
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
