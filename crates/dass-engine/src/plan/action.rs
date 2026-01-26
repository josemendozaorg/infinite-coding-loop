use serde::{Deserialize, Serialize};

/// Represents an Atomic Action in an Implementation Plan (The Instruction Set).
/// defined in the "Plan" phase of the DASS process.
///
/// Each action must be reversible and atomic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum Action {
    /// Create a new file with content.
    CreateFile {
        path: String,
        content: String,
    },
    /// Modify an existing file (replace content).
    /// For more granular edits, we might need a Diff/Patch struct later.
    ModifyFile {
        path: String,
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
    Verify {
        test_command: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationPlan {
    pub feature_id: String,
    /// Directed steps. For now, strictly sequential.
    pub steps: Vec<Action>,
}
