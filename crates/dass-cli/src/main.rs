use clap::Parser;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use console::{style, Emoji};
use dass_engine::{
    agents::{
        cli_client::ShellCliClient, product_manager::ProductManager, architect::Architect,
        planner::Planner,
    },
    product::requirement::Requirement,
    spec::feature_spec::FeatureSpec,
    plan::action::ImplementationPlan,
};
use std::fs;
use anyhow::{Result, Context};
use tokio::time::{sleep, Duration};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Skip confirmation prompts (auto-accept)
    #[arg(short, long)]
    yes: bool,

    /// Executable to use for AI calls (default: "gemini")
    #[arg(long, default_value = "gemini")]
    ai_cmd: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Banner
    println!("{}", style("   DASS SOFTWARE FACTORY   ").bold().on_blue().white());
    println!("{}", style("---------------------------").dim());

    // 1. Setup Agents
    let client = ShellCliClient::new(&args.ai_cmd);
    // Note: In real Rust ownership, we might need shared client or clones. 
    // ShellCliClient is lightweight (just a String), so cloning is fine.
    
    let pm = ProductManager::new(ShellCliClient::new(&args.ai_cmd));
    let architect = Architect::new(ShellCliClient::new(&args.ai_cmd));
    let planner = Planner::new(ShellCliClient::new(&args.ai_cmd));

    // 2. User Input
    let feature_idea: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What feature do you want to build?")
        .interact_text()?;

    // 3. Product Phase
    step_header("1. PRODUCT MANAGER: Analyzing Request...");
    let reqs = spin("Thinking...", || pm.process_request(&feature_idea))?;
    
    // Display Reqs
    println!("\n{}:", style("Generated Requirements").bold().green());
    for req in &reqs {
        println!("  • {}", req.user_story);
    }
    
    if !confirm_continue(&args)? {
        println!("{}", style("Aborted.").red());
        return Ok(());
    }

    // 4. Architect Phase
    step_header("2. ARCHITECT: Designing Spec...");
    let spec = spin("Designing...", || architect.design("new-feature", &reqs))?;

    // Display Spec Summary
    println!("\n{}:", style("Generated Feature Spec").bold().green());
    println!("  ID: {}", spec.id);
    println!("  UI Logic: {} chars", spec.ui_spec.len());
    
    if !confirm_continue(&args)? {
        println!("{}", style("Aborted.").red());
        return Ok(());
    }

    // 5. Planner Phase
    step_header("3. PLANNER: Creating Plan...");
    let plan = spin("Planning...", || planner.plan(&spec))?;

    // Display Plan
    println!("\n{}:", style("Implementation Plan").bold().green());
    for (i, step) in plan.steps.iter().enumerate() {
        println!("  {}. {:?}", i + 1, step);
    }

    if !confirm_continue(&args)? {
        println!("{}", style("Aborted.").red());
        return Ok(());
    }

    // 6. Execution (Mock for now, or real via PlanRunner)
    step_header("4. CONSTRUCTION: Executing...");
    println!("{}", style("Plan Execution not yet fully wired to file system.").yellow());
    println!("{}", style("Success! Pipeline Complete.").bold().green());

    Ok(())
}

fn step_header(text: &str) {
    println!("\n{}", style(text).bold().cyan());
}

fn spin<F, T>(msg: &str, f: F) -> Result<T> 
where F: FnOnce() -> Result<T> {
    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_message(msg.to_string());
    spinner.enable_steady_tick(Duration::from_millis(100));
    
    let res = f();
    
    spinner.finish_and_clear();
    res
}

fn confirm_continue(args: &Args) -> Result<bool> {
    if args.yes {
        return Ok(true);
    }
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Proceed?")
        .default(true)
        .interact()
        .context("Failed to read confirmation")
}
