use crate::spec::feature_spec::FeatureSpec;
use crate::product::requirement::Requirement;
use anyhow::Result;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct SpecValidator;

impl SpecValidator {
    /// Checks if a spec is structurally complete.
    pub fn check_completeness(spec: &FeatureSpec) -> Result<()> {
        if !spec.is_complete() {
            return Err(anyhow::anyhow!("Spec '{}' is incomplete. Missing sections.", spec.id));
        }
        Ok(())
    }

    /// Checks if the spec covers all provided requirements.
    pub fn check_coverage(spec: &FeatureSpec, reqs: &[Requirement]) -> Result<()> {
        let spec_req_ids = &spec.requirement_ids;
        
        for req in reqs {
            if !spec_req_ids.contains(&req.id) {
                // Warning: loose heuristic. A spec *might* not need to cover all passed reqs if
                // we are validating a subset, but typically a mismatch is bad.
                // For strict DASS, let's enforce that every req in the input list 
                // MUST be claimed by the spec.
                return Err(anyhow::anyhow!("Requirement {} not covered by Spec {}", req.id, spec.id));
            }
        }

        // Reverse check: Does the spec claim to cover a req that doesn't exist?
        let input_ids: Vec<String> = reqs.iter().map(|r| r.id.clone()).collect();
        for claimed_id in spec_req_ids {
            if !input_ids.contains(claimed_id) {
                 return Err(anyhow::anyhow!("Spec claims to cover unknown Requirement {}", claimed_id));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incomplete_spec_fails() {
        let spec = FeatureSpec::new("test", vec![]);
        assert!(SpecValidator::check_completeness(&spec).is_err());
    }

    #[test]
    fn test_coverage_mismatch() {
        let req = Requirement::new("story", vec![]);
        let spec = FeatureSpec::new("test", vec![]); // No reqs linked
        assert!(SpecValidator::check_coverage(&spec, &[req]).is_err());
    }
}
