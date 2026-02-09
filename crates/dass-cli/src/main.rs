use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::Parser;
use console::style;
use dass_engine::{
    agents::cli_client::ShellCliClient, interaction::UserInteraction, orchestrator::IterationInfo,
    orchestrator::Orchestrator,
};
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
struct Args {
    /// Skip confirmation prompts (auto-accept)
    #[arg(short, long)]
    yolo: bool,

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

    /// Enable debug mode
    #[arg(long)]
    debug: bool,

    /// Enable debug mode specifically for the AI CLI session
    #[arg(long)]
    debug_ai_cli: bool,

    /// Output format (default: "text")
    #[arg(long, default_value = "text")]
    output_format: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AppConfig {
    app_id: String,
    app_name: String,
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
            info!("Feature: {}", q);
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
        if self.args.yolo {
            return Ok(true);
        }
        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .default(true)
            .interact()
            .context("Failed to read confirmation")
    }

    fn start_step(&self, name: &str) {
        info!("Step started: {}", name);
        println!("\n{}", style(name).bold().cyan());
    }

    fn end_step(&self, name: &str) {
        info!("Step ended: {}", name);
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
        info!("{}", msg);
        println!("{}", style(msg).green());
    }

    fn log_error(&self, msg: &str) {
        warn!("{}", msg);
        println!("{}", style(msg).red());
    }
}

fn setup_logging(debug: bool) {
    let filter = if debug {
        EnvFilter::new("info,dass_cli=debug,dass_engine=debug")
    } else {
        EnvFilter::new("warn,dass_cli=info,dass_engine=info")
    };

    fmt::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();
}

async fn ensure_infinite_coding_loop(work_dir: &Path, app_name: &str, app_id: &str) -> Result<()> {
    let icl_dir = work_dir.join(".infinitecodingloop");
    if !icl_dir.exists() {
        tokio::fs::create_dir_all(&icl_dir).await?;
    }

    let app_json_path = icl_dir.join("app.json");
    if !app_json_path.exists() {
        let config = AppConfig {
            app_id: app_id.to_string(),
            app_name: app_name.to_string(),
        };
        let content = serde_json::to_string_pretty(&config)?;
        tokio::fs::write(app_json_path, content).await?;
    }

    // Ensure iterations directory
    tokio::fs::create_dir_all(icl_dir.join("iterations")).await?;

    Ok(())
}

async fn list_iterations(work_dir: &Path) -> Result<Vec<(String, String)>> {
    let mut iterations = Vec::new();
    let iters_dir = work_dir.join(".infinitecodingloop").join("iterations");

    if iters_dir.exists() {
        let mut entries = tokio::fs::read_dir(iters_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                let iter_json = path.join("iteration.json");
                if iter_json.exists() {
                    let content = tokio::fs::read_to_string(iter_json).await?;
                    let info: IterationInfo = serde_json::from_str(&content)?;
                    iterations.push((info.id, info.name));
                }
            }
        }
    }

    Ok(iterations)
}

async fn load_app_config(work_dir: &Path) -> Result<Option<AppConfig>> {
    let app_json_path = work_dir.join(".infinitecodingloop").join("app.json");
    if app_json_path.exists() {
        let content = tokio::fs::read_to_string(app_json_path).await?;
        let config: AppConfig = serde_json::from_str(&content)?;
        Ok(Some(config))
    } else {
        Ok(None)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 1. Setup Logging
    setup_logging(args.debug);

    // 2. Resolve Working Directory
    let work_dir_input: String = if let Some(ref wd) = args.work_dir {
        wd.clone()
    } else {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Application Path")
            .default(".".into())
            .interact_text()?
    };
    let work_dir_path = PathBuf::from(work_dir_input);

    if !work_dir_path.exists() {
        tokio::fs::create_dir_all(&work_dir_path).await?;
    }

    // 3. Try Resume or Create
    let (app_name, final_app_id) = if let Some(config) = load_app_config(&work_dir_path).await? {
        println!(
            "{}",
            style(format!(
                "Resuming Application: {} ({})",
                config.app_name, config.app_id
            ))
            .yellow()
        );
        (config.app_name, config.app_id)
    } else {
        let name: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Application Name")
            .default("MyNewApp".into())
            .interact_text()?;

        let id = if let Some(ref id) = args.app_id {
            id.clone()
        } else {
            uuid::Uuid::new_v4().to_string()
        };
        (name, id)
    };

    // Ensure .infinitecodingloop exists
    ensure_infinite_coding_loop(&work_dir_path, &app_name, &final_app_id).await?;

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
        .with_yolo(args.yolo)
        .with_model(args.model.clone())
        .with_debug(args.debug_ai_cli)
        .with_output_format(args.output_format.clone());

    let mut orchestrator = Orchestrator::new(
        client,
        final_app_id.clone(),
        app_name,
        work_dir_path.clone(),
    )
    .await?;

    let ui = CliInteraction::new(args.clone());

    // 4. Iteration Resumption
    let iterations = list_iterations(&work_dir_path).await?;
    if !iterations.is_empty() {
        let mut options: Vec<String> = iterations
            .iter()
            .map(|(_, name)| format!("Resume: {}", name))
            .collect();
        options.push("Start New Iteration".to_string());

        let selection = ui
            .select_option("Previous iterations found. How to proceed?", &options)
            .await?;

        if selection < iterations.len() {
            let (uuid, name) = &iterations[selection];
            orchestrator.load_iteration(uuid).await?;

            let (done, pending) = orchestrator.get_execution_status();
            println!(
                "\n{}",
                style(format!("ITERATION STATUS: {}", name)).bold().yellow()
            );
            println!("{}:", style("Completed Tasks").green());
            for task in done {
                println!("  {} {}", style("✔").green(), task);
            }
            println!("{}:", style("Pending Tasks").cyan());
            for task in pending {
                println!("  {} {}", style("➜").cyan(), task);
            }
            println!("");
        } else {
            let name: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("New Iteration Name")
                .default("Feature Development".into())
                .interact_text()?;
            orchestrator.start_iteration(&name).await?;
        }
    } else {
        // First run or no iterations
        orchestrator
            .start_iteration("Initial Implementation")
            .await?;
    }

    orchestrator.run(&ui).await?;

    Ok(())
}
