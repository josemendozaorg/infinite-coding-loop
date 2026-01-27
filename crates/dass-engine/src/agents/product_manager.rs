use anyhow::Result;
use crate::product::requirement::Requirement;
use crate::gates::ambiguity::AmbiguityChecker;
use crate::agents::cli_client::AiCliClient;

pub struct ProductManager<C: AiCliClient> {
    client: C,
}

impl<C: AiCliClient> ProductManager<C> {
    pub fn new(client: C) -> Self {
        Self { client }
    }

    /// The SOP: Generate Requirements -> Check Ambiguity -> Refine if needed.
    pub fn process_request(&self, user_input: &str) -> Result<Vec<Requirement>> {
        let mut attempts = 0;
        let max_attempts = 5;
        let mut current_context = format!(
            "Extract atomic requirements from this user request: '{}'. \
            Output purely YAML format list of Requirement structs. \
            Each struct MUST have exact fields: \
            - id: string \
            - user_story: string \
            - acceptance_criteria: list of strings", 
            user_input
        );

        while attempts < max_attempts {
            attempts += 1;
            let response = self.client.prompt(&current_context)?;
            
            // 1. Try to parse (with markdown stripping support)
            let cleaned_response = if let Some(start) = response.find("```yaml") {
                let after_tag = &response[start+7..];
                if let Some(end) = after_tag.find("```") {
                     after_tag[..end].trim().to_string()
                } else {
                    response.clone()
                }
            } else if let Some(start) = response.find("```") {
                 // Fallback for unspecified code block
                 let after_tag = &response[start+3..];
                 if let Some(end) = after_tag.find("```") {
                     after_tag[..end].trim().to_string()
                } else {
                    response.clone()
                }
            } else {
                response.clone()
            };

            let reqs = match Requirement::load_many_from_yaml(&cleaned_response) {
                Ok(r) => r,
                Err(e) => {
                    // Feedback loop for syntax error
                    current_context = format!(
                        "Your previous output was invalid YAML: {}. \
                        Please fix properly. Ensure you return a YAML list. Input was: '{}'", 
                        e, user_input
                    );
                    continue;
                }
            };

            // 2. Gate Check: Ambiguity
            let mut all_pass = true;
            let mut feedback = String::new();

            for req in &reqs {
                let check = AmbiguityChecker::check(req)?;
                if check.score < AmbiguityChecker::MIN_ACCEPTABLE_SCORE {
                    all_pass = false;
                    feedback.push_str(&format!(
                        "Requirement '{}' is ambiguous (Score {}/100). Issues: {:?}\n", 
                        req.user_story, check.score, check.notes
                    ));
                }
            }

            if all_pass {
                return Ok(reqs);
            } else {
                // Feedback loop for semantic error
                current_context = format!(
                    "The requirements were rejected by the Quality Gate. \
                    Feedback:\n{}\n \
                    Please REWRITE them to be more atomic and verifiable. \
                    Original request: '{}'",
                    feedback, user_input
                );
            }
        }

        Err(anyhow::anyhow!("ProductManager failed to generate valid requirements after {} attempts", max_attempts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::cli_client::MockCliClient;

    #[test]
    fn test_pm_refinement_loop() {
        // Simulation:
        // 1. First response is ambiguous (contains "fast").
        // 2. Second response is good.
        let bad_yaml = "
- id: 00000000-0000-0000-0000-000000000001
  user_story: 'Make it fast'
  acceptance_criteria: []
";
        let good_yaml = "
- id: 00000000-0000-0000-0000-000000000002
  user_story: 'As a user I want to see a spinner'
  acceptance_criteria: ['Spinner visible within 100ms']
";

        let mock = MockCliClient::new(vec![bad_yaml.to_string(), good_yaml.to_string()]);
        let pm = ProductManager::new(mock);

        let result = pm.process_request("I want a spinner").unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].user_story.contains("spinner"));
    }
}
