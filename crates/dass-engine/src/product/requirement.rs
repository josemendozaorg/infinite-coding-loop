use crate::clover::Verifiable;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

    pub fn load_many_from_yaml(yaml_content: &str) -> Result<Vec<Self>> {
        let reqs: Vec<Self> = serde_yaml::from_str(yaml_content)?;
        Ok(reqs)
    }
}

impl Verifiable for Requirement {
    fn verify(&self) -> Result<bool> {
        let mut score = 100u8;
        let mut notes = Vec::new();

        // Heuristic 1: Length check
        if self.user_story.len() < 10 {
            score = score.saturating_sub(50);
            notes.push("User story is suspiciously short.".to_string());
        }

        // Heuristic 2: Acceptance Criteria presence
        if self.acceptance_criteria.is_empty() {
            return Err(anyhow::anyhow!(
                "Quality Gate Failed: No acceptance criteria provided for '{}'",
                self.user_story
            ));
        }

        // Heuristic 3: Ambiguous keywords
        let ambiguous_words = ["fast", "user friendly", "modern", "better", "clean"];
        for word in ambiguous_words {
            if self.user_story.to_lowercase().contains(word) {
                score = score.saturating_sub(20);
                notes.push(format!("Contains subjective term: '{}'", word));
            }
            for criteria in &self.acceptance_criteria {
                if criteria.to_lowercase().contains(word) {
                    score = score.saturating_sub(25);
                    notes.push(format!("Criteria contains subjective term: '{}'", word));
                }
            }
        }

        if score < 70 {
            return Err(anyhow::anyhow!(
                "Quality Gate Failed: Requirement is too ambiguous (Score {}/100). Issues: {:?}",
                score,
                notes
            ));
        }

        Ok(true)
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
