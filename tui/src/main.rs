
use clap::Parser;
use anyhow::Result;
use ifcl_core::{Event, EventBus, InMemoryEventBus, LoopConfig};
use uuid::Uuid;
use chrono::Utc;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    goal: String,

    #[arg(short, long)]
    max_coins: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Starting Infinite Coding Loop...");
    println!("Goal: {}", args.goal);

    let bus = InMemoryEventBus::new(100);
    
    // Feature 01: Emit LoopStarted event
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
    println!("Loop Started event emitted.");

    // Placeholder for TUI Loop
    println!("Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;

    Ok(())
}
