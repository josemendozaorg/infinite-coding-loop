use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

/// Structured log event types for full execution traceability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LogEventType {
    IterationStart,
    IterationResumed,
    IterationEnd,
    LoopCycle,
    ActionIdentified,
    ActionDispatched,
    ActionSkipped,
    PromptSent,
    ResponseReceived,
    ArtifactPersisted,
    ValidationResult,
    VerificationResult,
    RefinementAttempt,
    Error,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// A single structured log entry, serialized as one JSON line in the JSONL file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub timestamp: String,
    pub event_type: LogEventType,
    pub level: LogLevel,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl LogEvent {
    pub fn new(
        event_type: LogEventType,
        level: LogLevel,
        message: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            event_type,
            level,
            message: message.into(),
            details,
        }
    }

    pub fn info(event_type: LogEventType, message: impl Into<String>) -> Self {
        Self::new(event_type, LogLevel::Info, message, None)
    }

    pub fn info_with_details(
        event_type: LogEventType,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self::new(event_type, LogLevel::Info, message, Some(details))
    }

    pub fn debug_with_details(
        event_type: LogEventType,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self::new(event_type, LogLevel::Debug, message, Some(details))
    }

    pub fn warn(event_type: LogEventType, message: impl Into<String>) -> Self {
        Self::new(event_type, LogLevel::Warn, message, None)
    }

    pub fn warn_with_details(
        event_type: LogEventType,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self::new(event_type, LogLevel::Warn, message, Some(details))
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(LogEventType::Error, LogLevel::Error, message, None)
    }

    pub fn error_with_details(message: impl Into<String>, details: serde_json::Value) -> Self {
        Self::new(LogEventType::Error, LogLevel::Error, message, Some(details))
    }
}

/// Manages writing structured JSONL log files for an iteration.
///
/// Inspired by OpenCode's session-based JSONL logging. Each iteration gets its own
/// `logs/execution.jsonl` file containing one JSON event per line.
pub struct IterationLogger {
    log_file_path: PathBuf,
}

impl IterationLogger {
    /// Creates a new logger for the given iteration directory.
    /// Creates the `logs/` subdirectory if it doesn't exist.
    pub async fn new(iteration_dir: &Path) -> Result<Self> {
        let logs_dir = iteration_dir.join("logs");
        tokio::fs::create_dir_all(&logs_dir)
            .await
            .context("Failed to create logs directory")?;

        let log_file_path = logs_dir.join("execution.jsonl");

        Ok(Self { log_file_path })
    }

    /// Appends a log event as a JSON line to the execution log file.
    /// Each call opens/appends/flushes for crash safety.
    pub async fn log(&self, event: LogEvent) -> Result<()> {
        let mut line = serde_json::to_string(&event).context("Failed to serialize log event")?;
        line.push('\n');

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file_path)
            .await
            .context("Failed to open log file")?;

        file.write_all(line.as_bytes()).await?;
        file.flush().await?;

