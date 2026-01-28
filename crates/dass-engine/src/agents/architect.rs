use crate::agents::cli_client::AiCliClient;
use crate::clover::{ConsistencyCheck, Verifiable};
use crate::product::requirement::Requirement;
use crate::spec::feature_spec::FeatureSpec;
use anyhow::Result;

pub struct Architect<C: AiCliClient> {
    client: C,
}

impl<C: AiCliClient> Architect<C> {
    pub fn new(client: C) -> Self {
        Self { client }
    }

    /// The SOP: Take Requirements -> Define Tech Stack & Architecture -> Generate Standard Primitives.
    pub fn establish_architecture(
        &self,
        reqs: &[Requirement],
    ) -> Result<Vec<crate::domain::primitive::Primitive>> {
        let req_list_str = serde_yaml::to_string(reqs).unwrap_or_default();
        let prompt = format!(
            "You are the Chief Architect. Based on these requirements:\n{}\n\
            Define the Technology Stack, Architecture Pattern, and Coding Standards.\n\
            Output a JSON list of objects associated with the following Structure:\n\
            struct Standard {{\n\
                category: String, // 'TechStack', 'Architecture', 'CodingStyle'\n\
                rules: Vec<String>, // List of rules/decisions\n\
                command_template: Option<String> // Optional command (e.g. 'cargo check')\n\
            }}\n\
            Example Output:\n\
            [\n\
                {{ \"category\": \"TechStack\", \"rules\": [\"Rust\", \"Axum\"], \"command_template\": null }},\n\
                {{ \"category\": \"Architecture\", \"rules\": [\"Hexagonal\"], \"command_template\": null }},\n\
                {{ \"category\": \"CodingStyle\", \"rules\": [\"No unwrap\"], \"command_template\": \"cargo clippy\" }}\n\
            ]",
            req_list_str
        );

        let response = self.client.prompt(&prompt)?;
        // 1. Try to parse JSON (robustly)
        let cleaned_response = if let Some(start) = response.find("```") {
            let after_structure = &response[start + 3..];
            if let Some(end) = after_structure.find("```") {
                let content = &after_structure[..end].trim();
                // Strip optional 'json' or other tag from first line if present
                if let Some(idx) = content.find(char::is_whitespace) {
                    if content[..idx].to_lowercase().contains("json") {
                        &content[idx..]
                    } else {
                        content
                    }
                } else {
                    content
                }
            } else {
                response.trim()
            }
        } else {
            response.trim()
        };

        #[derive(serde::Deserialize)]
        struct StandardDto {
            category: String,
            rules: Vec<String>,
            command_template: Option<String>,
        }

        let standards: Vec<StandardDto> = serde_json::from_str(cleaned_response).map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse Architecture Standards JSON: {}. Input: {}",
                e,
                cleaned_response
            )
        })?;

        let primitives = standards
            .into_iter()
            .map(|s| crate::domain::primitive::Primitive::Standard {
                category: s.category,
                rules: s.rules,
                command_template: s.command_template,
            })
            .collect();

        Ok(primitives)
    }

    /// The SOP: Take Requirements -> Design Spec -> Check Completeness -> Refine.
    pub fn design(&self, feature_id: &str, reqs: &[Requirement]) -> Result<FeatureSpec> {
        let mut attempts = 0;
        let max_attempts = 5;

        // Context construction
        let req_list_str = serde_yaml::to_string(reqs).unwrap_or_default();
        let mut current_context = format!(
            "Design a FeatureSpec for feature '{}' based on these requirements:\n{}\n \
            Output valid JSON for the FeatureSpec struct with fields: \
            id (string), requirement_ids (list of strings), ui_spec (markdown string), \
            logic_spec (markdown string), data_spec (markdown string), verification_plan (markdown string).",
            feature_id, req_list_str
        );

        while attempts < max_attempts {
            attempts += 1;
            let response = self.client.prompt(&current_context)?;

            // 1. Try to parse JSON
            // 1. Try to parse JSON (robustly)
            let cleaned_response = if let Some(start) = response.find("```") {
                let after_structure = &response[start + 3..];
                if let Some(end) = after_structure.find("```") {
                    let content = &after_structure[..end].trim();
                    // Strip optional 'json' or other tag from first line if present
                    if let Some(idx) = content.find(char::is_whitespace) {
                        if content[..idx].to_lowercase().contains("json") {
                            &content[idx..]
                        } else {
                            content
                        }
                    } else {
                        content
                    }
                } else {
                    response.trim()
                }
            } else {
                response.trim()
            };

            let mut spec: FeatureSpec = match serde_json::from_str(cleaned_response) {
                Ok(s) => s,
                Err(e) => {
                    current_context = format!(
                        "Invalid JSON: {}. Please fix. Input was: {}",
                        e, cleaned_response
                    );
                    continue;
                }
            };

            // Ensure ID matches
            spec.id = feature_id.to_string();
            // Ensure Req IDs are linked
            spec.requirement_ids = reqs.iter().map(|r| r.id.clone()).collect();

            // 2. Gate Check: Completeness & Consistency (Clover)
            if let Err(e) = spec.verify() {
                current_context = format!("Spec Gate Failed: {}. Please fill all sections.", e);
                continue;
            }
            if let Err(e) = spec.check_consistency(reqs) {
                current_context = format!(
                    "Spec Gate Failed: {}. Please ensure all reqs are covered.",
                    e
                );
                continue;
            }

            return Ok(spec);
        }

        Err(anyhow::anyhow!(
            "Architect failed to design valid spec after {} attempts",
            max_attempts
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::cli_client::mocks::MockCliClient;

    #[test]
    fn test_architect_design_flow() {
        let reqs = vec![Requirement::new(
            "User wants X",
            vec!["Must be X".to_string()],
        )];

        // Mock responses: 1. Bad JSON, 2. Good JSON
        let mock_client = MockCliClient::new(vec![
            "Not JSON".to_string(),
            r#"{
                "id": "Test",
                "requirement_ids": [],
                "ui_spec": "UI",
                "logic_spec": "Logic",
                "data_spec": "Data",
                "verification_plan": "Verif"
            }"#
            .to_string(),
        ]);

        let architect = Architect::new(mock_client);
        let spec = architect.design("Test", &reqs).expect("Should succeed");

        assert_eq!(spec.id, "Test");
        assert_eq!(spec.requirement_ids.len(), 1); // Should link req
    }
}
