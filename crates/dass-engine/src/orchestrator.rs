use crate::agents::cli_client::AiCliClient;
use crate::agents::generic::GenericAgent;

use crate::domain::types::AgentRole;
use crate::graph::executor::{GraphExecutor, InMemoryExecutor, Task};
use crate::graph::{DependencyGraph, RelationCategory};
use crate::logging::IterationLogger;
use anyhow::{Context, Result};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionPlan {
    pub agent: String,
    pub target: String,
    pub relation: String,
    pub category: RelationCategory,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IterationInfo {
    pub id: String,
    pub name: String,
    pub timestamp: String,
}

pub struct Orchestrator<C: AiCliClient + Clone + Send + Sync + 'static> {
    pub app_id: String,
    pub app_name: String,
    pub work_dir: Option<PathBuf>,
    pub docs_folder: String,
    // New Graph Components
    pub executor: InMemoryExecutor,
    // Tracking produced artifacts (State)
    pub artifacts: HashMap<String, serde_json::Value>, // EntityKind -> Last Produced Value
    pub verification_feedback: HashMap<String, String>, // Target -> Feedback
    pub verified_artifacts: std::collections::HashSet<String>, // Tracks those with score 1.0
    pub refinement_attempts: HashMap<String, usize>,   // Target -> retry count
    max_iterations: usize,
    // Iteration tracking
    pub current_iteration: Option<IterationInfo>,
    // Execution logger for full traceability
    pub logger: Option<IterationLogger>,
    // The client used to interact with the AI
    pub client: C,
    // Category mapping defaults
    pub category_defaults: HashMap<String, crate::graph::executor::ExecutionOptions>,
}

