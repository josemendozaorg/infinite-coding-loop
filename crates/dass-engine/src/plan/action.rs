use crate::clover::Verifiable;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Represents an Atomic Action in an Implementation Plan (The Instruction Set).
/// defined in the "Plan" phase of the DASS process.
///
/// Each action must be reversible and atomic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Action {
    /// Create a new file with content.
    CreateFile {
        path: String,
        #[serde(alias = "payload")]
        content: String,
    },
    /// Modify an existing file (replace content).
    /// For more granular edits, we might need a Diff/Patch struct later.
    ModifyFile {
        path: String,
        #[serde(alias = "payload")]
        new_content: String,
    },
    /// Run a shell command.
    RunCommand {
        command: String,
        cwd: Option<String>,
        /// If true, failure aborts the plan.
        /// If false, failure might be recoverable or ignored (e.g., `mkdir -p` where exists).
        must_succeed: bool,
    },
    /// Run a verification step (Test).
    Verify { test_command: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationPlan {
    pub feature_id: String,
    /// Directed steps. For now, strictly sequential.
    pub steps: Vec<Action>,
    #[serde(default)]
    pub completed_steps: usize,
}

impl Verifiable for ImplementationPlan {
    fn verify(&self) -> Result<bool> {
        for action in &self.steps {
            match action {
                Action::RunCommand { command, .. } => {
                    if command.contains("rm ") || command.contains("del ") {
                        return Err(anyhow::anyhow!(
                            "Safety Gate Failed: Destructive command detected: '{}'",
                            command
                        ));
                    }
                    if command.contains("sudo ") {
                        return Err(anyhow::anyhow!(
                            "Safety Gate Failed: Sudo command forbidden: '{}'",
                            command
                        ));
                    }
                }
                Action::ModifyFile { path, .. } => {
                    if path.starts_with("/etc") || path.starts_with("/var") {
                        return Err(anyhow::anyhow!(
                            "Safety Gate Failed: Modification of system path forbidden: '{}'",
                            path
                        ));
                    }
                }
                _ => {}
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clover_safety() {
        let plan = ImplementationPlan {
            feature_id: "test".to_string(),
            steps: vec![Action::RunCommand {
                command: "rm -rf /".to_string(),
                cwd: None,
                must_succeed: true,
            }],
            completed_steps: 0,
        };
        assert!(plan.verify().is_err());
    }
}
