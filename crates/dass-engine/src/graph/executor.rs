use crate::agents::Agent;
use crate::domain::types::AgentRole;
use crate::graph::DependencyGraph;
use anyhow::Result;
use async_trait::async_trait;
use petgraph::graph::NodeIndex;
use serde_json::Value;
use std::collections::HashMap;

// Task definition matching Plan.tasks schema partially
#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub inputs: Vec<String>, // IDs of input artifacts
    pub prompt: Option<String>,
}

#[async_trait]
pub trait GraphExecutor {
    /// Given a node in the graph, identify which nodes must be executed first.
    async fn resolve_dependencies(&self, node_idx: NodeIndex) -> Result<Vec<NodeIndex>>;

    /// Assign a task to an agent role and await the result (Artifact).
    async fn dispatch_agent(&self, role: AgentRole, task: Task) -> Result<Value>;
}

pub struct InMemoryExecutor {
    pub graph: DependencyGraph,
    agents: HashMap<AgentRole, Box<dyn Agent>>,
}

impl InMemoryExecutor {
    pub fn new(graph: DependencyGraph) -> Self {
        Self {
            graph,
            agents: HashMap::new(),
        }
    }

    pub fn register_agent(&mut self, agent: Box<dyn Agent>) {
        self.agents.insert(agent.role(), agent);
    }
}

#[async_trait]
impl GraphExecutor for InMemoryExecutor {
    async fn resolve_dependencies(&self, node_idx: NodeIndex) -> Result<Vec<NodeIndex>> {
        let neighbors = self
            .graph
            .graph
            .neighbors_directed(node_idx, petgraph::Direction::Incoming);
        Ok(neighbors.collect())
    }

    async fn dispatch_agent(&self, role: AgentRole, task: Task) -> Result<Value> {
        if let Some(agent) = self.agents.get(&role) {
            println!(
                "Thinking... [Agent: {:?}] executing Task: {}",
                role, task.description
            );
            return agent.execute(task).await;
        }

        // Fallback or Error if agent not found
        Err(anyhow::anyhow!("No agent registered for role: {:?}", role))
    }
}
