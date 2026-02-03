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
    executor: InMemoryExecutor,
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
        let metamodel_json =
            include_str!("../../../ontology-software-engineering/artifact/schema/metamodel.schema.json");
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

        // Feed initial input as a "Requirement" (In a real system, this would be a "SoftwareApplication" signal)
        self.artifacts.insert(
            "FeatureIdea".to_string(),
            serde_json::json!({ "goal": initial_goal }),
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
                ui.log_info("No more actions identified. Flow complete.");
                break;
            }

            for action in next_actions {
                ui.log_info(&format!(
                    "Dispatched Action: {} {} {}",
                    action.agent, action.relation, action.target
                ));
                self.execute_action(action).await?;
            }
        }

        ui.end_step("Goal achieved or max iterations reached.");
        Ok(())
    }

    fn identify_next_actions(&self) -> Vec<ActionPlan> {
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

    async fn execute_action(&mut self, action: ActionPlan) -> Result<()> {
        let agent_role = AgentRole::from(action.agent.as_str());

        // Find Input Context (Simplified: take all existing artifacts for now)
        let mut context = serde_json::to_string_pretty(&self.artifacts).unwrap_or_default();

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

        let final_prompt = prompt_template
            .replace("{{source_content}}", &context)
            .replace("{{source}}", &action.agent)
            .replace("{{relation}}", &action.relation)
            .replace("{{target}}", &action.target);

        let enhanced_prompt = self.enhance_prompt(
            final_prompt,
            &action.target,
            &format!("{}.json", action.target.to_lowercase()),
        );

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
                if let Err(e) = self
                    .executor
                    .graph
                    .validate_artifact(&action.target, &result)
                {
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
                self.artifacts.insert(action.target.clone(), result);
            }
        }

        Ok(())
    }

    fn enhance_prompt(&self, prompt: String, target: &str, filename: &str) -> String {
        format!(
            "{}\n\n**CRITICAL**: You MUST use your file-writing tool to save this {} to `{}` in the current directory. After saving, output ONLY the {} content in a code block.",
            prompt, target, filename, target
        )
    }
}
