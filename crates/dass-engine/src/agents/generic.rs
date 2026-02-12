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
        let mut end_search_pos = input.len();

        while let Some(end) = input[..end_search_pos].rfind("```") {
            let mut start_search_pos = end;
            while let Some(start) = input[..start_search_pos].rfind("```") {
                let after_start = &input[start + 3..];

                // Skip language identifier (e.g., "json\n", "yaml\n")
                // Only skip if it's a single word followed by a newline
                let mut content_start_offset = 0;
                if let Some(newline_pos) = after_start.find('\n') {
                    let first_line = &after_start[..newline_pos].trim();
                    // Alphanumeric + underscores is typical for language tags
                    if !first_line.is_empty()
                        && first_line.chars().all(|c| c.is_alphanumeric() || c == '_')
                    {
                        content_start_offset = newline_pos + 1;
                    }
                }

                if start + 3 + content_start_offset < end {
                    let content = input[start + 3 + content_start_offset..end].trim();

                    // Check if it's valid JSON or YAML
                    let is_json = serde_json::from_str::<Value>(content).is_ok();
                    let is_yaml = serde_yaml::from_str::<Value>(content).is_ok();

                    if is_json || is_yaml {
                        return content.to_string(); // Found the largest valid block ending here!
                    }
                }
                start_search_pos = start;
            }
            // If no start worked for this 'end', move end back
            end_search_pos = end;
        }

        input.trim().to_string()
    }
    fn extract_json_object(&self, input: &str) -> Option<String> {
        let start = input.find('{')?;
        let mut balance = 0;
        let mut in_string = false;
        let mut escape = false;

        for (i, c) in input[start..].char_indices() {
            if escape {
                escape = false;
                continue;
            }
            if c == '\\' {
                escape = true;
                continue;
            }
            if c == '"' {
                in_string = !in_string;
                continue;
            }
            if !in_string {
                if c == '{' {
                    balance += 1;
                } else if c == '}' {
                    balance -= 1;
                    if balance == 0 {
                        return Some(input[start..=start + i].to_string());
                    }
                }
            }
        }
        None
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

        let response = self.client.prompt(&full_prompt).await?;
        let cleaned = self.clean_response(&response);

        // Try parsing as JSON first
        if let Ok(val) = serde_json::from_str::<Value>(&cleaned) {
            return Ok(val);
        }

        // Try parsing as YAML (requirements use YAML)
        if let Ok(val) = serde_yaml::from_str::<Value>(&cleaned) {
            return Ok(val);
        }

        // Fallback: Try to extract JSON object from the raw input
        if let Some(extracted) = self.extract_json_object(&response) {
            if let Ok(val) = serde_json::from_str::<Value>(&extracted) {
                return Ok(val);
            }
        }

        // Return error if neither (or return raw string wrapper?)
        // For 'infinite-coding-loop', we expect structured data.
        Err(anyhow::anyhow!(
            "Failed to parse response as JSON or YAML. Response: {}",
            cleaned
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockClient;
    #[async_trait]
    impl AiCliClient for MockClient {
        async fn prompt(&self, _p: &str) -> Result<String> {
            Ok("".into())
        }
    }

    #[test]
    fn test_clean_response_multi_block() {
        let agent = GenericAgent::new(MockClient, "ProductManager".into(), "".into());

        let multi = r#"Here is a preview:
```json
{"preview": true}
```
Now here is the real one:
```json
{"real": "deal"}
```
Some final text."#;
        let cleaned = agent.clean_response(multi);
        assert_eq!(cleaned, r#"{"real": "deal"}"#);
    }

    #[test]
    fn test_clean_response_nested() {
        let agent = GenericAgent::new(MockClient, "ProductManager".into(), "".into());

        let nested = r#"```json
{
  "data": "Here is some nested json: ```json {\"test\": 1} ```"
}
```"#;
        let cleaned = agent.clean_response(nested);
        assert_eq!(
            cleaned,
            r#"{
  "data": "Here is some nested json: ```json {\"test\": 1} ```"
}"#
        );
    }

    #[test]
    fn test_extract_json_object() {
        let agent = GenericAgent::new(MockClient, "ProductManager".into(), "".into());
        let input = r#"Here is the file:
{"files": ["test.rs"]}
Hope you like it."#;
        let extracted = agent
            .extract_json_object(input)
            .expect("Should extract JSON");
        assert_eq!(extracted, r#"{"files": ["test.rs"]}"#);

        let input_nested = r#"start {"a": {"b": 1}} end"#;
        let extracted_nested = agent
            .extract_json_object(input_nested)
            .expect("Should extract nested JSON");
        assert_eq!(extracted_nested, r#"{"a": {"b": 1}}"#);
    }
}
