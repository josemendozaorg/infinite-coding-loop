use anyhow::Result;
use crate::spec::feature_spec::FeatureSpec;
use crate::plan::action::ImplementationPlan;
use crate::gates::safety::SafetyChecker;
use crate::agents::cli_client::AiCliClient;

pub struct Planner<C: AiCliClient> {
    client: C,
}

impl<C: AiCliClient> Planner<C> {
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
            Output valid JSON for ImplementationPlan. Use 'type' and 'payload' in Actions.", 
            spec_json
        );

        while attempts < max_attempts {
            attempts += 1;
            let response = self.client.prompt(&current_context)?;

            let plan: ImplementationPlan = match serde_json::from_str(&response) {
                Ok(p) => p,
                Err(e) => {
                     // Basic retry for JSON error (omitted complex extraction for brevity)
                     current_context = format!("Invalid JSON for Plan: {}. Fix it.", e);
                     continue;
                }
            };

            // Gate Check: Safety
            let report = SafetyChecker::check_plan(&plan)?;
            if !report.is_safe {
                let warnings = report.warnings.join(", ");
                current_context = format!(
                    "Safety Gate Failed: {}. \
                    REMOVE all destructive commands from the plan.", 
                    warnings
                );
                continue;
            }

            return Ok(plan);
        }

        Err(anyhow::anyhow!("Planner failed to create safe plan after {} attempts", max_attempts))
    }
}
