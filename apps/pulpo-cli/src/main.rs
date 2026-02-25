use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::Parser;
use console::style;
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use pulpo_engine::{
    agents::cli_client::ShellCliClient,
    config::{self, IclConfig},
    interaction::UserInteraction,
    orchestrator::{IterationInfo, Orchestrator},
};
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

    /// Model to use (if not specified, prompts for selection)
    #[arg(long)]
    model: Option<String>,

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

    /// Maximum number of iterations (default: 100)
    #[arg(long, default_value = "100")]
    max_iterations: usize,

    /// Path to search for ontologies (default: current directory)
    #[arg(long)]
    ontology_path: Option<String>,
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
        EnvFilter::new("info,pulpo_cli=debug,pulpo_engine=debug")
    } else {
        EnvFilter::new("warn,pulpo_cli=info,pulpo_engine=info")
    };

    fmt::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();
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

/// Recursively discover directories containing an `ontology.json` file.
/// Returns a sorted list of parent directory paths.
async fn discover_ontologies(base_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut results = Vec::new();
    discover_ontologies_recursive(base_dir, &mut results).await?;
    results.sort();
    Ok(results)
}

async fn discover_ontologies_recursive(dir: &Path, results: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|n| n.to_str()) == Some("ontology.json") {
            results.push(dir.to_path_buf());
        } else if path.is_dir() {
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // Skip hidden dirs, node_modules, target, dist, .git
            if !dir_name.starts_with('.')
                && dir_name != "node_modules"
                && dir_name != "target"
                && dir_name != "dist"
            {
                Box::pin(discover_ontologies_recursive(&path, results)).await?;
            }
        }
    }

    Ok(())
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
    let base_work_dir = PathBuf::from(work_dir_input);

    if !base_work_dir.exists() {
        tokio::fs::create_dir_all(&base_work_dir).await?;
    }

    // Banner
    println!(
        "\n{}",
        style("   DASS SOFTWARE FACTORY   ")
            .bold()
            .on_blue()
            .white()
    );
    println!("{}", style("---------------------------").dim());

    // 3. Ontology selection (NEW: before project discovery)
    let ontology_search_path = if let Some(ref path) = args.ontology_path {
        path.clone()
    } else {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Ontology Search Path")
            .default(".".into())
            .interact_text()?
    };
    let ontology_search_dir = std::fs::canonicalize(&ontology_search_path)
        .unwrap_or_else(|_| PathBuf::from(&ontology_search_path));

    let discovered = discover_ontologies(&ontology_search_dir).await?;
    let ontology_dir = match discovered.len() {
        0 => {
            anyhow::bail!(
                "No ontology.json found under '{}'.",
                ontology_search_dir.display()
            );
        }
        1 => {
            let onto = discovered[0].clone();
            println!(
                "{}",
                style(format!("Using ontology: {}", onto.display())).dim()
            );
            onto
        }
        _ => {
            let options: Vec<String> = discovered
                .iter()
                .map(|p| {
                    p.strip_prefix(&ontology_search_dir)
                        .unwrap_or(p)
                        .display()
                        .to_string()
                })
                .collect();
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select ontology")
                .items(&options)
                .default(0)
                .interact()?;
            let onto = discovered[selection].clone();
            println!(
                "{}",
                style(format!("Using ontology: {}", onto.display())).dim()
            );
            onto
        }
    };

    let ontology_content = tokio::fs::read_to_string(ontology_dir.join("ontology.json")).await?;

    // 4. Discover existing projects or create new
    let existing_projects: Vec<(PathBuf, IclConfig)> =
        config::discover_projects(&base_work_dir).await?;

    let (final_work_dir, app_name, final_app_id, docs_folder): (PathBuf, String, String, String) =
        if !existing_projects.is_empty() {
            let mut options: Vec<String> = existing_projects
                .iter()
                .map(|(path, config)| format!("{} ({})", config.app_name, path.display()))
                .collect();
            options.push(style("✚ Create New Project").bold().to_string());

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select a project")
                .items(&options)
                .default(0)
                .interact()?;

            if selection < existing_projects.len() {
                let (project_path, config) = &existing_projects[selection];
                println!(
                    "{}",
                    style(format!("Resuming: {} ({})", config.app_name, config.app_id)).yellow()
                );
                let docs = config.docs_folder.clone();
                (
                    project_path.clone(),
                    config.app_name.clone(),
                    config.app_id.clone(),
                    docs,
                )
            } else {
                // Create new project
                let name: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Application Name")
                    .default("MyNewApp".into())
                    .interact_text()?;
                let id = if let Some(ref id) = args.app_id {
                    id.clone()
                } else {
                    uuid::Uuid::new_v4().to_string()
                };
                let docs: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Documents folder (relative to project root)")
                    .default("spec".into())
                    .interact_text()?;
                let project_path = base_work_dir.join(&name);
                if !project_path.exists() {
                    tokio::fs::create_dir_all(&project_path).await?;
                }
                (project_path, name, id, docs)
            }
        } else {
            // No existing projects found — check if this dir itself is a project or create new
            let name: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Application Name")
                .default("MyNewApp".into())
                .interact_text()?;
            let id = if let Some(ref id) = args.app_id {
                id.clone()
            } else {
                uuid::Uuid::new_v4().to_string()
            };
            let docs: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Documents folder (relative to project root)")
                .default("spec".into())
                .interact_text()?;
            let project_path = base_work_dir.join(&name);
            if !project_path.exists() {
                tokio::fs::create_dir_all(&project_path).await?;
            }
            (project_path, name, id, docs)
        };

    // Ensure .infinitecodingloop exists
    config::ensure_infinite_coding_loop(&final_work_dir, &app_name, &final_app_id, &docs_folder)
        .await?;
    println!(
        "{}",
        style(format!("Documents folder: {}", docs_folder)).dim()
    );

    println!("{}", style("Running in LIVE MODE (calling AI CLI)").green());

    // CLI defaults & category mapping Prompt
    use pulpo_engine::graph::executor::ExecutionOptions;
    use std::collections::HashMap;

    let models = vec![
        "gemini-3-pro-preview".to_string(),
        "gemini-3-flash-preview".to_string(),
        "gemini-2.5-pro".to_string(),
        "gemini-2.5-flash".to_string(),
        "gemini-2.5-flash-lite".to_string(),
        "Custom...".to_string(),
    ];

    let mut category_defaults = HashMap::new();
    let categories_to_map = vec!["High Reasoning", "Fast Execution", "Daily Driver"];

    println!(
        "{}",
        style("Please map AI models to execution categories:")
            .magenta()
            .bold()
    );

    // Default fallback model if the user skips or uses `-m` flag.
    let global_model = if let Some(ref m) = args.model {
        Some(m.clone())
    } else {
        None
    };

    for category in categories_to_map {
        let model_for_cat = if let Some(ref m) = global_model {
            m.clone()
        } else {
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt(&format!(
                    "Select model for category: {}",
                    style(category).cyan()
                ))
                .items(&models)
                .default(3) // Default to flash
                .interact()?;

            if models[selection] == "Custom..." {
                Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter custom model string")
                    .interact_text()?
            } else {
                models[selection].clone()
            }
        };

        category_defaults.insert(
            category.to_string(),
            ExecutionOptions {
                model_type: Some(category.to_string()),
                model: Some(model_for_cat.clone()),
                ai_cli: Some("gemini".to_string()), // Default to gemini for now
            },
        );
        println!(
            "{} mapped to {}",
            style(category).cyan(),
            style(&model_for_cat).green()
        );
    }

    // Prepare default shell client (still used for commands not tied to a category, like git commit)
    let client = ShellCliClient::new("gemini", final_work_dir.to_string_lossy().to_string())
        .with_yolo(args.yolo)
        .with_model(global_model.unwrap_or_else(|| "gemini-2.5-flash".to_string()))
        .with_debug(args.debug_ai_cli)
        .with_output_format(args.output_format.clone());

    let mut orchestrator = Orchestrator::new_with_metamodel(
        client,
        final_app_id.clone(),
        app_name.clone(),
        final_work_dir.clone(),
        &ontology_content,
        Some(ontology_dir.as_path()),
    )
    .await?
    .with_max_iterations(args.max_iterations)
    .with_docs_folder(docs_folder)
    .with_category_defaults(category_defaults);

    let ui = CliInteraction::new(args.clone());

    // 4. Iteration Resumption
    let iterations = list_iterations(&final_work_dir).await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_iterations_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let result = list_iterations(tmp.path()).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_list_iterations_with_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let iters_dir = tmp
            .path()
            .join(".infinitecodingloop")
            .join("iterations")
            .join("20260213_0001");
        tokio::fs::create_dir_all(&iters_dir).await.unwrap();

        let iter_info = serde_json::json!({
            "id": "20260213_0001",
            "name": "Test Iteration",
            "timestamp": "20260213_120000"
        });
        tokio::fs::write(
            iters_dir.join("iteration.json"),
            serde_json::to_string_pretty(&iter_info).unwrap(),
        )
        .await
        .unwrap();

        let result = list_iterations(tmp.path()).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "20260213_0001");
        assert_eq!(result[0].1, "Test Iteration");
    }

    #[tokio::test]
    async fn test_load_icl_config_default_docs_folder() {
        // icl.json without docs_folder should default to "spec"
        let tmp = tempfile::tempdir().unwrap();
        let icl_dir = tmp.path().join(".infinitecodingloop");
        tokio::fs::create_dir_all(&icl_dir).await.unwrap();
        let icl_json = r#"{ "version": "1.0.0", "app_id": "id-1", "app_name": "App1", "docs_folder": "spec" }"#;
        tokio::fs::write(icl_dir.join("icl.json"), icl_json)
            .await
            .unwrap();

        let loaded = config::load_icl_config(tmp.path()).await.unwrap().unwrap();
        assert_eq!(loaded.docs_folder, "spec");
    }

    #[tokio::test]
    async fn test_ensure_infinite_coding_loop_creates_icl_and_docs() {
        let tmp = tempfile::tempdir().unwrap();
        config::ensure_infinite_coding_loop(tmp.path(), "MyApp", "app-123", "my_specs")
            .await
            .unwrap();

        // icl.json should exist
        assert!(tmp.path().join(".infinitecodingloop/icl.json").exists());
        // docs folder should exist
        assert!(tmp.path().join("my_specs").exists());
        // iterations dir should exist
        assert!(tmp.path().join(".infinitecodingloop/iterations").exists());

        // Verify icl.json content
        let config = config::load_icl_config(tmp.path()).await.unwrap().unwrap();
        assert_eq!(config.docs_folder, "my_specs");
        assert_eq!(config.app_name, "MyApp");
        assert_eq!(config.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_discover_ontologies_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let result = discover_ontologies(tmp.path()).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_discover_ontologies_single() {
        let tmp = tempfile::tempdir().unwrap();
        let onto_dir = tmp.path().join("my_ontology");
        tokio::fs::create_dir_all(&onto_dir).await.unwrap();
        tokio::fs::write(onto_dir.join("ontology.json"), "[]")
            .await
            .unwrap();

        let result = discover_ontologies(tmp.path()).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], onto_dir);
    }

    #[tokio::test]
    async fn test_discover_ontologies_nested() {
        let tmp = tempfile::tempdir().unwrap();
        let deep = tmp.path().join("a").join("b").join("c");
        tokio::fs::create_dir_all(&deep).await.unwrap();
        tokio::fs::write(deep.join("ontology.json"), "[]")
            .await
            .unwrap();

        let result = discover_ontologies(tmp.path()).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], deep);
    }

    #[tokio::test]
    async fn test_discover_ontologies_multiple() {
        let tmp = tempfile::tempdir().unwrap();
        let dir_a = tmp.path().join("alpha");
        let dir_b = tmp.path().join("beta");
        tokio::fs::create_dir_all(&dir_a).await.unwrap();
        tokio::fs::create_dir_all(&dir_b).await.unwrap();
        tokio::fs::write(dir_a.join("ontology.json"), "[]")
            .await
            .unwrap();
        tokio::fs::write(dir_b.join("ontology.json"), "[]")
            .await
            .unwrap();

        let result = discover_ontologies(tmp.path()).await.unwrap();
        assert_eq!(result.len(), 2);
        // Results should be sorted
        assert!(result[0].ends_with("alpha"));
        assert!(result[1].ends_with("beta"));
    }

    #[tokio::test]
    async fn test_discover_ontologies_skips_hidden_and_dist() {
        let tmp = tempfile::tempdir().unwrap();
        // These should be skipped
        let hidden = tmp.path().join(".hidden");
        let dist = tmp.path().join("dist");
        let target = tmp.path().join("target");
        for d in [&hidden, &dist, &target] {
            tokio::fs::create_dir_all(d).await.unwrap();
            tokio::fs::write(d.join("ontology.json"), "[]")
                .await
                .unwrap();
        }

        // This should be found
        let visible = tmp.path().join("visible");
        tokio::fs::create_dir_all(&visible).await.unwrap();
        tokio::fs::write(visible.join("ontology.json"), "[]")
            .await
            .unwrap();

        let result = discover_ontologies(tmp.path()).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], visible);
    }

    #[tokio::test]
    async fn test_migration_from_old_files() {
        let tmp = tempfile::tempdir().unwrap();
        let icl_dir = tmp.path().join(".infinitecodingloop");
        tokio::fs::create_dir_all(&icl_dir).await.unwrap();

        // Create old files
        let app_json = r#"{ "app_id": "migrated-id", "app_name": "MigratedApp" }"#;
        tokio::fs::write(icl_dir.join("app.json"), app_json)
            .await
            .unwrap();

        let config_json = r#"{ "app_id": "migrated-id", "app_name": "MigratedApp", "docs_folder": "migrated_specs" }"#;
        tokio::fs::write(icl_dir.join("config.json"), config_json)
            .await
            .unwrap();

        // Run ensure_infinite_coding_loop
        config::ensure_infinite_coding_loop(
            tmp.path(),
            "FallbackName",
            "fallback-id",
            "fallback_specs",
        )
        .await
        .unwrap();

        // Verify icl.json contains migrated data
        let config = config::load_icl_config(tmp.path()).await.unwrap().unwrap();
        assert_eq!(config.app_id, "migrated-id");
        assert_eq!(config.app_name, "MigratedApp");
        assert_eq!(config.docs_folder, "migrated_specs");
        assert_eq!(config.version, "1.0.0");

        // Verify old files are gone
        assert!(!icl_dir.join("app.json").exists());
        assert!(!icl_dir.join("config.json").exists());
    }
}
