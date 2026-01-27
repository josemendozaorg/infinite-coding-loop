use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::Parser;
use console::style;
use dass_engine::{
    agents::cli_client::ShellCliClient, interaction::UserInteraction, orchestrator::Orchestrator,
    plan::action::ImplementationPlan, product::requirement::Requirement,
    spec::feature_spec::FeatureSpec,
};
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
struct Args {
    /// Skip confirmation prompts (auto-accept)
    #[arg(short, long)]
    yes: bool,

    /// Executable to use for AI calls (default: "gemini")
    #[arg(long, default_value = "gemini")]
    ai_cmd: String,

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

    fn render_requirements(&self, reqs: &[Requirement]) {
        println!("\n{}:", style("Requirements").bold().green());
        for req in reqs {
            println!("  • {}", req.user_story);
        }
    }

    fn render_spec(&self, spec: &FeatureSpec) {
        println!("\n{}:", style("Feature Spec").bold().green());
        println!("  ID: {}", spec.id);
        println!("  UI Logic: {} chars", spec.ui_spec.len());
    }

    fn render_plan(&self, plan: &ImplementationPlan) {
        println!("\n{}:", style("Implementation Plan").bold().green());
        for (i, step) in plan.steps.iter().enumerate() {
            println!("  {}. {:?}", i + 1, step);
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
    let mut args = Args::parse();

    // 1. Resolve Application ID
    let final_app_id = if let Some(id) = args.app_id.clone() {
        id
    } else {
        let available = Orchestrator::<ShellCliClient>::list_available_apps()
            .await
            .unwrap_or_default();
        if available.is_empty() {
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Application ID (new)")
                .default("default-app".into())
                .interact_text()?
        } else {
            let mut items: Vec<String> = available
                .iter()
                .map(|(id, name, work_dir)| {
                    let dir = work_dir.as_deref().unwrap_or("No path");
                    format!("{} ({}) [{}]", name, id, style(dir).dim())
                })
                .collect();
            items.push(style("Create New Application").yellow().to_string());

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select an application")
                .items(&items)
                .default(0)
                .interact()?;

            if selection < available.len() {
                available[selection].0.clone()
            } else {
                Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Application ID (new)")
                    .interact_text()?
            }
        }
    };
    args.app_id = Some(final_app_id.clone());

    // 2. Resolve Working Directory
    let stored_work_dir = Orchestrator::<ShellCliClient>::get_app_work_dir(&final_app_id)
        .await
        .unwrap_or_default();

    if args.work_dir.is_none() {
        let default_dir = stored_work_dir.unwrap_or_else(|| ".".to_string());
        let path: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Output directory")
            .default(default_dir)
            .interact_text()?;
        args.work_dir = Some(path);
    }

    let work_dir = std::path::PathBuf::from(args.work_dir.as_ref().unwrap());

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
    let client = ShellCliClient::new(&args.ai_cmd);

    let mut orchestrator = Orchestrator::new(
        client,
        final_app_id.clone(),
        format!("App: {}", final_app_id),
        work_dir,
    )
    .await?;

    let ui = CliInteraction::new(args.clone());

    orchestrator.run(&ui).await?;

    Ok(())
}
