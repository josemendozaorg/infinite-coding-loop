use crate::clover::{ConsistencyCheck, Verifiable};
use crate::product::requirement::Requirement;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Represents a Feature Specification (The Contract).
/// defined in the "Design" phase of the DASS process.
///
/// A Feature Spec must cover all linked Requirements and provide
/// specific technical details for implementation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FeatureSpec {
    /// Unique Identifier for the spec (usually matches the Feature Name/ID).
    pub id: String,
    /// Links to the Atomic Requirements this spec satisfies.
    pub requirement_ids: Vec<String>,
    /// UI Specification (Markdown content).
    pub ui_spec: String,
    /// Business Logic Specification (Markdown content).
    pub logic_spec: String,
    /// Data Model Specification (Markdown content).
    pub data_spec: String,
    /// Verification Plan (Markdown content with PBT definitions).
    pub verification_plan: String,
}

impl FeatureSpec {
    pub fn new(id: impl Into<String>, req_ids: Vec<String>) -> Self {
        Self {
            id: id.into(),
            requirement_ids: req_ids,
            ui_spec: String::new(),
            logic_spec: String::new(),
            data_spec: String::new(),
            verification_plan: String::new(),
        }
    }

    /// Checks if the spec is "complete" (no empty sections).
    /// This is a basic Gate check.
    pub fn is_complete(&self) -> bool {
        !self.ui_spec.is_empty()
            && !self.logic_spec.is_empty()
            && !self.data_spec.is_empty()
            && !self.verification_plan.is_empty()
    }
}

impl Verifiable for FeatureSpec {
    fn verify(&self) -> Result<bool> {
        if !self.is_complete() {
            return Err(anyhow::anyhow!(
                "Spec '{}' is incomplete. Missing sections.",
                self.id
            ));
        }
        Ok(true)
    }
}

impl ConsistencyCheck<[Requirement]> for FeatureSpec {
    fn check_consistency(&self, reqs: &[Requirement]) -> Result<()> {
        let spec_req_ids = &self.requirement_ids;

        for req in reqs {
            if !spec_req_ids.contains(&req.id) {
                return Err(anyhow::anyhow!(
                    "Requirement {} not covered by Spec {}",
                    req.id,
                    self.id
                ));
            }
        }

        // Reverse check: Does the spec claim to cover a req that doesn't exist?
        let input_ids: Vec<String> = reqs.iter().map(|r| r.id.clone()).collect();
        for claimed_id in spec_req_ids {
            if !input_ids.contains(claimed_id) {
                return Err(anyhow::anyhow!(
                    "Spec claims to cover unknown Requirement {}",
                    claimed_id
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clover_completeness() {
        let spec = FeatureSpec::new("test", vec![]);
        assert!(spec.verify().is_err());
    }

    #[test]
    fn test_clover_coverage() {
        let req = Requirement::new("story", vec![]);
        let spec = FeatureSpec::new("test", vec![]);
        assert!(spec.check_consistency(&vec![req]).is_err());
    }
}
