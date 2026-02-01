use anyhow::Result;
use std::process::Command;

/// Abstract interface for an AI CLI Client (e.g., gemini, claude).
pub trait AiCliClient {
    /// Sends a prompt to the AI and returns the response.
    fn prompt(&self, prompt_text: &str) -> Result<String>;
}

/// A real implementation that calls a CLI command (default: gemini).
#[derive(Clone)]
pub struct ShellCliClient {
    pub executable: String,
    pub work_dir: Option<String>,
    pub yolo: bool,
    pub model: Option<String>,
}

impl ShellCliClient {
    pub fn new(executable: &str) -> Self {
        Self {
            executable: executable.to_string(),
            work_dir: None,
            yolo: false,
            model: None,
        }
    }

    pub fn with_work_dir(mut self, work_dir: String) -> Self {
        self.work_dir = Some(work_dir);
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

impl AiCliClient for ShellCliClient {
    fn prompt(&self, prompt_text: &str) -> Result<String> {
        // Example execution: gemini -p "some prompt"
        let mut cmd = Command::new(&self.executable);
        cmd.arg("-p").arg(prompt_text);

        if self.yolo {
            cmd.arg("--yolo");
        }

        if let Some(ref m) = self.model {
            cmd.arg("--model").arg(m);
        }

        if let Some(ref wd) = self.work_dir {
            cmd.current_dir(wd);
        }

        let output = cmd.output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "AI CLI failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

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

    impl AiCliClient for MockCliClient {
        fn prompt(&self, _prompt: &str) -> Result<String> {
            let mut guard = self.responses.lock().unwrap();
            if let Some(res) = guard.pop_front() {
                Ok(res)
            } else {
                Ok("MOCK_RESPONSE".to_string())
            }
        }
    }
}
