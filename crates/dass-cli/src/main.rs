use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::Parser;
use console::style;
use dass_engine::{
    agents::cli_client::ShellCliClient, interaction::UserInteraction, orchestrator::Orchestrator,
};
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use serde_json::Value;

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
struct Args {
    /// Skip confirmation prompts (auto-accept)
    #[arg(short, long)]
    yes: bool,

    /// Model to use (default: "gemini-2.5-flash")
    #[arg(long, default_value = "gemini-2.5-flash")]
    model: String,

    /// Feature idea (skips input prompt)
    query: Option<String>,

    /// Application ID for persistence
    #[arg(short, long)]
    app_id: Option<String>,

    /// Working directory for code generation
    #[arg(short, long)]
    work_dir: Option<String>,
}

struct CliInteraction {
    args: Args,
}

impl CliInteraction {
    fn new(args: Args) -> Self {
        Self { args }
    }
}

#[async_trait]
impl UserInteraction for CliInteraction {
    async fn ask_user(&self, prompt: &str) -> Result<String> {
        let input = Input::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .interact_text()?;
        Ok(input)
    }

    async fn ask_for_feature(&self, prompt: &str) -> Result<String> {
        if let Some(q) = &self.args.query {
            println!("Feature: {}", style(q).cyan());
            return Ok(q.clone());
        }
        self.ask_user(prompt).await
    }

    async fn select_option(&self, prompt: &str, options: &[String]) -> Result<usize> {
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .items(options)
            .default(0)
            .interact()?;
        Ok(selection)
    }

    async fn confirm(&self, prompt: &str) -> Result<bool> {
        if self.args.yes {
            return Ok(true);
        }
        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .default(true)
            .interact()
            .context("Failed to read confirmation")
    }

    fn start_step(&self, name: &str) {
        println!("\n{}", style(name).bold().cyan());
    }

    fn end_step(&self, _name: &str) {
        // Could be used for timing or spinner cleanup if we managed global state
    }

    fn render_artifact(&self, kind: &str, data: &Value) {
        println!("\n{}:", style(kind).bold().green());
        if let Some(obj) = data.as_object() {
            for (key, val) in obj {
                if val.is_string() {
                    println!("  {}: {}", style(key).dim(), val.as_str().unwrap());
                } else if val.is_array() {
                    println!("  {}:", style(key).dim());
                    for item in val.as_array().unwrap() {
                        println!("    • {:?}", item);
                    }
                } else {
                    println!("  {}: {:?}", style(key).dim(), val);
                }
            }
        } else {
            println!("  {:?}", data);
        }
    }

    fn log_info(&self, msg: &str) {
        println!("{}", style(msg).green());
    }

    fn log_error(&self, msg: &str) {
        println!("{}", style(msg).red());
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 2. Create New Application
    let app_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Application Name")
        .default("MyNewApp".into())
        .interact_text()?;

    let final_app_id = if let Some(ref id) = args.app_id {
        id.clone()
    } else {
        uuid::Uuid::new_v4().to_string()
    };

    // Resolve Working Directory
    let work_dir_path = if let Some(ref wd) = args.work_dir {
        std::path::PathBuf::from(wd)
    } else {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        std::env::current_dir()?
            .join("tmp")
            .join(format!("{}_{}", app_name, timestamp))
    };

    if !work_dir_path.exists() {
        tokio::fs::create_dir_all(&work_dir_path).await?;
    }

    // Banner
    println!(
        "{}",
        style("   DASS SOFTWARE FACTORY   ")
            .bold()
            .on_blue()
            .white()
    );
    println!("{}", style("---------------------------").dim());

    println!("{}", style("Running in LIVE MODE (calling AI CLI)").green());
    let client = ShellCliClient::new("gemini", work_dir_path.to_string_lossy().to_string())
        .with_yolo(args.yes)
        .with_model(args.model.clone());

    let mut orchestrator =
        Orchestrator::new(client, final_app_id.clone(), app_name, work_dir_path).await?;

    let ui = CliInteraction::new(args.clone());

    orchestrator.run(&ui).await?;

    Ok(())
}
