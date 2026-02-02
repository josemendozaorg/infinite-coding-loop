use crate::agents::Agent;
use crate::agents::cli_client::AiCliClient;
use crate::domain::types::AgentRole;
use crate::graph::executor::Task;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

pub struct GenericAgent<C: AiCliClient> {
    client: C,
    role: AgentRole,
    system_prompt: String,
}

impl<C: AiCliClient> GenericAgent<C> {
    pub fn new(client: C, role: AgentRole, system_prompt: String) -> Self {
        Self {
            client,
            role,
            system_prompt,
        }
    }

    fn clean_response(&self, input: &str) -> String {
        // 1. Try to find markdown code blocks
        if let Some(start) = input.find("```") {
            let after = &input[start + 3..];
            // Skip language identifier (e.g. "json", "yaml")
            let start_content = if let Some(newline) = after.find('\n') {
                newline + 1
            } else {
                0
            };

            // Allow for ```` to handle nested blocks? No, standard is ```
            if let Some(end) = after[start_content..].find("```") {
                return after[start_content..start_content + end].trim().to_string();
            }
        }
        // Fallback: assume the whole response is the content if no code blocks
        input.trim().to_string()
    }
}

#[async_trait]
impl<C: AiCliClient + Send + Sync> Agent for GenericAgent<C> {
    fn role(&self) -> AgentRole {
        self.role.clone()
    }

    async fn execute(&self, task: Task) -> Result<Value> {
        let prompt = task.prompt.ok_or_else(|| {
            anyhow::anyhow!("GenericAgent requires a 'prompt' in the Task definition.")
        })?;

        let full_prompt = if !self.system_prompt.is_empty() {
            format!("{}\n\n{}", self.system_prompt, prompt)
        } else {
            prompt
        };

        let response = self.client.prompt(&full_prompt)?;
        let cleaned = self.clean_response(&response);

        // Try parsing as JSON first
        if let Ok(val) = serde_json::from_str::<Value>(&cleaned) {
            return Ok(val);
        }

        // Try parsing as YAML (requirements use YAML)
        if let Ok(val) = serde_yaml::from_str::<Value>(&cleaned) {
            return Ok(val);
        }

        // Return error if neither (or return raw string wrapper?)
        // For 'infinite-coding-loop', we expect structured data.
        Err(anyhow::anyhow!(
            "Failed to parse response as JSON or YAML. Response: {}",
            cleaned
        ))
    }
}
