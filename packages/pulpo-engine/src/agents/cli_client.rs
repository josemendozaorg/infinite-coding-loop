use anyhow::Result;
use std::io::Write;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

/// Abstract interface for an AI CLI Client (e.g., gemini, claude).
use async_trait::async_trait;

/// Abstract interface for an AI CLI Client (e.g., gemini, claude).
#[async_trait]
pub trait AiCliClient: Send + Sync {
    /// Sends a prompt to the AI and returns the response.
    async fn prompt(
        &self,
        prompt_text: &str,
        options: crate::graph::executor::ExecutionOptions,
    ) -> Result<String>;
}

/// A real implementation that calls a CLI command (default: gemini).
#[derive(Clone)]
pub struct ShellCliClient {
    pub executable: String,
    pub work_dir: String,
    pub yolo: bool,
    pub model: Option<String>,
    pub debug_ai_cli: bool,
    pub output_format: Option<String>,
}

impl ShellCliClient {
    pub fn new(executable: &str, work_dir: String) -> Self {
        Self {
            executable: executable.to_string(),
            work_dir,
            yolo: false,
            model: None,
            debug_ai_cli: false,
            output_format: None,
        }
    }

    pub fn with_work_dir(mut self, work_dir: String) -> Self {
        self.work_dir = work_dir;
        self
    }

    pub fn with_yolo(mut self, yolo: bool) -> Self {
        self.yolo = yolo;
        self
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug_ai_cli = debug;
        self
    }

    pub fn with_output_format(mut self, output_format: String) -> Self {
        self.output_format = Some(output_format);
        self
    }
}

