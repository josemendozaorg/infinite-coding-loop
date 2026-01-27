use serde::{Deserialize, Serialize};
use uuid::Uuid;
use anyhow::Result;

/// Represents an Atomic Requirement (The Truth).
/// defined in the "Product" phase of the DASS process.
///
/// An atomic requirement must describing exactly one verifiable logic constraint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Requirement {
    /// Unique Identifier for the requirement.
    pub id: String,
    /// The user story: "As a <role>, I want <feature> so that <benefit>".
    pub user_story: String,
    /// A list of verifiable boolean statements that define success.
    /// Each statement must imply a Test Oracle (True/False).
    pub acceptance_criteria: Vec<String>,
}

impl Requirement {
    /// Creates a new Atomic Requirement.
    pub fn new(user_story: impl Into<String>, acceptance_criteria: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(), // Default to UUID, but allow String override via serialization
            user_story: user_story.into(),
            acceptance_criteria,
        }
    }

    /// Loads a list of requirements from a YAML string.
    pub fn load_many_from_yaml(yaml_content: &str) -> Result<Vec<Self>> {
        let reqs: Vec<Self> = serde_yaml::from_str(yaml_content)?;
        Ok(reqs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_serialization() {
        let req = Requirement::new(
            "As a user I want to login",
            vec!["Login fails with connection error".to_string()],
        );
        let yaml = serde_yaml::to_string(&vec![&req]).unwrap();
        let loaded = Requirement::load_many_from_yaml(&yaml).unwrap();
        assert_eq!(loaded[0], req);
    }
}