impl<C: AiCliClient + Clone + Send + Sync + 'static> Orchestrator<C> {
    pub async fn new(
        client: C,
        app_id: String,
        app_name: String,
        work_dir: PathBuf,
    ) -> Result<Self> {
        let metamodel_json = include_str!("../../../ontology-software-engineering/ontology.json");
        Self::new_with_metamodel(client, app_id, app_name, work_dir, metamodel_json, None).await
    }

    pub async fn new_with_metamodel(
        client: C,
        app_id: String,
        app_name: String,
        work_dir: PathBuf,
        metamodel_json: &str,
        ontology_base_path: Option<&std::path::Path>,
    ) -> Result<Self> {
        info!(
            "Initializing Orchestrator for app: {} ({})",
            app_name, app_id
        );

        // Initialize Graph (Load Metamodel)
        let graph = DependencyGraph::load_from_metamodel(metamodel_json, ontology_base_path)?;

        // Initialize Executor and Register Agents
        let mut executor = InMemoryExecutor::new(graph);

        // Dynamic Registration from Graph
        // Collect roles and configs first to avoid borrowing conflict
        let agent_data: Vec<(String, String)> = executor
            .graph
            .loaded_agents
            .iter()
            .map(|(r, c)| (r.clone(), c.clone()))
            .collect();

        for (role_str, config_json) in agent_data {
            let role = AgentRole::from(role_str);

            // Parse Config
            let system_prompt =
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&config_json) {
                    v.get("system_prompt")
                        .and_then(|t| t.as_str())
                        .unwrap_or("")
                        .to_string()
                } else {
                    "".to_string()
                };

            debug!("Registering agent: {:?}", role);
            executor.register_agent(Box::new(GenericAgent::new(
                client.clone(),
                role,
                system_prompt,
            )));
        }

        Ok(Self {
            app_id,
            app_name,
            work_dir: Some(work_dir),
            docs_folder: "spec".to_string(),
            executor,
            artifacts: HashMap::new(),
            verification_feedback: HashMap::new(),
            verified_artifacts: std::collections::HashSet::new(),
            refinement_attempts: HashMap::new(),
            max_iterations: 100,
            current_iteration: None,
            logger: None,
            client,
            category_defaults: HashMap::new(),
        })
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    pub fn with_docs_folder(mut self, folder: String) -> Self {
        self.docs_folder = folder;
        self
    }

    pub fn with_category_defaults(
        mut self,
        defaults: HashMap<String, crate::graph::executor::ExecutionOptions>,
    ) -> Self {
        self.category_defaults = defaults;
        self
    }

    async fn ensure_persistence_dirs(&self) -> Result<PathBuf> {
        let work_dir = self.work_dir.as_ref().context("Work directory not set")?;
        let icl_dir = work_dir.join(".infinitecodingloop");
        let iterations_dir = icl_dir.join("iterations");

        if !iterations_dir.exists() {
            tokio::fs::create_dir_all(&iterations_dir).await?;
        }

        Ok(icl_dir)
    }

    pub async fn start_iteration(&mut self, name: &str) -> Result<()> {
        let icl_dir = self.ensure_persistence_dirs().await?;
        let work_dir = self.work_dir.as_ref().context("Work directory not set")?;
        let now = chrono::Local::now();
        let date_prefix = now.format("%Y%m%d").to_string();
        let timestamp = now.format("%Y%m%d_%H%M%S").to_string();

        // Generate sequential ID: scan existing iterations for today's date
        let iterations_dir = icl_dir.join("iterations");
        let mut max_seq = 0u32;
        if let Ok(mut entries) = tokio::fs::read_dir(&iterations_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(&date_prefix) {
                        if let Some(seq_str) = name.split('_').nth(1) {
                            if let Ok(seq) = seq_str.parse::<u32>() {
                                max_seq = max_seq.max(seq);
                            }
                        }
                    }
                }
            }
        }
        let id = format!("{}_{:04}", date_prefix, max_seq + 1);

        let iter_info = IterationInfo {
            id: id.clone(),
            name: name.to_string(),
            timestamp,
        };

        let iter_folder = iterations_dir.join(&id);
        tokio::fs::create_dir_all(&iter_folder).await?;

        let iter_json_path = iter_folder.join("iteration.json");
        let content = serde_json::to_string_pretty(&iter_info)?;
        tokio::fs::write(iter_json_path, content).await?;

        // Ensure docs folder exists
        let docs_dir = work_dir.join(&self.docs_folder);
        if !docs_dir.exists() {
            tokio::fs::create_dir_all(&docs_dir).await?;
        }

        // Initialize execution logger
        let logger = IterationLogger::new(&iter_folder).await?;
        logger.log_iteration_start(&id, name).await?;

        info!("Started new iteration: {} ({})", name, id);
        self.logger = Some(logger);
        self.current_iteration = Some(iter_info);
        Ok(())
    }

    pub async fn load_iteration(&mut self, iteration_id: &str) -> Result<()> {
        let work_dir = self.work_dir.as_ref().context("Work directory not set")?;
        let iter_folder = work_dir
            .join(".infinitecodingloop")
            .join("iterations")
            .join(iteration_id);

        let iter_json_path = iter_folder.join("iteration.json");
        let content = tokio::fs::read_to_string(iter_json_path).await?;
        let iter_info: IterationInfo = serde_json::from_str(&content)?;
        self.current_iteration = Some(iter_info);

        // Initialize execution logger (append mode for resumed iterations)
        let logger = IterationLogger::new(&iter_folder).await?;
        logger.log_iteration_resumed(iteration_id).await?;
        self.logger = Some(logger);

        // Load artifacts from the docs folder
        let docs_dir = work_dir.join(&self.docs_folder);
        if docs_dir.exists() {
            let mut entries = tokio::fs::read_dir(&docs_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown");
                    let content = tokio::fs::read_to_string(&path).await?;
                    let data: serde_json::Value = serde_json::from_str(&content)?;

                    let mut found_kind = name.to_string();
                    for node_idx in self.executor.graph.graph.node_indices() {
                        let kind = &self.executor.graph.graph[node_idx];
                        if kind.to_lowercase() == name.to_lowercase() {
                            found_kind = kind.clone();
                            break;
                        }
                    }

                    self.artifacts.insert(found_kind, data);
                }
            }
        }

        info!(
            "Loaded iteration: {} ({})",
            self.current_iteration.as_ref().unwrap().name,
            iteration_id
        );
        Ok(())
    }

    pub fn get_execution_status(&self) -> (Vec<String>, Vec<String>) {
        let mut done = Vec::new();
        let mut pending = Vec::new();

        for node_idx in self.executor.graph.graph.node_indices() {
            let kind = &self.executor.graph.graph[node_idx];
            if self.executor.graph.is_agent(kind) {
                continue;
            }
            if kind == "SoftwareApplication" {
                continue;
            }

            if self.artifacts.contains_key(kind) {
                done.push(kind.clone());
            } else {
                // Check if it's actionable
                let mut actionable = false;
                for edge in self
                    .executor
                    .graph
                    .graph
                    .edges_directed(node_idx, petgraph::Direction::Incoming)
                {
                    let source_idx = edge.source();
                    let source_kind = &self.executor.graph.graph[source_idx];
                    let relation = edge.weight();
                    let edge_key = (
                        source_kind.to_string(),
                        relation.to_string(),
                        kind.to_string(),
                    );
                    let category = self
                        .executor
                        .graph
                        .edge_categories
                        .get(&edge_key)
                        .copied()
                        .unwrap_or(RelationCategory::Context);
                    if category == RelationCategory::Creation {
                        actionable = true;
                        break;
                    }
                }
                if actionable {
                    pending.push(kind.clone());
                }
            }
        }

        (done, pending)
    }

    async fn persist_artifact(&self, name: &str, data: &serde_json::Value) -> Result<()> {
        let iteration = self
            .current_iteration
            .as_ref()
            .context("No active iteration to persist artifact")?;
        let work_dir = self.work_dir.as_ref().context("Work directory not set")?;

        // Write artifact to the docs folder
        let docs_dir = work_dir.join(&self.docs_folder);
        if !docs_dir.exists() {
            tokio::fs::create_dir_all(&docs_dir).await?;
        }

        let filename = format!("{}.json", name.to_lowercase());
        let artifact_path = docs_dir.join(&filename);
        let content = serde_json::to_string_pretty(data)?;
        tokio::fs::write(&artifact_path, content).await?;

        // Record metadata in .infinitecodingloop/iterations/{id}/artifacts.json
        let iter_dir = work_dir
            .join(".infinitecodingloop")
            .join("iterations")
            .join(&iteration.id);
        if !iter_dir.exists() {
            tokio::fs::create_dir_all(&iter_dir).await?;
        }

        let artifacts_meta_path = iter_dir.join("artifacts.json");
        let mut entries: Vec<serde_json::Value> = if artifacts_meta_path.exists() {
            let existing = tokio::fs::read_to_string(&artifacts_meta_path).await?;
            serde_json::from_str(&existing).unwrap_or_default()
        } else {
            Vec::new()
        };

        let relative_path = format!("{}/{}", self.docs_folder, filename);
        let now = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        entries.push(serde_json::json!({
            "name": name,
            "timestamp": now,
            "path": relative_path
        }));

        let meta_content = serde_json::to_string_pretty(&entries)?;
        tokio::fs::write(artifacts_meta_path, meta_content).await?;

        debug!(
            "Persisted artifact {} to {}/{} (iteration {})",
            name, self.docs_folder, filename, iteration.id
        );

        // Log artifact persisted
        if let Some(ref logger) = self.logger {
            let _ = logger.log_artifact_persisted(name, &relative_path).await;
        }

        Ok(())
    }

    pub async fn run(&mut self, ui: &impl crate::interaction::UserInteraction) -> Result<()> {
        ui.log_info("Starting Generic Graph-Driven Orchestration...");

        // 1. Initial Input (Skip if already exists in resumed iteration)
        if !self.artifacts.contains_key("SoftwareApplication") {
            let initial_goal = ui
                .ask_for_feature("What feature do you want to build?")
                .await?;
            if initial_goal.is_empty() {
                return Ok(());
            }

            if self.current_iteration.is_none() {
                self.start_iteration("Initial Goal Formulation").await?;
            }

            // Feed initial input as a "SoftwareApplication" signal to start the orchestration
            let initial_app =
                serde_json::json!({ "name": self.app_name.clone(), "goal": initial_goal });
            self.artifacts
                .insert("SoftwareApplication".to_string(), initial_app.clone());
            self.persist_artifact("SoftwareApplication", &initial_app)
                .await?;
        }

        // 2. Generic Execution Loop (State Machine)
        let mut iterations = 0;

        while iterations < self.max_iterations {
            iterations += 1;
            ui.start_step(&format!(
                "Iteration {} - Evaluating Graph State...",
                iterations
            ));

            // Log loop cycle
            if let Some(ref logger) = self.logger {
                let _ = logger.log_loop_cycle(iterations).await;
            }

            let next_actions = self.identify_next_actions();
            if next_actions.is_empty() {
                ui.log_info(&format!(
                    "No more actions identified. Current artifacts: {:?}",
                    self.artifacts.keys().collect::<Vec<_>>()
                ));
                break;
            }

            // Log all identified actions
            if let Some(ref logger) = self.logger {
                for action in &next_actions {
                    let _ = logger
                        .log_action_identified(
                            &action.agent,
                            &action.relation,
                            &action.target,
                            &format!("{:?}", action.category),
                        )
                        .await;
                }
            }

            for action in next_actions {
                ui.log_info(&format!(
                    "Next Action: {} {} {}",
                    action.agent, action.relation, action.target
                ));

                if !ui
                    .confirm(&format!(
                        "Execute: {} {} {}?",
                        action.agent, action.relation, action.target
                    ))
                    .await?
                {
                    // Log action skipped
                    if let Some(ref logger) = self.logger {
                        let _ = logger
                            .log_action_skipped(&action.agent, &action.relation, &action.target)
                            .await;
                    }
                    ui.log_info("Action skipped by user. Pausing iteration.");
                    ui.end_step("Iteration paused by user.");
                    return Ok(());
                }

                self.execute_action(action, ui).await?;
            }
        }

        // Log iteration end
        if let Some(ref logger) = self.logger {
            let _ = logger
                .log(crate::logging::LogEvent::info(
                    crate::logging::LogEventType::IterationEnd,
                    "Iteration completed",
                ))
                .await;
        }

        ui.end_step("Goal achieved or max iterations reached.");
        Ok(())
    }

    pub fn identify_next_actions(&self) -> Vec<ActionPlan> {
        let mut plans = Vec::new();

        for edge_idx in self.executor.graph.graph.edge_indices() {
            let (source_idx, target_idx) =
                self.executor.graph.graph.edge_endpoints(edge_idx).unwrap();
            let source_kind = &self.executor.graph.graph[source_idx];
            let target_kind = &self.executor.graph.graph[target_idx];
            let relation = self.executor.graph.graph.edge_weight(edge_idx).unwrap();
            let edge_key = (
                source_kind.to_string(),
                relation.to_string(),
                target_kind.to_string(),
            );
            let category = self
                .executor
                .graph
                .edge_categories
                .get(&edge_key)
                .copied()
                .unwrap_or(RelationCategory::Context);

            if self.executor.graph.is_agent(source_kind) {
                // Check if this action is actionable based on dependencies
                // An artifact creation/verification is only actionable if all its Dependency edges are met.
                let mut dependencies_met = true;
                for target_edge in self
                    .executor
                    .graph
                    .graph
                    .edges_directed(target_idx, petgraph::Direction::Outgoing)
                {
                    let dep_target_idx = target_edge.target();
                    let dep_kind = &self.executor.graph.graph[dep_target_idx];
                    let dep_relation = target_edge.weight();
                    let dep_edge_key = (
                        target_kind.to_string(),
                        dep_relation.to_string(),
                        dep_kind.to_string(),
                    );
                    let dep_category = self
                        .executor
                        .graph
                        .edge_categories
                        .get(&dep_edge_key)
                        .copied()
                        .unwrap_or(RelationCategory::Context);
                    if dep_category == RelationCategory::Dependency {
                        if !self.artifacts.contains_key(dep_kind) {
                            debug!(
                                "Action {} {} {} is blocked by missing dependency: {}",
                                source_kind, relation, target_kind, dep_kind
                            );
                            dependencies_met = false;
                            break;
                        }
                    }
                }

                if !dependencies_met {
                    continue;
                }

                match category {
                    RelationCategory::Creation => {
                        // Create if it doesn't exist
                        if !self.artifacts.contains_key(target_kind) {
                            plans.push(ActionPlan {
                                agent: source_kind.clone(),
                                target: target_kind.clone(),
                                relation: relation.clone(),
                                category,
                            });
                        }
                    }
                    RelationCategory::Verification => {
                        // Verify if artifact exists but isn't already verified-perfect
                        if self.artifacts.contains_key(target_kind)
                            && !self.verified_artifacts.contains(target_kind)
                            && !self.verification_feedback.contains_key(target_kind)
                        {
                            plans.push(ActionPlan {
                                agent: source_kind.clone(),
                                target: target_kind.clone(),
                                relation: relation.clone(),
                                category,
                            });
                        }
                    }
                    RelationCategory::Refinement => {
                        // Refine if artifact exists AND we have feedback (indicating it needs work)
                        // Gate on max retries from LoopConfig
                        if self.artifacts.contains_key(target_kind)
                            && self.verification_feedback.contains_key(target_kind)
                        {
                            let max_retries = self
                                .executor
                                .graph
                                .loop_configs
                                .get(&edge_key)
                                .map(|lc| lc.max_retries)
                                .unwrap_or(3);
                            let attempts = self.refinement_attempts.get(target_kind).unwrap_or(&0);
                            if *attempts < max_retries {
                                plans.push(ActionPlan {
                                    agent: source_kind.clone(),
                                    target: target_kind.clone(),
                                    relation: relation.clone(),
                                    category,
                                });
                            } else {
                                warn!(
                                    "Max retries ({}) reached for refining {}. Skipping.",
                                    max_retries, target_kind
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Return all logical actions. The orchestrator loop will execute them sequentially.
        plans
    }

    async fn execute_action(
        &mut self,
        action: ActionPlan,
        ui: &impl crate::interaction::UserInteraction,
    ) -> Result<()> {
        let agent_role = AgentRole::from(action.agent.as_str());

        // Log action dispatched
        if let Some(ref logger) = self.logger {
            let _ = logger
                .log_action_dispatched(
                    &action.agent,
                    &action.relation,
                    &action.target,
                    &format!("{:?}", action.category),
                )
                .await;
        }

        // Find Input Context
        let mut context_map = HashMap::new();
        let mut reference_instructions = String::new();

        // Get related artifacts from the graph
        let mut related_artifacts: HashSet<String> = self
            .executor
            .graph
            .get_related_artifacts(&action.target)
            .into_iter()
            .collect();

        // Always include the target itself if we are refining or verifying
        if matches!(
            action.category,
            RelationCategory::Refinement | RelationCategory::Verification
        ) {
            related_artifacts.insert(action.target.clone());
        }

        // Always include SoftwareApplication if available (as global context)
        related_artifacts.insert("SoftwareApplication".to_string());

        for (kind, val) in &self.artifacts {
            // Only include if related
            if !related_artifacts.contains(kind) {
                continue;
            }

            debug!("  [Context] Retrieving: {}", kind);

            // Check if this artifact is "Code" type (Reference-Based)
            let is_code = self
                .executor
                .graph
                .node_types
                .get(kind)
                .map(|t| t == "Code")
                .unwrap_or(false);

            if is_code {
                // Parse as Reference Artifact (files array)
                if let Some(files) = val.get("files").and_then(|f| f.as_array()) {
                    let file_list: Vec<String> = files
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();

                    context_map.insert(kind.clone(), serde_json::json!({
                        "summary": format!("Reference-based artifact. Contains {} files.", file_list.len()),
                        "files": file_list
                    }));

                    reference_instructions.push_str(&format!(
                        "\n\n[REFERENCE ARTIFACT: {}]\nThis artifact contains references to files on disk. available files:\n", 
                        kind
                    ));
                    for f in &file_list {
                        reference_instructions.push_str(&format!("- {}\n", f));
                    }
                    reference_instructions.push_str(&format!(
                        "Use your `read_file` tool to inspect the content of these files as needed. Do NOT guess the content.\n"
                    ));
                } else {
                    // Fallback if schema doesn't match expected reference format
                    context_map.insert(kind.clone(), val.clone());
                }
            } else {
                // Default: Value-Based Artifact
                context_map.insert(kind.clone(), val.clone());
            }
        }

        let mut context = serde_json::to_string_pretty(&context_map).unwrap_or_default();
        if !reference_instructions.is_empty() {
            context.push_str(&reference_instructions);
        }

        // If refining, inject feedback
        if action.category == RelationCategory::Refinement {
            if let Some(feedback) = self.verification_feedback.get(&action.target) {
                context = format!("{}\n\n### FEEDBACK FOR REFINEMENT:\n{}", context, feedback);
            }
        }

        let prompt_template = self
            .executor
            .graph
            .get_prompt_template(&action.agent, &action.relation, &action.target)
            .unwrap_or_else(|| {
                format!(
                    "Perform {} on {} based on the context.",
                    action.relation, action.target
                )
            });

        let mut final_prompt = prompt_template
            .replace("{{source}}", &action.agent)
            .replace("{{relation}}", &action.relation)
            .replace("{{target}}", &action.target);

        // Validating if the prompt template contains {{source_content}} or {{input}}
        if final_prompt.contains("{{source_content}}") {
            final_prompt = final_prompt.replace("{{source_content}}", &context);
        } else if final_prompt.contains("{{input}}") {
            final_prompt = final_prompt.replace("{{input}}", &context);
        } else if !context.is_empty() {
            final_prompt = format!("{}\n\n### Context / Input:\n{}", final_prompt, context);
        }

        let filename = if action.category == RelationCategory::Verification {
            format!("{}_verification.json", action.target.to_lowercase())
        } else {
            format!("{}.json", action.target.to_lowercase())
        };

        let entity_type = self
            .executor
            .graph
            .node_types
            .get(&action.target)
            .map(|s| s.as_str());
        let enhanced_prompt =
            self.enhance_prompt(final_prompt, &action.target, &filename, entity_type);

        // Log prompt sent
        if let Some(ref logger) = self.logger {
            let _ = logger
                .log_prompt_sent(&action.agent, &action.target, &enhanced_prompt)
                .await;
        }

        // Map the execution options
        let options = self
            .executor
            .graph
            .node_configs
            .get(&action.target)
            .map(|config| crate::graph::executor::ExecutionOptions {
                model_type: config.model_type.clone(),
                model: config.model.clone(),
                ai_cli: config.ai_cli.clone(),
            })
            .unwrap_or_default();

        let task = Task {
            id: format!("task_{}_{}", action.relation, action.target),
            description: format!("{} {}", action.relation, action.target),
            inputs: vec![],
            prompt: Some(enhanced_prompt),
            options,
        };

        let result = match self.executor.dispatch_agent(agent_role, task).await {
            Ok(val) => {
                // Log response received
                if let Some(ref logger) = self.logger {
                    let _ = logger
                        .log_response_received(&action.agent, &action.target, &val)
                        .await;
                }
                val
            }
            Err(e) => {
                // Log error
                if let Some(ref logger) = self.logger {
                    let _ = logger
                        .log_error(
                            &format!(
                                "Agent dispatch failed: {} {} {}",
                                action.agent, action.relation, action.target
                            ),
                            Some(&format!("{}", e)),
                        )
                        .await;
                }
                return Err(e);
            }
        };

        // Semantic Result Handling
        match action.category {
            RelationCategory::Verification => {
                let feedback = result
                    .get("feedback")
                    .and_then(|v| v.as_str())
                    .or_else(|| result.get("test_results").and_then(|v| v.as_str()))
                    .unwrap_or("No detailed feedback provided.");

                let score = result.get("score").and_then(|v| v.as_f64()).unwrap_or(1.0);

                // Look up pass_threshold from any verification LoopConfig for this target
                let pass_threshold = self
                    .executor
                    .graph
                    .loop_configs
                    .iter()
                    .find(|((_, _, t), _)| t == &action.target)
                    .map(|(_, lc)| lc.pass_threshold)
                    .unwrap_or(1.0);

                // Log verification result
                if let Some(ref logger) = self.logger {
                    let _ = logger
                        .log_verification(&action.target, score, pass_threshold, feedback)
                        .await;
                }

                if score < pass_threshold {
                    warn!(
                        "Verification failed for {} (score: {:.2}, threshold: {:.2}): {}",
                        action.target, score, pass_threshold, feedback
                    );
                    self.verification_feedback
                        .insert(action.target.clone(), feedback.to_string());
                    self.verified_artifacts.remove(&action.target);
                } else {
                    info!(
                        "Verification passed for {} (score: {:.2} >= {:.2})",
                        action.target, score, pass_threshold
                    );
                    self.verification_feedback.remove(&action.target);
                    self.verified_artifacts.insert(action.target.clone());
                }

                // Persist verification report
                self.persist_artifact(&format!("{}_verification", action.target), &result)
                    .await?;
            }
            RelationCategory::Refinement | RelationCategory::Creation => {
                let graph = self.executor.graph.clone();
                let target = action.target.clone();
                let result_clone = result.clone();

                let validation_result = tokio::task::spawn_blocking(move || {
                    graph.validate_artifact(&target, &result_clone)
                })
                .await?;

                if let Err(e) = validation_result {
                    let is_code = self
                        .executor
                        .graph
                        .node_types
                        .get(&action.target)
                        .map(|t| t == "Code")
                        .unwrap_or(false);

                    // Log validation failure
                    if let Some(ref logger) = self.logger {
                        let _ = logger
                            .log_validation(&action.target, false, Some(&format!("{}", e)))
                            .await;
                    }

                    if is_code {
                        warn!(
                            "Artifact type 'Code' detected. Skipping strict schema validation for {}. Validation error was: {}",
                            action.target, e
                        );
                    } else {
                        warn!("Artifact validation failed for {}: {}", action.target, e);
                        return Err(anyhow::anyhow!(
                            "Artifact validation failed for {}: {}. Result: {}",
                            action.target,
                            e,
                            result
                        ));
                    }
                } else {
                    // Log validation success
                    if let Some(ref logger) = self.logger {
                        let _ = logger.log_validation(&action.target, true, None).await;
                    }
                }

                info!("Successfully created/refined artifact: {}", action.target);
                self.artifacts.insert(action.target.clone(), result.clone());
                self.persist_artifact(&action.target, &result).await?;

                self.verified_artifacts.remove(&action.target);

                // Instruct AI CLI to commit the changes
                let commit_prompt = format!(
                    "The agent {} has successfully {} the artifact {}. Please stage all changes (`git add .`) and commit them to the git repository with a descriptive message like '{} {} {}'. Execute the git commands to commit the work.",
                    action.agent,
                    if action.category == RelationCategory::Creation {
                        "created"
                    } else {
                        "refined"
                    },
                    action.target,
                    if action.category == RelationCategory::Creation {
                        "Create"
                    } else {
                        "Refine"
                    },
                    action.target,
                    "artifact"
                );

                if let Err(e) = self.client.prompt(&commit_prompt, Default::default()).await {
                    warn!("Failed to commit changes via AI CLI: {}", e);
                } else {
                    info!("Successfully committed changes to git via AI CLI.");
                }

                if action.category == RelationCategory::Refinement {
                    self.verification_feedback.remove(&action.target);
                    // Track refinement attempts for loop exit
                    let attempts = self
                        .refinement_attempts
                        .entry(action.target.clone())
                        .or_insert(0);
                    *attempts += 1;
                    info!("Refinement attempt {} for {}", attempts, action.target);

                    // Log refinement attempt
                    if let Some(ref logger) = self.logger {
                        let max_retries = self
                            .executor
                            .graph
                            .loop_configs
                            .iter()
                            .find(|((_, _, t), _)| t == &action.target)
                            .map(|(_, lc)| lc.max_retries)
                            .unwrap_or(3);
                        let _ = logger
                            .log_refinement_attempt(&action.target, *attempts, max_retries)
                            .await;
                    }
                }
            }
            _ => {
                self.artifacts.insert(action.target.clone(), result.clone());
                self.persist_artifact(&action.target, &result).await?;
            }
        }

        ui.render_artifact(&action.target, self.artifacts.get(&action.target).unwrap());

        Ok(())
    }

    fn enhance_prompt(
        &self,
        prompt: String,
        target: &str,
        filename: &str,
        entity_type: Option<&str>,
    ) -> String {
        let is_code = entity_type == Some("Code");
        let has_schema = self.executor.graph.schemas.contains_key(target);

        let mut base = format!("{}\n\nPlease generate the {} artifact.", prompt, target);

        // Determine the documents path for file writing
        let docs_path = &self.docs_folder;

        // For artifacts WITHOUT a schema, instruct the AI CLI to write the file
        // For artifacts WITH a schema, the orchestrator handles persistence via persist_artifact
        if !has_schema || is_code {
            base.push_str("\n\n**Tool-Driven Persistence Required**:\n");
            base.push_str(&format!(
                "1. You MUST use your tools (e.g., `write_file`) to persist the {} content to the file `{}/{}`.\n",
                target, docs_path, filename
            ));
            base.push_str(
                "2. Do NOT just output the text; you are responsible for the file creation.\n",
            );
        }

        base.push_str("\n**Strict Output Rules**:\n");
        if is_code {
            base.push_str(
                "1. Provide the file manifest as the ONLY output in a single triple-backtick JSON code block.\n",
            );
            base.push_str(
                "2. The JSON object MUST have a `files` key listing the updated paths.\n",
            );
            base.push_str("3. Example: ````json\n{\"files\": [\"main.rs\", \"Cargo.toml\"], \"main_file\": \"main.rs\"}\n````\n");
        } else {
            // Inject Schema if present
            if let Some(schema_content) = self.executor.graph.schemas.get(target) {
                base.push_str("\n**STRICT SCHEMA ADHERENCE REQUIRED**:\n");
                base.push_str("Your output MUST adhere strictly to the following JSON schema.\n");
                base.push_str("Output Schema:\n");
                base.push_str(&format!("```json\n{}\n```\n", schema_content));

                // Inject Base Schema if available (to resolve $ref visibility for LLM)
                if let Some(base_schema) = self
                    .executor
                    .graph
                    .schemas
                    .get("https://infinite-coding-loop.dass/schemas/base.schema.json")
                {
                    base.push_str("\nBase Schema Definitions (Reference):\n");
                    base.push_str(&format!("```json\n{}\n```\n", base_schema));
                }

                // Inject Taxonomy Schema to clarify "Kind_*" values
                if let Some(taxonomy_schema) = self
                    .executor
                    .graph
                    .schemas
                    .get("https://infinite-coding-loop.dass/schemas/taxonomy.schema.json")
                {
                    base.push_str("\nTaxonomy (Valid Kinds):\n");
                    base.push_str(&format!("```json\n{}\n```\n", taxonomy_schema));
                }
            }

            base.push_str(&format!(
                "1. Provide the {} content as the ONLY output in a single triple-backtick JSON code block.\n",
                target
            ));
            base.push_str("2. Do NOT include any intro, outro, or multiple blocks. Do NOT nest triple-backticks inside values.");
        }

        base
    }
}
