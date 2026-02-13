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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AppConfig {
    app_id: String,
    app_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ProjectConfig {
    app_id: String,
    app_name: String,
    #[serde(default = "default_docs_folder")]
    docs_folder: String,
}

fn default_docs_folder() -> String {
    "spec".to_string()
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

async fn ensure_infinite_coding_loop(
    work_dir: &Path,
    app_name: &str,
    app_id: &str,
    docs_folder: &str,
) -> Result<()> {
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

    // Ensure config.json exists
    let config_json_path = icl_dir.join("config.json");
    if !config_json_path.exists() {
        let project_config = ProjectConfig {
            app_id: app_id.to_string(),
            app_name: app_name.to_string(),
            docs_folder: docs_folder.to_string(),
        };
        let content = serde_json::to_string_pretty(&project_config)?;
        tokio::fs::write(config_json_path, content).await?;
    }

    // Ensure iterations directory
    tokio::fs::create_dir_all(icl_dir.join("iterations")).await?;

    // Ensure docs folder exists
    let docs_dir = work_dir.join(docs_folder);
    if !docs_dir.exists() {
        tokio::fs::create_dir_all(&docs_dir).await?;
    }

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

async fn load_project_config(work_dir: &Path) -> Result<Option<ProjectConfig>> {
    let config_json_path = work_dir.join(".infinitecodingloop").join("config.json");
    if config_json_path.exists() {
        let content = tokio::fs::read_to_string(config_json_path).await?;
        let config: ProjectConfig = serde_json::from_str(&content)?;
        Ok(Some(config))
    } else {
        Ok(None)
    }
}

/// Discover existing projects in subdirectories of the given path.
/// Returns a list of (subdirectory_path, app_config) pairs.
async fn discover_projects(base_dir: &Path) -> Result<Vec<(PathBuf, AppConfig)>> {
    let mut projects = Vec::new();

    // Check the base dir itself
    if let Some(config) = load_app_config(base_dir).await? {
        projects.push((base_dir.to_path_buf(), config));
    }

    // Check subdirectories
    if let Ok(mut entries) = tokio::fs::read_dir(base_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_dir() {
                if let Some(config) = load_app_config(&path).await? {
                    projects.push((path, config));
                }
            }
        }
    }

    // Sort by app name for consistent display
    projects.sort_by(|a, b| a.1.app_name.cmp(&b.1.app_name));
    Ok(projects)
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
    let work_dir_path = PathBuf::from(work_dir_input);

    if !work_dir_path.exists() {
        tokio::fs::create_dir_all(&work_dir_path).await?;
    }

    // 3. Discover existing projects or create new
    let existing_projects = discover_projects(&work_dir_path).await?;

    let (work_dir_path, app_name, final_app_id, docs_folder) = if !existing_projects.is_empty() {
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
            // Load docs_folder from config.json (or default to "spec")
            let docs = load_project_config(project_path)
                .await?
                .map(|c| c.docs_folder)
                .unwrap_or_else(default_docs_folder);
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
            let project_path = work_dir_path.join(&name);
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
        let project_path = work_dir_path.join(&name);
        if !project_path.exists() {
            tokio::fs::create_dir_all(&project_path).await?;
        }
        (project_path, name, id, docs)
    };

    // Ensure .infinitecodingloop exists
    ensure_infinite_coding_loop(&work_dir_path, &app_name, &final_app_id, &docs_folder).await?;
    println!(
        "{}",
        style(format!("Documents folder: {}", docs_folder)).dim()
    );

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

    // Model selection: use CLI flag or prompt user
    let model = if let Some(ref m) = args.model {
        m.clone()
    } else {
        let models = vec![
            "gemini-3-pro-preview".to_string(),
            "gemini-3-flash-preview".to_string(),
            "gemini-2.5-pro".to_string(),
            "gemini-2.5-flash".to_string(),
            "gemini-2.5-flash-lite".to_string(),
        ];
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select AI model")
            .items(&models)
            .default(0)
            .interact()?;
        models[selection].clone()
    };
    println!("{}", style(format!("Using model: {}", model)).dim());

    // Ontology selection
    let ontology_search_path = args.ontology_path.as_deref().unwrap_or(".");
    let ontology_search_dir = std::fs::canonicalize(ontology_search_path)
        .unwrap_or_else(|_| PathBuf::from(ontology_search_path));

    let discovered = discover_ontologies(&ontology_search_dir).await?;
    let ontology_dir = match discovered.len() {
        0 => {
            anyhow::bail!(
                "No ontology.json found under '{}'. Use --ontology-path to specify the search root.",
                ontology_search_dir.display()
            );
        }
        1 => {
            println!(
                "{}",
                style(format!("Using ontology: {}", discovered[0].display())).dim()
            );
            discovered[0].clone()
        }
        _ => {
            let options: Vec<String> = discovered
                .iter()
                .map(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                })
                .collect();
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select ontology")
                .items(&options)
                .default(0)
                .interact()?;
            println!(
                "{}",
                style(format!(
                    "Using ontology: {}",
                    discovered[selection].display()
                ))
                .dim()
            );
            discovered[selection].clone()
        }
    };

    let ontology_content = tokio::fs::read_to_string(ontology_dir.join("ontology.json")).await?;

    let client = ShellCliClient::new("gemini", work_dir_path.to_string_lossy().to_string())
        .with_yolo(args.yolo)
        .with_model(model)
        .with_debug(args.debug_ai_cli)
        .with_output_format(args.output_format.clone());

    let mut orchestrator = Orchestrator::new_with_metamodel(
        client,
        final_app_id.clone(),
        app_name,
        work_dir_path.clone(),
        &ontology_content,
        Some(ontology_dir.as_path()),
    )
    .await?
    .with_max_iterations(args.max_iterations)
    .with_docs_folder(docs_folder);

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a fake project in a directory.
    async fn create_fake_project(dir: &Path, app_name: &str, app_id: &str) {
        let icl_dir = dir.join(".infinitecodingloop");
        tokio::fs::create_dir_all(&icl_dir).await.unwrap();
        let config = AppConfig {
            app_id: app_id.to_string(),
            app_name: app_name.to_string(),
        };
        let content = serde_json::to_string_pretty(&config).unwrap();
        tokio::fs::write(icl_dir.join("app.json"), content)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_load_app_config_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let result = load_app_config(tmp.path()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_load_app_config_present() {
        let tmp = tempfile::tempdir().unwrap();
        create_fake_project(tmp.path(), "TestApp", "test-id-123").await;

        let config = load_app_config(tmp.path()).await.unwrap().unwrap();
        assert_eq!(config.app_name, "TestApp");
        assert_eq!(config.app_id, "test-id-123");
    }

    #[tokio::test]
    async fn test_discover_projects_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let projects = discover_projects(tmp.path()).await.unwrap();
        assert!(projects.is_empty());
    }

    #[tokio::test]
    async fn test_discover_projects_single_in_base() {
        let tmp = tempfile::tempdir().unwrap();
        create_fake_project(tmp.path(), "BaseApp", "base-001").await;

        let projects = discover_projects(tmp.path()).await.unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].1.app_name, "BaseApp");
        assert_eq!(projects[0].0, tmp.path().to_path_buf());
    }

    #[tokio::test]
    async fn test_discover_projects_multiple_subdirs() {
        let tmp = tempfile::tempdir().unwrap();

        // Create three projects in subdirectories
        let app_a = tmp.path().join("AppAlpha");
        let app_b = tmp.path().join("AppBeta");
        let app_c = tmp.path().join("AppGamma");
        tokio::fs::create_dir_all(&app_a).await.unwrap();
        tokio::fs::create_dir_all(&app_b).await.unwrap();
        tokio::fs::create_dir_all(&app_c).await.unwrap();

        create_fake_project(&app_a, "Alpha", "id-a").await;
        create_fake_project(&app_b, "Beta", "id-b").await;
        create_fake_project(&app_c, "Gamma", "id-c").await;

        // Also create a plain subdir without a project
        tokio::fs::create_dir_all(tmp.path().join("not-a-project"))
            .await
            .unwrap();

        let projects = discover_projects(tmp.path()).await.unwrap();
        assert_eq!(projects.len(), 3);

        // Should be sorted by app_name
        assert_eq!(projects[0].1.app_name, "Alpha");
        assert_eq!(projects[1].1.app_name, "Beta");
        assert_eq!(projects[2].1.app_name, "Gamma");
    }

    #[tokio::test]
    async fn test_discover_projects_base_and_subdirs() {
        let tmp = tempfile::tempdir().unwrap();

        // Project in the base dir itself
        create_fake_project(tmp.path(), "RootProject", "root-id").await;

        // Project in a subdir
        let sub = tmp.path().join("SubProject");
        tokio::fs::create_dir_all(&sub).await.unwrap();
        create_fake_project(&sub, "ChildProject", "child-id").await;

        let projects = discover_projects(tmp.path()).await.unwrap();
        assert_eq!(projects.len(), 2);

        // Sorted by name: ChildProject, RootProject
        assert_eq!(projects[0].1.app_name, "ChildProject");
        assert_eq!(projects[1].1.app_name, "RootProject");
    }

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
    async fn test_load_project_config_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let result = load_project_config(tmp.path()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_load_project_config_present() {
        let tmp = tempfile::tempdir().unwrap();
        let icl_dir = tmp.path().join(".infinitecodingloop");
        tokio::fs::create_dir_all(&icl_dir).await.unwrap();
        let config = ProjectConfig {
            app_id: "test-id".to_string(),
            app_name: "TestApp".to_string(),
            docs_folder: "docs".to_string(),
        };
        let content = serde_json::to_string_pretty(&config).unwrap();
        tokio::fs::write(icl_dir.join("config.json"), content)
            .await
            .unwrap();

        let loaded = load_project_config(tmp.path()).await.unwrap().unwrap();
        assert_eq!(loaded.docs_folder, "docs");
        assert_eq!(loaded.app_name, "TestApp");
    }

    #[tokio::test]
    async fn test_load_project_config_default_docs_folder() {
        // config.json without docs_folder should default to "spec"
        let tmp = tempfile::tempdir().unwrap();
        let icl_dir = tmp.path().join(".infinitecodingloop");
        tokio::fs::create_dir_all(&icl_dir).await.unwrap();
        let config_json = r#"{ "app_id": "id-1", "app_name": "App1" }"#;
        tokio::fs::write(icl_dir.join("config.json"), config_json)
            .await
            .unwrap();

        let loaded = load_project_config(tmp.path()).await.unwrap().unwrap();
        assert_eq!(loaded.docs_folder, "spec");
    }

    #[tokio::test]
    async fn test_ensure_infinite_coding_loop_creates_config_and_docs() {
        let tmp = tempfile::tempdir().unwrap();
        ensure_infinite_coding_loop(tmp.path(), "MyApp", "app-123", "my_specs")
            .await
            .unwrap();

        // app.json should exist
        assert!(tmp.path().join(".infinitecodingloop/app.json").exists());
        // config.json should exist
        assert!(tmp.path().join(".infinitecodingloop/config.json").exists());
        // docs folder should exist
        assert!(tmp.path().join("my_specs").exists());
        // iterations dir should exist
        assert!(tmp.path().join(".infinitecodingloop/iterations").exists());

        // Verify config.json content
        let config = load_project_config(tmp.path()).await.unwrap().unwrap();
        assert_eq!(config.docs_folder, "my_specs");
        assert_eq!(config.app_name, "MyApp");
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
}
