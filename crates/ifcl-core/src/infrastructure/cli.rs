#[derive(Debug)]
pub struct CliResult {
    pub stdout: String,
    pub stderr: String,
    pub status: std::process::ExitStatus,
}

pub struct CliExecutor {
    pub binary: String,
    pub additional_flags: Vec<String>,
}

impl CliExecutor {
    pub fn new(binary: String, additional_flags: Vec<String>) -> Self {
        Self { binary, additional_flags }
    }

    pub async fn execute(&self, prompt: &str) -> anyhow::Result<CliResult> {
        let mut args = Vec::new();
        args.extend(self.additional_flags.clone());
        args.push(prompt.to_string());

        let child = tokio::process::Command::new(&self.binary)
            .args(&args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let timeout = tokio::time::Duration::from_secs(60);

        match tokio::time::timeout(timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => Ok(CliResult {
                stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
                status: output.status,
            }),
            Ok(Err(e)) => anyhow::bail!("Execution failed: {}", e),
            Err(_) => anyhow::bail!("CLI execution timed out after 60s"),
        }
    }
}
