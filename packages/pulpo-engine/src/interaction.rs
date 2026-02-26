use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait UserInteraction {
    /// Ask the user for open-ended input.
    async fn ask_user(&self, prompt: &str) -> Result<String>;

    /// Ask the user specifically for a feature request (could be pre-seeded from args).
    async fn ask_for_feature(&self, prompt: &str) -> Result<String>;

    /// Ask for a boolean confirmation.
    async fn confirm(&self, prompt: &str) -> Result<bool>;

    /// Ask the user to select an option from a list.
    async fn select_option(&self, prompt: &str, options: &[String]) -> Result<usize>;

    /// Indicate that a long-running step is starting (e.g., show spinner).
    fn start_step(&self, name: &str);

    /// Indicate that a step has completed.
    fn end_step(&self, name: &str);

    /// Display a produced or retrieved artifact generically.
    fn render_artifact(&self, kind: &str, data: &Value);

    /// Log general information.
    fn log_info(&self, msg: &str);

    /// Log error information.
    fn log_error(&self, msg: &str);
}

// Exposed for testing
pub mod mocks {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    #[derive(Default, Clone)]
    pub struct MockUserInteraction {
        pub feature_responses: Arc<Mutex<VecDeque<String>>>,
        pub confirmations: Arc<Mutex<VecDeque<bool>>>,
    }

    impl MockUserInteraction {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn add_feature_response(&self, response: String) {
            self.feature_responses.lock().unwrap().push_back(response);
        }

        pub fn add_confirmation(&self, response: bool) {
            self.confirmations.lock().unwrap().push_back(response);
        }
    }

    #[async_trait]
    impl UserInteraction for MockUserInteraction {
        async fn ask_user(&self, _prompt: &str) -> Result<String> {
            Ok("MOCK_USER_INPUT".to_string())
        }

        async fn ask_for_feature(&self, _prompt: &str) -> Result<String> {
            Ok(self
                .feature_responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_default())
        }

        async fn confirm(&self, _prompt: &str) -> Result<bool> {
            Ok(self
                .confirmations
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(true))
        }

        async fn select_option(&self, _prompt: &str, _options: &[String]) -> Result<usize> {
            Ok(0)
        }

        fn start_step(&self, _name: &str) {}
        fn end_step(&self, _name: &str) {}
        fn render_artifact(&self, _kind: &str, _data: &Value) {}
        fn log_info(&self, _msg: &str) {}
        fn log_error(&self, _msg: &str) {}
    }
}
