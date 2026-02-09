use crate::agents::cli_client::AiCliClient;
use crate::agents::generic::GenericAgent;

use crate::domain::types::AgentRole;
use crate::graph::executor::{GraphExecutor, InMemoryExecutor, Task};
use crate::graph::{DependencyGraph, RelationCategory};
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ActionPlan {
    pub agent: String,
    pub target: String,
    pub relation: String,
    pub category: RelationCategory,
}

pub struct Orchestrator<C: AiCliClient + Clone + Send + Sync + 'static> {
    pub app_id: String,
    pub app_name: String,
    pub work_dir: Option<String>,
    // New Graph Components
    pub executor: InMemoryExecutor,
    // Tracking produced artifacts (State)
    pub artifacts: HashMap<String, serde_json::Value>, // EntityKind -> Last Produced Value
    pub verification_feedback: HashMap<String, String>, // Target -> Feedback
    pub verified_artifacts: std::collections::HashSet<String>, // Tracks those with score 1.0
    max_iterations: usize,
    // Marker for the client generic, or we can store it if needed later
    _client: std::marker::PhantomData<C>,
}

impl<C: AiCliClient + Clone + Send + Sync + 'static> Orchestrator<C> {
    pub async fn new(
        client: C,
        app_id: String,
        app_name: String,
        work_dir: std::path::PathBuf,
    ) -> Result<Self> {
        let metamodel_json = include_str!("../../../ontology-software-engineering/ontology.json");
        Self::new_with_metamodel(client, app_id, app_name, work_dir, metamodel_json, None).await
    }

    pub async fn new_with_metamodel(
        client: C,
        app_id: String,
        app_name: String,
        work_dir: std::path::PathBuf,
        metamodel_json: &str,
        ontology_base_path: Option<&std::path::Path>,
    ) -> Result<Self> {
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

            executor.register_agent(Box::new(GenericAgent::new(
                client.clone(),
                role,
                system_prompt,
            )));
        }

        Ok(Self {
            app_id,
            app_name,
            work_dir: Some(work_dir.to_string_lossy().to_string()),
            executor,
            artifacts: HashMap::new(),
            verification_feedback: HashMap::new(),
            verified_artifacts: std::collections::HashSet::new(),
            max_iterations: 10,
            _client: std::marker::PhantomData,
        })
    }

    pub async fn run(&mut self, ui: &impl crate::interaction::UserInteraction) -> Result<()> {
        ui.log_info("Starting Generic Graph-Driven Orchestration...");

        // 1. Initial Input
        let initial_goal = ui
            .ask_for_feature("What feature do you want to build?")
            .await?;
        if initial_goal.is_empty() {
            return Ok(());
        }

        // Feed initial input as a "SoftwareApplication" signal to start the orchestration
        self.artifacts.insert(
            "SoftwareApplication".to_string(),
            serde_json::json!({ "name": self.app_name.clone(), "goal": initial_goal }),
        );

        // 2. Generic Execution Loop (State Machine)
        let mut iterations = 0;

        while iterations < self.max_iterations {
            iterations += 1;
            ui.start_step(&format!(
                "Iteration {} - Evaluating Graph State...",
                iterations
            ));

            let next_actions = self.identify_next_actions();
            if next_actions.is_empty() {
                ui.log_info(&format!(
                    "No more actions identified. Current artifacts: {:?}",
                    self.artifacts.keys().collect::<Vec<_>>()
                ));
                break;
            }

            for action in next_actions {
                ui.log_info(&format!(
                    "Dispatched Action: {} {} {}",
                    action.agent, action.relation, action.target
                ));
                self.execute_action(action, ui).await?;
            }

            if !ui.confirm("Proceed to next iteration?").await? {
                ui.log_info("Iteration paused by user.");
                break;
            }
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
            let category = RelationCategory::from_str(relation);

            if self.executor.graph.is_agent(source_kind) {
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
                        if self.artifacts.contains_key(target_kind)
                            && self.verification_feedback.contains_key(target_kind)
                        {
                            plans.push(ActionPlan {
                                agent: source_kind.clone(),
                                target: target_kind.clone(),
                                relation: relation.clone(),
                                category,
                            });
                        }
                    }
                    _ => {
                        // All other categories (Context, Other) are informative only
                        // and do not trigger ActionPlans.
                    }
                }
            }
        }

        // Only return the first logically sound action for this iteration for this POC
        if !plans.is_empty() {
            vec![plans[0].clone()]
        } else {
            vec![]
        }
    }

    async fn execute_action(
        &mut self,
        action: ActionPlan,
        ui: &impl crate::interaction::UserInteraction,
    ) -> Result<()> {
        let agent_role = AgentRole::from(action.agent.as_str());

        // Find Input Context
        // In a more advanced version, we'd filter based on Context relations in the graph
        let mut context_map = HashMap::new();
        let mut reference_instructions = String::new();

        for (kind, val) in &self.artifacts {
            ui.log_info(&format!("  [Context] Retrieving: {}", kind));

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
        // If not, we append the context automatically.
        if final_prompt.contains("{{source_content}}") {
            final_prompt = final_prompt.replace("{{source_content}}", &context);
        } else if final_prompt.contains("{{input}}") {
            // Legacy support or specific alias
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

        let task = Task {
            id: format!("task_{}_{}", action.relation, action.target),
            description: format!("{} {}", action.relation, action.target),
            inputs: vec![],
            prompt: Some(enhanced_prompt),
        };

        let result = self.executor.dispatch_agent(agent_role, task).await?;

        // Semantic Result Handling
        match action.category {
            RelationCategory::Verification => {
                // Verification results go to feedback map
                let feedback = result
                    .get("feedback")
                    .and_then(|v| v.as_str())
                    .or_else(|| result.get("test_results").and_then(|v| v.as_str()))
                    .unwrap_or("No detailed feedback provided.");

                let score = result.get("score").and_then(|v| v.as_f64()).unwrap_or(1.0);

                if score < 1.0 {
                    self.verification_feedback
                        .insert(action.target.clone(), feedback.to_string());
                    self.verified_artifacts.remove(&action.target);
                } else {
                    // Passed! Clear feedback and mark as verified
                    self.verification_feedback.remove(&action.target);
                    self.verified_artifacts.insert(action.target.clone());
                }
            }
            RelationCategory::Refinement | RelationCategory::Creation => {
                // Validate Result against Schema (only for artifacts, not necessarily for verification reports yet)
                // We spawn blocking because jsonschema compilation/validation can trigger blocking operations (e.g. if refs are resolved)
                // or simply be computationally expensive, which effectively blocks the async runtime if taking too long.
                let graph = self.executor.graph.clone();
                let target = action.target.clone();
                let result_clone = result.clone();

                let validation_result = tokio::task::spawn_blocking(move || {
                    graph.validate_artifact(&target, &result_clone)
                })
                .await?;

                if let Err(e) = validation_result {
                    return Err(anyhow::anyhow!(
                        "Artifact validation failed for {}: {}. Result: {}",
                        action.target,
                        e,
                        result
                    ));
                }

                self.artifacts.insert(action.target.clone(), result.clone());

                // If it was a refinement or creation, it's no longer "verified"
                self.verified_artifacts.remove(&action.target);

                // If it was a refinement, we can clear the feedback
                if action.category == RelationCategory::Refinement {
                    self.verification_feedback.remove(&action.target);
                }
            }
            _ => {
                // Other or Context categories - should not be triggered by scheduler normally
                self.artifacts.insert(action.target.clone(), result.clone());
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
        let work_dir = self.work_dir.as_deref().unwrap_or(".");

        let mut base = format!(
            "{}\n\nPlease create or modify the {} artifact.",
            prompt, target
        );

        base.push_str("\n\n**Persistence Instructions**:\n");
        base.push_str(&format!(
            "1. You MUST create or update the following file: `{}` in the directory: `{}`\n",
            filename, work_dir
        ));
        base.push_str("2. Use your tools to persist this content to disk. Do NOT just output the text; ensure the file is written.\n");

        if is_code {
            base.push_str("\n**Output Rules**:\n");
            base.push_str("1. Return a JSON object with a `files` key containing the list of filenames you created or modified.\n");
            base.push_str("2. Example: `{\"files\": [\"main.rs\", \"Cargo.toml\"], \"main_file\": \"main.rs\"}`\n");
        } else {
            base.push_str(&format!(
                "\n**Output Rules**:\n1. Output the {} content in a strict JSON code block.\n",
                target
            ));
            base.push_str("2. IMPORTANT: Do NOT nest triple-backticks (```) inside any JSON string values. If you need to include code or schemas, provide them as plain text or use alternative formatting.");
        }

        base
    }
}