#[async_trait]
impl AiCliClient for ShellCliClient {
    async fn prompt(
        &self,
        prompt_text: &str,
        options: crate::graph::executor::ExecutionOptions,
    ) -> Result<String> {
        let max_retries = 3u32;
        let mut attempt = 0u32;

        loop {
            attempt += 1;
            let result = self.execute_prompt(prompt_text, &options).await;

            match result {
                Ok(output) => return Ok(output),
                Err(e) => {
                    let err_msg = format!("{}", e);
                    let is_rate_limit = err_msg.contains("exhausted your capacity")
                        || err_msg.contains("rate limit")
                        || err_msg.contains("quota");

                    if is_rate_limit && attempt < max_retries {
                        let backoff_secs = 2u64.pow(attempt); // 2s, 4s, 8s
                        eprintln!(
                            "{} (attempt {}/{}). Retrying in {}s...",
                            console::style("Model rate limited").bold().yellow(),
                            attempt,
                            max_retries,
                            backoff_secs
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
                        continue;
                    }

                    return Err(e);
                }
            }
        }
    }
}

impl ShellCliClient {
    async fn execute_prompt(
        &self,
        prompt_text: &str,
        options: &crate::graph::executor::ExecutionOptions,
    ) -> Result<String> {
        let work_dir = self.work_dir.clone();

        let executable = options
            .ai_cli
            .clone()
            .unwrap_or_else(|| self.executable.clone());
        let model = options.model.clone().or_else(|| self.model.clone());

        let prompt_text_owned = prompt_text.to_string();
        let yolo = self.yolo;

        let mut cmd = Command::new(&executable);
        cmd.current_dir(&work_dir);
        if let Some(ref m) = model {
            cmd.arg("-m").arg(m);
        }
        if self.debug_ai_cli {
            cmd.arg("--debug");
        }
        if let Some(ref f) = self.output_format {
            cmd.arg("--output-format").arg(f);
        }
        cmd.arg("--approval-mode").arg("yolo");
        cmd.arg(&prompt_text_owned);

        if self.debug_ai_cli {
            eprintln!(
                "\n{}",
                console::style("--- AI CLI PROMPT ---").bold().yellow()
            );
            eprintln!("WorkDir: {}", work_dir);
            eprintln!("Command: {:?}", cmd);
            eprintln!("Engine YOLO (Auto-Confirm): {}", yolo);
            eprintln!("Prompt Length: {} chars", prompt_text.len());
            eprintln!(
                "{}\n",
                console::style("----------------------").bold().yellow()
            );
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();

        let mut full_stdout = String::new();
        let mut full_stderr = String::new();
        let show_output = self.debug_ai_cli || self.output_format.as_deref() == Some("text");

        let mut stdout_done = false;
        let mut stderr_done = false;
        let mut stdout_buf = [0u8; 1024];
        let mut stderr_buf = [0u8; 1024];

        while !stdout_done || !stderr_done {
            tokio::select! {
                res = stdout.read(&mut stdout_buf), if !stdout_done => {
                    match res {
                        Ok(0) => stdout_done = true,
                        Ok(n) => {
                            let chunk = &stdout_buf[..n];
                            full_stdout.push_str(&String::from_utf8_lossy(chunk));
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
                res = stderr.read(&mut stderr_buf), if !stderr_done => {
                    match res {
                        Ok(0) => stderr_done = true,
                        Ok(n) => {
                            let chunk = &stderr_buf[..n];
                            full_stderr.push_str(&String::from_utf8_lossy(chunk));
                            if show_output {
                                std::io::stderr().write_all(chunk).ok();
                                std::io::stderr().flush().ok();
                            }
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
                _ = child.wait(), if stdout_done && stderr_done => {
                    break;
                }
            }
        }

        let status = child.wait().await?;

        if !status.success() {
            // Include stderr in the error so retry logic can detect rate limiting
            let err_msg = format!(
                "AI CLI failed with status: {}. Stderr: {}",
                status,
                full_stderr.chars().take(500).collect::<String>()
            );
            eprintln!(
                "{}: {}",
                console::style("AI CLI FAILED").bold().red(),
                status
            );
            return Err(anyhow::anyhow!(err_msg));
        }

        eprintln!("{}", console::style("AI CLI SUCCESS").bold().green());
        Ok(full_stdout)
    }
}

// Exposed for e2e and integration testing
pub mod mocks {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    type MockResponseAction = Box<dyn Fn(&str) -> Result<String> + Send + Sync>;

    /// A Mock Client for testing/simulation.
    #[derive(Clone)]
    pub struct MockCliClient {
        pub responses: Arc<Mutex<VecDeque<MockResponseAction>>>,
    }

    impl Default for MockCliClient {
        fn default() -> Self {
            Self::new()
        }
    }

    impl MockCliClient {
        pub fn new() -> Self {
            Self {
                responses: Arc::new(Mutex::new(VecDeque::new())),
            }
        }

        pub fn add_response(&self, response: String) {
            let mut guard = self.responses.lock().unwrap();
            guard.push_back(Box::new(move |_| Ok(response.clone())));
        }

        pub fn add_action<F>(&self, action: F)
        where
            F: Fn(&str) -> Result<String> + Send + Sync + 'static,
        {
            let mut guard = self.responses.lock().unwrap();
            guard.push_back(Box::new(action));
        }
    }

    #[async_trait]
    impl AiCliClient for MockCliClient {
        async fn prompt(
            &self,
            prompt: &str,
            _options: crate::graph::executor::ExecutionOptions,
        ) -> Result<String> {
            let action_opt = {
                let mut guard = self.responses.lock().unwrap();
                guard.pop_front()
            };

            if let Some(action) = action_opt {
                action(prompt)
            } else {
                Ok("MOCK_RESPONSE".to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_cli_client_builder() {
        let client = ShellCliClient::new("gemini", "/tmp".to_string())
            .with_yolo(true)
            .with_model("gemini-2.0-flash".to_string())
            .with_debug(true)
            .with_output_format("text".to_string());

        assert_eq!(client.executable, "gemini");
        assert_eq!(client.work_dir, "/tmp");
        assert!(client.yolo);
        assert_eq!(client.model, Some("gemini-2.0-flash".to_string()));
        assert!(client.debug_ai_cli);
        assert_eq!(client.output_format, Some("text".to_string()));
    }

    #[tokio::test]
    async fn test_shell_cli_client_invalid_command() {
        // This test is fast because it immediately fails to spawn the process
        let client = ShellCliClient::new("non_existent_command_12345", "/tmp".to_string());
        let options = crate::graph::executor::ExecutionOptions::default();
        let result = client.prompt("hello", options).await;
        assert!(result.is_err());
    }
}
