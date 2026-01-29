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

    /// Display the analyzed requirements.
    fn render_requirements(&self, reqs: &[Value]);

    /// Display the generated specification.
    fn render_spec(&self, spec: &Value);

    /// Display the generated plan.
    fn render_plan(&self, plan: &Value);

    /// Log general information.
    fn log_info(&self, msg: &str);

    /// Log error information.
    fn log_error(&self, msg: &str);
}
