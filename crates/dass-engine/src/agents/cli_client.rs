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
}

impl ShellCliClient {
    pub fn new(executable: &str) -> Self {
        Self {
            executable: executable.to_string(),
        }
    }
}

impl AiCliClient for ShellCliClient {
    fn prompt(&self, prompt_text: &str) -> Result<String> {
        // Example execution: gemini -p "some prompt"
        let output = Command::new(&self.executable)
            .arg("-p")
            .arg(prompt_text)
            .output()?;

        if !output.status.success() {
             return Err(anyhow::anyhow!(
                "AI CLI failed: {}", 
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

/// A Mock Client for testing/simulation.
#[derive(Clone)]
pub struct MockCliClient {
    pub responses: std::rc::Rc<std::cell::RefCell<std::collections::VecDeque<String>>>,
}

impl MockCliClient {
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses: std::rc::Rc::new(std::cell::RefCell::new(responses.into())),
        }
    }
}

impl AiCliClient for MockCliClient {
    fn prompt(&self, _prompt: &str) -> Result<String> {
        if let Some(res) = self.responses.borrow_mut().pop_front() {
            Ok(res)
        } else {
            Ok("MOCK_RESPONSE".to_string())
        }
    }
}
