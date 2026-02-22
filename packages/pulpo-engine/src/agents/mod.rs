use crate::domain::types::AgentRole;
use crate::graph::executor::Task;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

pub mod cli_client;
pub mod generic;

#[async_trait]
pub trait Agent: Send + Sync {
    /// The unique role of this agent
    fn role(&self) -> AgentRole;

    /// Execute a task assigned by the Graph Engine
    async fn execute(&self, task: Task) -> Result<Value>;
}