        Ok(())
    }

    /// Convenience: log an iteration start event.
    pub async fn log_iteration_start(&self, iteration_id: &str, name: &str) -> Result<()> {
        self.log(LogEvent::info_with_details(
            LogEventType::IterationStart,
            format!("Started iteration: {} ({})", name, iteration_id),
            serde_json::json!({
                "iteration_id": iteration_id,
                "name": name,
            }),
        ))
        .await
    }

    /// Convenience: log an iteration resumed event.
    pub async fn log_iteration_resumed(&self, iteration_id: &str) -> Result<()> {
        self.log(LogEvent::info_with_details(
            LogEventType::IterationResumed,
            format!("Resumed iteration: {}", iteration_id),
            serde_json::json!({ "iteration_id": iteration_id }),
        ))
        .await
    }

    /// Convenience: log a loop cycle start.
    pub async fn log_loop_cycle(&self, cycle_number: usize) -> Result<()> {
        self.log(LogEvent::info_with_details(
            LogEventType::LoopCycle,
            format!("Loop cycle {}", cycle_number),
            serde_json::json!({ "cycle": cycle_number }),
        ))
        .await
    }

    /// Convenience: log an action identified by the planner.
    pub async fn log_action_identified(
        &self,
        agent: &str,
        relation: &str,
        target: &str,
        category: &str,
    ) -> Result<()> {
        self.log(LogEvent::info_with_details(
            LogEventType::ActionIdentified,
            format!("{} {} {}", agent, relation, target),
            serde_json::json!({
                "agent": agent,
                "relation": relation,
                "target": target,
                "category": category,
            }),
        ))
        .await
    }

    /// Convenience: log an action being dispatched to an agent.
    pub async fn log_action_dispatched(
        &self,
        agent: &str,
        relation: &str,
        target: &str,
        category: &str,
    ) -> Result<()> {
        self.log(LogEvent::info_with_details(
            LogEventType::ActionDispatched,
            format!("Dispatching: {} {} {}", agent, relation, target),
            serde_json::json!({
                "agent": agent,
                "relation": relation,
                "target": target,
                "category": category,
            }),
        ))
        .await
    }

    /// Convenience: log an action skipped by user.
    pub async fn log_action_skipped(
        &self,
        agent: &str,
        relation: &str,
        target: &str,
    ) -> Result<()> {
        self.log(LogEvent::info_with_details(
            LogEventType::ActionSkipped,
            format!("Skipped by user: {} {} {}", agent, relation, target),
            serde_json::json!({
                "agent": agent,
                "relation": relation,
                "target": target,
            }),
        ))
        .await
    }

    /// Convenience: log the full prompt sent to an agent.
    pub async fn log_prompt_sent(&self, agent: &str, target: &str, prompt: &str) -> Result<()> {
        self.log(LogEvent::debug_with_details(
            LogEventType::PromptSent,
            format!("Prompt sent to {} for {}", agent, target),
            serde_json::json!({
                "agent": agent,
                "target": target,
                "prompt": prompt,
                "prompt_length": prompt.len(),
            }),
        ))
        .await
    }

    /// Convenience: log the raw response from an agent.
    pub async fn log_response_received(
        &self,
        agent: &str,
        target: &str,
        response: &serde_json::Value,
    ) -> Result<()> {
        let response_str = serde_json::to_string(response).unwrap_or_default();
        self.log(LogEvent::debug_with_details(
            LogEventType::ResponseReceived,
            format!("Response from {} for {}", agent, target),
            serde_json::json!({
                "agent": agent,
                "target": target,
                "response": response,
                "response_length": response_str.len(),
            }),
        ))
        .await
    }

    /// Convenience: log artifact validation result.
    pub async fn log_validation(
        &self,
        target: &str,
        passed: bool,
        error: Option<&str>,
    ) -> Result<()> {
        let (level, msg) = if passed {
            (LogLevel::Info, format!("Validation passed for {}", target))
        } else {
            (LogLevel::Warn, format!("Validation failed for {}", target))
        };
        self.log(LogEvent::new(
            LogEventType::ValidationResult,
            level,
            msg,
            Some(serde_json::json!({
                "target": target,
                "passed": passed,
                "error": error,
            })),
        ))
        .await
    }

    /// Convenience: log verification result with score.
    pub async fn log_verification(
        &self,
        target: &str,
        score: f64,
        threshold: f64,
        feedback: &str,
    ) -> Result<()> {
        let passed = score >= threshold;
        let level = if passed {
            LogLevel::Info
        } else {
            LogLevel::Warn
        };
        self.log(LogEvent::new(
            LogEventType::VerificationResult,
            level,
            format!(
                "Verification {} for {} (score: {:.2}, threshold: {:.2})",
                if passed { "passed" } else { "failed" },
                target,
                score,
                threshold,
            ),
            Some(serde_json::json!({
                "target": target,
                "score": score,
                "threshold": threshold,
                "passed": passed,
                "feedback": feedback,
            })),
        ))
        .await
    }

    /// Convenience: log artifact persisted.
    pub async fn log_artifact_persisted(&self, name: &str, path: &str) -> Result<()> {
        self.log(LogEvent::info_with_details(
            LogEventType::ArtifactPersisted,
            format!("Persisted {} to {}", name, path),
            serde_json::json!({
                "name": name,
                "path": path,
            }),
        ))
        .await
    }

    /// Convenience: log a refinement attempt.
    pub async fn log_refinement_attempt(
        &self,
        target: &str,
        attempt: usize,
        max_retries: usize,
    ) -> Result<()> {
        self.log(LogEvent::info_with_details(
            LogEventType::RefinementAttempt,
            format!(
                "Refinement attempt {}/{} for {}",
                attempt, max_retries, target
            ),
            serde_json::json!({
                "target": target,
                "attempt": attempt,
                "max_retries": max_retries,
            }),
        ))
        .await
    }

    /// Convenience: log an error.
    pub async fn log_error(&self, message: &str, details: Option<&str>) -> Result<()> {
        self.log(LogEvent::new(
            LogEventType::Error,
            LogLevel::Error,
            message,
            details.map(|d| serde_json::json!({ "error": d })),
        ))
        .await
    }

    /// Returns the path to the log file.
    pub fn log_file_path(&self) -> &Path {
        &self.log_file_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_logger_creates_log_file() {
        let tmp = tempdir().unwrap();
        let iter_dir = tmp.path().join("20260214_0001");
        tokio::fs::create_dir_all(&iter_dir).await.unwrap();

        let logger = IterationLogger::new(&iter_dir).await.unwrap();

        logger
            .log_iteration_start("20260214_0001", "Test Iteration")
            .await
            .unwrap();
        logger
            .log_action_dispatched("Architect", "creates", "DesignSpec", "Creation")
            .await
            .unwrap();
        logger
            .log_prompt_sent("Architect", "DesignSpec", "Design the spec please")
            .await
            .unwrap();
        logger
            .log_response_received(
                "Architect",
                "DesignSpec",
                &serde_json::json!({"result": "ok"}),
            )
            .await
            .unwrap();
        logger
            .log_validation("DesignSpec", true, None)
            .await
            .unwrap();
        logger
            .log_artifact_persisted("DesignSpec", "spec/designspec.json")
            .await
            .unwrap();

        // Verify the log file exists and has correct content
        let log_path = iter_dir.join("logs/execution.jsonl");
        assert!(log_path.exists());

        let content = tokio::fs::read_to_string(&log_path).await.unwrap();
        let lines: Vec<&str> = content.trim().split('\n').collect();
        assert_eq!(lines.len(), 6);

        // Verify each line is valid JSON and deserializes to LogEvent
        for line in &lines {
            let event: LogEvent = serde_json::from_str(line).unwrap();
            assert!(!event.timestamp.is_empty());
            assert!(!event.message.is_empty());
        }

        // Verify first event is IterationStart
        let first: LogEvent = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first.event_type, LogEventType::IterationStart);
        assert_eq!(first.level, LogLevel::Info);

        // Verify prompt event is Debug level
        let prompt: LogEvent = serde_json::from_str(lines[2]).unwrap();
        assert_eq!(prompt.event_type, LogEventType::PromptSent);
        assert_eq!(prompt.level, LogLevel::Debug);
        let details = prompt.details.unwrap();
        assert_eq!(details["prompt"], "Design the spec please");
        assert_eq!(details["prompt_length"], 22);
    }

    #[tokio::test]
    async fn test_logger_verification_events() {
        let tmp = tempdir().unwrap();
        let iter_dir = tmp.path().join("20260214_0002");
        tokio::fs::create_dir_all(&iter_dir).await.unwrap();

        let logger = IterationLogger::new(&iter_dir).await.unwrap();

        logger
            .log_verification("Code", 0.5, 0.8, "Tests failing")
            .await
            .unwrap();
        logger
            .log_verification("Code", 0.9, 0.8, "All tests pass")
            .await
            .unwrap();
        logger.log_refinement_attempt("Code", 1, 3).await.unwrap();

        let content = tokio::fs::read_to_string(logger.log_file_path())
            .await
            .unwrap();
        let lines: Vec<&str> = content.trim().split('\n').collect();
        assert_eq!(lines.len(), 3);

        let fail_event: LogEvent = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(fail_event.event_type, LogEventType::VerificationResult);
        assert_eq!(fail_event.level, LogLevel::Warn);
        assert!(fail_event.details.as_ref().unwrap()["passed"] == false);

        let pass_event: LogEvent = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(pass_event.level, LogLevel::Info);
        assert!(pass_event.details.as_ref().unwrap()["passed"] == true);
    }

    #[tokio::test]
    async fn test_logger_error_events() {
        let tmp = tempdir().unwrap();
        let iter_dir = tmp.path().join("20260214_0003");
        tokio::fs::create_dir_all(&iter_dir).await.unwrap();

        let logger = IterationLogger::new(&iter_dir).await.unwrap();

        logger
            .log_error("Something broke", Some("stack trace here"))
            .await
            .unwrap();

        let content = tokio::fs::read_to_string(logger.log_file_path())
            .await
            .unwrap();
        let event: LogEvent = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(event.event_type, LogEventType::Error);
        assert_eq!(event.level, LogLevel::Error);
        assert_eq!(event.message, "Something broke");
    }

    #[tokio::test]
    async fn test_logger_append_mode() {
        let tmp = tempdir().unwrap();
        let iter_dir = tmp.path().join("20260214_0004");
        tokio::fs::create_dir_all(&iter_dir).await.unwrap();

        // Create logger and write one event
        let logger = IterationLogger::new(&iter_dir).await.unwrap();
        logger
            .log(LogEvent::info(LogEventType::Info, "First"))
            .await
            .unwrap();

        // Create a NEW logger for the same directory (simulates resume)
        let logger2 = IterationLogger::new(&iter_dir).await.unwrap();
        logger2
            .log(LogEvent::info(LogEventType::Info, "Second"))
            .await
            .unwrap();

        let content = tokio::fs::read_to_string(logger.log_file_path())
            .await
            .unwrap();
        let lines: Vec<&str> = content.trim().split('\n').collect();
        assert_eq!(lines.len(), 2, "Should append, not overwrite");
    }
}
