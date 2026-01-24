use async_trait::async_trait;
use tokio::process::Command;
use std::process::Stdio;
use tokio::io::{BufReader, AsyncBufReadExt};
use serde::{Serialize, Deserialize};

use tokio::sync::mpsc::Sender;

/// Trait defining an Agent that can execute commands/prompts.
#[async_trait]
pub trait Agent: Send + Sync + std::fmt::Debug {
    /// Unique identifier for the agent (e.g., "gemini", "git")
    fn id(&self) -> &str;

    /// Execute the input (prompt or command args) and return the raw output.
    /// If `stream_tx` is provided, output lines are sent to it (content, is_stderr).
    async fn execute(&self, input: &str, current_dir: &str, stream_tx: Option<Sender<(String, bool)>>) -> anyhow::Result<String>;
}

/// An Agent that wraps an AI CLI tool (e.g., gemini, claude, opencode)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCliAgent {
    pub binary: String,
    pub model_flag: Option<String>,
}

impl AiCliAgent {
    pub fn new(binary: String, model_flag: Option<String>) -> Self {
        Self { binary, model_flag }
    }
}

#[async_trait]
impl Agent for AiCliAgent {
    fn id(&self) -> &str {
        &self.binary
    }

    async fn execute(&self, input: &str, current_dir: &str, stream_tx: Option<Sender<(String, bool)>>) -> anyhow::Result<String> {
        let mut args = vec![input.to_string()];
        if let Some(model) = &self.model_flag {
            args.insert(0, model.clone());
            args.insert(0, "--model".to_string());
        }

        let mut child = Command::new(&self.binary)
            .args(&args)
            .current_dir(current_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();
        // We accumulate errors but typically don't fail just because of stderr unless status fails
        let mut _err_str = String::new(); 

        // We can't use select! easily with two streams if we want to drain both fully.
        // We spawn distinct tasks for stdout and stderr to ensure we don't deadlock or block
        
        let tx_out = stream_tx.clone();
        let tx_err = stream_tx.clone();

        let h_out = tokio::spawn(async move {
            let mut captured = String::new();
            while let Ok(Some(line)) = stdout_reader.next_line().await {
                captured.push_str(&line);
                captured.push('\n');
                if let Some(tx) = &tx_out {
                    let _ = tx.send((line, false)).await;
                }
            }
            captured
        });

        let h_err = tokio::spawn(async move {
            let mut captured = String::new();
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                captured.push_str(&line);
                captured.push('\n');
                if let Some(tx) = &tx_err {
                    let _ = tx.send((line, true)).await;
                }
            }
            captured
        });

        let status = child.wait().await?;
        let out_str = h_out.await?;
        _err_str = h_err.await?;

        if status.success() {
            Ok(out_str)
        } else {
            anyhow::bail!("Agent Execution Failed ({}): {}", status, _err_str)
        }
    }
}

/// A Mock Agent for testing purposes.
#[derive(Debug, Clone)]
pub struct MockAgent {
    pub id: String,
    pub output_sequence: Vec<String>, // Sequence of outputs to output
}

#[async_trait]
impl Agent for MockAgent {
    fn id(&self) -> &str {
        &self.id
    }

    async fn execute(&self, _input: &str, _current_dir: &str, stream_tx: Option<Sender<(String, bool)>>) -> anyhow::Result<String> {
        let response = if let Some(first) = self.output_sequence.first() {
            first.clone()
        } else {
            "Mock Output".to_string()
        };

        // Simulate streaming
        if let Some(tx) = stream_tx {
            for line in response.lines() {
                let _ = tx.send((line.to_string(), false)).await;
            }
        }
        
        Ok(response)
    }
}
