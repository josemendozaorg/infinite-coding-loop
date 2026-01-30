use crate::agents::cli_client::AiCliClient;
use crate::agents::generic::GenericAgent;

use crate::domain::types::AgentRole;
use crate::graph::DependencyGraph;
use crate::graph::executor::{GraphExecutor, InMemoryExecutor, Task};
use anyhow::Result;

pub struct Orchestrator<C: AiCliClient + Clone + Send + Sync + 'static> {
    pub app_id: String,
    pub app_name: String,
    pub work_dir: Option<String>,
    // New Graph Components
    executor: InMemoryExecutor,
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
        // Initialize Graph (Load Metamodel)
        let metamodel_json = include_str!("../../../spec/schemas/metamodel.schema.json");
        let graph = DependencyGraph::load_from_metamodel(metamodel_json)?;

        // Initialize Executor and Register Agents
        let mut executor = InMemoryExecutor::new(graph);

        // Dynamic Registration from Graph
        // Collect roles first to avoid borrowing conflict
        let loaded_roles: Vec<String> = executor.graph.loaded_agents.keys().cloned().collect();

        for role_str in loaded_roles {
            // Map string role to AgentRole Enum
            // In a real implementation this would be more robust or fully string-based
            let role_enum = match role_str.as_str() {
                "ProductManager" => Some(AgentRole::ProductManager),
                "Architect" => Some(AgentRole::Architect),
                "Engineer" => Some(AgentRole::Engineer),
                "QA" => Some(AgentRole::QA),
                "Manager" => Some(AgentRole::Manager),
                _ => None,
            };

            if let Some(r) = role_enum {
                // Parse Config
                let config_json = executor.graph.loaded_agents.get(&role_str).unwrap();
                let system_prompt =
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(config_json) {
                        v.get("system_prompt")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string()
                    } else {
                        "".to_string()
                    };

                executor.register_agent(Box::new(GenericAgent::new(
                    client.clone(),
                    r,
                    system_prompt,
                )));
            } else {
                eprintln!(
                    "Warning: Unknown agent role '{}' in configuration",
                    role_str
                );
            }
        }

        Ok(Self {
            app_id,
            app_name,
            work_dir: Some(work_dir.to_string_lossy().to_string()),
            executor,
            _client: std::marker::PhantomData,
        })
    }

    pub async fn run(&mut self, ui: &impl crate::interaction::UserInteraction) -> Result<()> {
        ui.log_info("Starting Graph-Driven Orchestration...");

        // 1. Determine Goal (e.g., Implement Feature)
        let feature_idea = ui
            .ask_for_feature("What feature do you want to build?")
            .await?;
        if feature_idea.is_empty() {
            return Ok(());
        }

        // 2. Query Graph for Dependencies of "SoftwareApplication" -> we need "Feature"
        // In our Metamodel: SoftwareApplication contains Feature.
        // Agent creates Feature.

        // This is a simplified "Goal Seeking" loop
        let _goal = "Feature";

        ui.start_step("Resolving Dependencies for Feature...");

        // Example: The Orchestrator knows it needs a "Feature".
        // It asks the Executor: "Who creates Feature?"
        // Executor checks Graph -> "Agent".

        // In this Proof of Concept, we manually trigger the flow derived from the graph

        // Step 1: Create Requirement (Agent: ProductManager)
        let req_template =
            self.executor
                .graph
                .get_prompt_template("ProductManager", "creates", "Requirement");
        let req_task = Task {
            id: "task_req_001".to_string(),
            description: format!("Analyze request: {}", feature_idea),
            inputs: vec![],
            prompt: req_template,
        };
        // The InMemoryExecutor currently returns a dummy artifact
        let _req_artifact = self
            .executor
            .dispatch_agent(AgentRole::ProductManager, req_task)
            .await?;
        ui.log_info("Product Manager produced Requirements");

        // Step 2: Create DesignSpec (Agent: Architect)
        let spec_template =
            self.executor
                .graph
                .get_prompt_template("Architect", "creates", "DesignSpec");
        let spec_task = Task {
            id: "task_spec_001".to_string(),
            description: "Design technical specification".to_string(),
            inputs: vec!["task_req_001".to_string()],
            prompt: spec_template,
        };
        let _spec_artifact = self
            .executor
            .dispatch_agent(AgentRole::Architect, spec_task)
            .await?;
        ui.log_info("Architect produced Design Spec");

        // Step 3: Create ProjectStructure (Agent: Architect)
        let struc_template =
            self.executor
                .graph
                .get_prompt_template("Architect", "creates", "ProjectStructure");
        let struc_task = Task {
            id: "task_struc_001".to_string(),
            description: "Define directory structure".to_string(),
            inputs: vec!["task_spec_001".to_string()],
            prompt: struc_template,
        };
        let _struc_artifact = self
            .executor
            .dispatch_agent(AgentRole::Architect, struc_task)
            .await?;
        ui.log_info("Architect defined Project Structure");

        // Step 4: Create Plan (Agent: Engineer)
        let plan_template = self
            .executor
            .graph
            .get_prompt_template("Engineer", "creates", "Plan");
        let plan_task = Task {
            id: "task_plan_001".to_string(),
            description: "Create implementation plan".to_string(),
            inputs: vec!["task_spec_001".to_string()],
            prompt: plan_template,
        };
        let _plan_artifact = self
            .executor
            .dispatch_agent(AgentRole::Engineer, plan_task)
            .await?;
        ui.log_info("Engineer created Plan");

        ui.end_step("Pipeline Complete (Graph-Driven)");
        Ok(())
    }
}
