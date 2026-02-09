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
    async fn prompt(&self, prompt_text: &str) -> Result<String>;
}

/// A real implementation that calls a CLI command (default: gemini).
#[derive(Clone)]
pub struct ShellCliClient {
    pub executable: String,
    pub work_dir: String,
    pub yolo: bool,
    pub model: Option<String>,
    pub debug: bool,
    pub output_format: Option<String>,
}

impl ShellCliClient {
    pub fn new(executable: &str, work_dir: String) -> Self {
        Self {
            executable: executable.to_string(),
            work_dir,
            yolo: false,
            model: None,
            debug: false,
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
        self.debug = debug;
        self
    }

    pub fn with_output_format(mut self, output_format: String) -> Self {
        self.output_format = Some(output_format);
        self
    }
}

#[async_trait]
impl AiCliClient for ShellCliClient {
    async fn prompt(&self, prompt_text: &str) -> Result<String> {
        let work_dir = self.work_dir.clone();
        let executable = self.executable.clone();
        let prompt_text_owned = prompt_text.to_string();
        let yolo = self.yolo;
        let model = self.model.clone();

        let mut cmd = Command::new(&executable);
        cmd.current_dir(&work_dir);
        if let Some(ref m) = model {
            cmd.arg("-m").arg(m);
        }
        if self.debug {
            cmd.arg("--debug");
        }
        if let Some(ref f) = self.output_format {
            cmd.arg("--output-format").arg(f);
        }
        cmd.arg("--approval-mode").arg("yolo");
        cmd.arg(&prompt_text_owned);

        if self.debug {
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
        let show_output = self.debug || self.output_format.as_deref() == Some("text");

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
            eprintln!(
                "{}: {}",
                console::style("AI CLI FAILED").bold().red(),
                status
            );
            return Err(anyhow::anyhow!("AI CLI failed with status: {}", status));
        }

        eprintln!("{}", console::style("AI CLI SUCCESS").bold().green());
        Ok(full_stdout)
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
