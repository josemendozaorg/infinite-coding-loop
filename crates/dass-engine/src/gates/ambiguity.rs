use crate::product::requirement::Requirement;
use anyhow::Result;

#[derive(Debug, Clone, Default)]
pub struct AmbiguityScore {
    pub score: u8, // 0-100, where 100 is perfectly clear
    pub notes: Vec<String>,
}

#[derive(Debug, Default)]
pub struct AmbiguityChecker;

impl AmbiguityChecker {
    pub const MIN_ACCEPTABLE_SCORE: u8 = 70;

    /// Checks a requirement for ambiguity based on heuristics.
    pub fn check(requirement: &Requirement) -> Result<AmbiguityScore> {
        let mut score = 100u8;
        let mut notes = Vec::new();

        // Heuristic 1: Length check (Too short is suspicious)
        if requirement.user_story.len() < 10 {
            score = score.saturating_sub(50);
            notes.push("User story is suspiciously short.".to_string());
        }

        // Heuristic 2: Acceptance Criteria presence
        if requirement.acceptance_criteria.is_empty() {
            score = 0; // Immediate failure
            notes.push("No acceptance criteria provided.".to_string());
        }

        // Heuristic 3: Ambiguous keywords
        let ambiguous_words = ["fast", "user friendly", "modern", "better", "clean"];
        for word in ambiguous_words {
            if requirement.user_story.to_lowercase().contains(word) {
                score = score.saturating_sub(20);
                notes.push(format!("Contains subjective term: '{}'", word));
            }
            for criteria in &requirement.acceptance_criteria {
                if criteria.to_lowercase().contains(word) {
                    score = score.saturating_sub(25);
                     notes.push(format!("Criteria contains subjective term: '{}'", word));
                }
            }
        }

        Ok(AmbiguityScore { score, notes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_requirement() {
        let req = Requirement::new(
            "As a user I want to login with email",
            vec!["Returns 200 OK on success".to_string()]
        );
        let result = AmbiguityChecker::check(&req).unwrap();
        assert_eq!(result.score, 100);
    }

    #[test]
    fn test_vague_requirement() {
        let req = Requirement::new(
            "Make it user friendly",
            vec!["It should be fast".to_string()]
        );
        let result = AmbiguityChecker::check(&req).unwrap();
        assert!(result.score < 70);
        assert!(result.notes.iter().any(|n| n.contains("user friendly")));
        assert!(result.notes.iter().any(|n| n.contains("fast")));
    }
}
