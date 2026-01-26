use anyhow::Result;
use crate::product::requirement::Requirement;
use crate::spec::feature_spec::FeatureSpec;
use crate::gates::consistency::SpecValidator;
use crate::agents::cli_client::AiCliClient;

pub struct Architect<C: AiCliClient> {
    client: C,
}

impl<C: AiCliClient> Architect<C> {
    pub fn new(client: C) -> Self {
        Self { client }
    }

    /// The SOP: Take Requirements -> Design Spec -> Check Completeness -> Refine.
    pub fn design(&self, feature_id: &str, reqs: &[Requirement]) -> Result<FeatureSpec> {
        let mut attempts = 0;
        let max_attempts = 5;
        
        // Context construction
        let req_list_str = serde_yaml::to_string(reqs).unwrap_or_default();
        let mut current_context = format!(
            "Design a FeatureSpec for feature '{}' based on these requirements:\n{}\n \
            Output valid JSON for the FeatureSpec struct.", 
            feature_id, req_list_str
        );

        while attempts < max_attempts {
            attempts += 1;
            let response = self.client.prompt(&current_context)?;

            // 1. Try to parse JSON
            let mut spec: FeatureSpec = match serde_json::from_str(&response) {
                Ok(s) => s,
                Err(e) => {
                    // Try to extract JSON block if wrapped in markdown
                    if let Some(start) = response.find("```json") {
                        if let Some(end) = response[start..].find("```") {
                             let json_part = &response[start+7..start+end];
                             if let Ok(s) = serde_json::from_str(json_part) {
                                 s
                             } else {
                                  current_context = format!("Invalid JSON: {}. Please fix.", e);
                                  continue;
                             }
                        } else {
                            current_context = format!("Invalid JSON: {}. Please fix.", e);
                            continue;
                        }
                    } else {
                        current_context = format!("Invalid JSON: {}. Please fix.", e);
                        continue;
                    }
                }
            };
            
            // Ensure ID matches
            spec.id = feature_id.to_string(); 
            // Ensure Req IDs are linked
            spec.requirement_ids = reqs.iter().map(|r| r.id).collect();

            // 2. Gate Check: Completeness & Consistency
            if let Err(e) = SpecValidator::check_completeness(&spec) {
                 current_context = format!("Spec Gate Failed: {}. Please fill all sections.", e);
                 continue;
            }
             if let Err(e) = SpecValidator::check_coverage(&spec, reqs) {
                 current_context = format!("Spec Gate Failed: {}. Please ensure all reqs are covered.", e);
                 continue;
            }

            return Ok(spec);
        }

        Err(anyhow::anyhow!("Architect failed to design valid spec after {} attempts", max_attempts))
    }
}
