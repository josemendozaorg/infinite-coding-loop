use crate::agents::Agent;
use crate::agents::cli_client::AiCliClient;
use crate::clover::Verifiable;
use crate::plan::action::ImplementationPlan;
use crate::spec::feature_spec::FeatureSpec;
use anyhow::Result;

pub struct Engineer<C: AiCliClient> {
    client: C,
}

impl<C: AiCliClient> Agent for Engineer<C> {
    fn role(&self) -> &str {
        "Engineer"
    }
}

impl<C: AiCliClient> Engineer<C> {
    pub fn new(client: C) -> Self {
        Self { client }
    }

    /// The SOP: Spec -> Plan -> Safety Check -> Refine.
    pub fn plan(&self, spec: &FeatureSpec) -> Result<ImplementationPlan> {
        let mut attempts = 0;
        let max_attempts = 5;

        let spec_json = serde_json::to_string(spec).unwrap_or_default();
        let mut current_context = format!(
            "Create an ImplementationPlan for this Spec:\n{}\n \
            Output valid JSON for ImplementationPlan with fields: feature_id (string), steps (list of Action). \
            Action types: \
            - CreateFile {{ path, content }} \
            - ModifyFile {{ path, new_content }} \
            - RunCommand {{ command, cwd (optional), must_succeed (bool) }} \
            - Verify {{ test_command }}",
            spec_json
        );

        while attempts < max_attempts {
            attempts += 1;
            let response = self.client.prompt(&current_context)?;

            let cleaned_response = if let Some(start) = response.find("```") {
                let after_structure = &response[start + 3..];
                if let Some(end) = after_structure.find("```") {
                    let content = &after_structure[..end].trim();
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

            let plan: ImplementationPlan = match serde_json::from_str(cleaned_response) {
                Ok(p) => p,
                Err(e) => {
                    current_context = format!(
                        "Invalid JSON for Plan: {}. Fix it. Input was: {}",
                        e, cleaned_response
                    );
                    continue;
                }
            };

            // Gate Check: Safety (Clover)
            if let Err(e) = plan.verify() {
                current_context = format!(
                    "Safety Gate Failed: {}. \
                    REMOVE all destructive commands from the plan.",
                    e
                );
                continue;
            }

            return Ok(plan);
        }

        Err(anyhow::anyhow!(
            "Engineer failed to create safe plan after {} attempts",
            max_attempts
        ))
    }
}
