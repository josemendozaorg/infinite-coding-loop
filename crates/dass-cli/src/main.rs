use clap::Parser;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use console::style;
use dass_engine::{
    agents::{
        cli_client::{ShellCliClient, MockCliClient, AiCliClient}, 
        product_manager::ProductManager, 
        architect::Architect,
        planner::Planner,
    },
    spec::feature_spec::FeatureSpec,
};
use anyhow::{Result, Context};
use tokio::time::Duration;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Skip confirmation prompts (auto-accept)
    #[arg(short, long)]
    yes: bool,

    /// Executable to use for AI calls (default: "gemini")
    #[arg(long, default_value = "gemini")]
    ai_cmd: String,

    /// Run in Mock Mode (Simulated responses)
    #[arg(long)]
    mock: bool,

    /// Feature idea (skips input prompt)
    query: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Banner
    println!("{}", style("   DASS SOFTWARE FACTORY   ").bold().on_blue().white());
    println!("{}", style("---------------------------").dim());

    if args.mock {
        println!("{}", style("Running in MOCK MODE").yellow());
        let mut responses = Vec::new();
        
        // 1. Reqs Refinement (Failure then Success)
        responses.push(r#"
- id: 00000000-0000-0000-0000-000000000001
  user_story: 'Make it fast'
  acceptance_criteria: []
"#.to_string()); // Ambiguous, should fail Gate

        responses.push(r#"
- id: 00000000-0000-0000-0000-000000000001
  user_story: 'As a user I want to see a spinner'
  acceptance_criteria: ['Spinner visible within 100ms']
"#.to_string()); // Good

        // 2. Spec Response
        responses.push(serde_json::to_string(&FeatureSpec {
            id: "new-feature".to_string(),
            requirement_ids: vec![uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()],
            ui_spec: "Spinner".to_string(),
            logic_spec: "Show spinner".to_string(),
            data_spec: "None".to_string(),
            verification_plan: "Test".to_string(),
        }).unwrap());

        // 3. Plan Response
        responses.push(r#"
{
  "feature_id": "new-feature",
  "steps": [
     { "type": "RunCommand", "payload": { "command": "echo Hello", "cwd": null, "must_succeed": true } }
  ]
}
"#.to_string());

        let mock_client = MockCliClient::new(responses);
        run_pipeline(mock_client, &args)?;
    } else {
        println!("{}", style("Running in LIVE MODE (calling AI CLI)").green());
        let client = ShellCliClient::new(&args.ai_cmd);
        run_pipeline(client, &args)?;
    }

    Ok(())
}

fn run_pipeline<C: AiCliClient + Clone>(client: C, args: &Args) -> Result<()> {
    // 1. Setup Agents
    let pm = ProductManager::new(client.clone());
    let architect = Architect::new(client.clone());
    let planner = Planner::new(client.clone());

    // 2. User Input
    let feature_idea = if let Some(q) = &args.query {
        println!("Feature: {}", style(q).cyan());
        q.clone()
    } else {
         Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What feature do you want to build?")
        .interact_text()?
    };

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
