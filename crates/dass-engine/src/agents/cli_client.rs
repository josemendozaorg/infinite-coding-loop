use anyhow::Result;
use std::process::Command;

/// Abstract interface for an AI CLI Client (e.g., gemini, claude).
use async_trait::async_trait;

/// Abstract interface for an AI CLI Client (e.g., gemini, claude).
#[async_trait]
pub trait AiCliClient: Send + Sync {
    /// Sends a prompt to the AI and returns the response.
    async fn prompt(&self, prompt_text: &str) -> Result<String>;
}

/// A real implementation that calls a CLI command (default: gemini).
#[derive(Clone)]
pub struct ShellCliClient {
    pub executable: String,
    pub work_dir: String,
    pub yolo: bool,
    pub model: Option<String>,
}

impl ShellCliClient {
    pub fn new(executable: &str, work_dir: String) -> Self {
        Self {
            executable: executable.to_string(),
            work_dir,
            yolo: false,
            model: None,
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
}

#[async_trait]
impl AiCliClient for ShellCliClient {
    async fn prompt(&self, prompt_text: &str) -> Result<String> {
        // We execute via sh -c to ensure we can cd to the workdir before running the AI CLI.
        // Command: sh -c 'cd "$1" && shift && exec "$@"' -- <work_dir> <executable> -p <prompt> ...
        let work_dir = self.work_dir.clone();
        let executable = self.executable.clone();
        let prompt_text_owned = prompt_text.to_string();
        let yolo = self.yolo;
        let model = self.model.clone();

        // We always use approval-mode yolo for child processes to prevent hanging on interactive prompts.
        let mut cmd = Command::new(&executable);
        cmd.current_dir(&work_dir);
        if let Some(ref m) = model {
            cmd.arg("-m").arg(m);
        }
        cmd.arg("--approval-mode").arg("yolo");
        cmd.arg(&prompt_text_owned);

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

        // Run the blocking Command in a blocking task to avoid panicking the runtime
        let output = tokio::task::spawn_blocking(move || cmd.output()).await??;

        if !output.status.success() {
            eprintln!(
                "{}: {}",
                console::style("AI CLI FAILED").bold().red(),
                output.status
            );
            return Err(anyhow::anyhow!(
                "AI CLI failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        eprintln!("{}", console::style("AI CLI SUCCESS").bold().green());
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        eprintln!("[DEBUG] Model Output: {}", stdout); // Log for user visibility
        Ok(stdout)
    }
}

#[cfg(test)]
pub mod mocks {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    /// A Mock Client for testing/simulation.
    #[derive(Clone)]
    pub struct MockCliClient {
        pub responses: Arc<Mutex<VecDeque<String>>>,
    }

    impl MockCliClient {
        pub fn new(responses: Vec<String>) -> Self {
            Self {
                responses: Arc::new(Mutex::new(responses.into())),
            }
        }
    }

    #[async_trait]
    impl AiCliClient for MockCliClient {
        async fn prompt(&self, _prompt: &str) -> Result<String> {
            let mut guard = self.responses.lock().unwrap();
            if let Some(res) = guard.pop_front() {
                Ok(res)
            } else {
                Ok("MOCK_RESPONSE".to_string())
            }
        }
    }
}
