use crate::plan::action::{ImplementationPlan, Action};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct SafetyReport {
    pub is_safe: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Default)]
pub struct SafetyChecker;

impl SafetyChecker {
    pub fn check_plan(plan: &ImplementationPlan) -> Result<SafetyReport> {
        let mut warnings = Vec::new();
        let mut is_safe = true;

        for action in &plan.steps {
            match action {
                Action::RunCommand { command, .. } => {
                    if command.contains("rm ") || command.contains("del ") {
                         warnings.push(format!("Destructive command detected: '{}'", command));
                         is_safe = false;
                    }
                    if command.contains("sudo ") {
                        warnings.push(format!("Sudo command forbidden: '{}'", command));
                        is_safe = false;
                    }
                }
                Action::ModifyFile { path, .. } => {
                    if path.contains("/etc/") || path.starts_with("/") {
                        // Heuristic: absolute paths outside cwd are suspect, but typical absolute paths 
                        // might be valid if they point to the repo. 
                        // For now, let's just warn on highly sensitive paths.
                        if path.starts_with("/etc") || path.starts_with("/var") {
                             warnings.push(format!("Modification of system path forbidden: '{}'", path));
                             is_safe = false;
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(SafetyReport { is_safe, warnings })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_rm() {
        let plan = ImplementationPlan {
            feature_id: "test".to_string(),
            steps: vec![Action::RunCommand { 
                command: "rm -rf /".to_string(), 
                cwd: None, 
                must_succeed: true 
            }]
        };
        let report = SafetyChecker::check_plan(&plan).unwrap();
        assert!(!report.is_safe);
        assert!(report.warnings[0].contains("Destructive command"));
    }
}
