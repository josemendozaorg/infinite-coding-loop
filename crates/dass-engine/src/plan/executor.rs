use crate::domain::primitive::Primitive;
use crate::plan::action::{Action, ImplementationPlan};
use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct PlanExecutor;

impl PlanExecutor {
    pub fn execute(plan: &ImplementationPlan, work_dir: &Path) -> Result<(Vec<Primitive>, bool)> {
        let mut primitives = Vec::new();
        let mut all_success = true;

        for (i, step) in plan.steps.iter().enumerate().skip(plan.completed_steps) {
            let action_ref = format!("{}:{}", plan.feature_id, i);
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            match step {
                Action::CreateFile { path, content } => {
                    let target_path = work_dir.join(path);
                    if let Some(parent) = target_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&target_path, content)?;

                    primitives.push(Primitive::Code {
                        path: path.clone(),
                        content: content.clone(),
                    });
                    primitives.push(Primitive::ExecutionStep {
                        action_ref,
                        status: "Success".to_string(),
                        stdout: format!("Created file: {}", path),
                        stderr: String::new(),
                        timestamp,
                    });
                }
                Action::ModifyFile { path, new_content } => {
                    let target_path = work_dir.join(path);
                    if let Some(parent) = target_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&target_path, new_content)?;

                    primitives.push(Primitive::Code {
                        path: path.clone(),
                        content: new_content.clone(),
                    });
                    primitives.push(Primitive::ExecutionStep {
                        action_ref,
                        status: "Success".to_string(),
                        stdout: format!("Modified file: {}", path),
                        stderr: String::new(),
                        timestamp,
                    });
                }
                Action::RunCommand {
                    command,
                    cwd,
                    must_succeed,
                } => {
                    let mut cmd = Command::new("sh");
                    cmd.arg("-c").arg(command);

                    let target_cwd = if let Some(dir) = cwd {
                        work_dir.join(dir)
                    } else {
                        work_dir.to_path_buf()
                    };

                    if !target_cwd.exists() {
                        fs::create_dir_all(&target_cwd)?;
                    }
                    cmd.current_dir(target_cwd);

                    let output = cmd.output()?;
                    let success = output.status.success();
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                    primitives.push(Primitive::ExecutionStep {
                        action_ref,
                        status: if success {
                            "Success".to_string()
                        } else {
                            "Failure".to_string()
                        },
                        stdout,
                        stderr: stderr.clone(),
                        timestamp,
                    });

                    if !success && *must_succeed {
                        all_success = false;
                        break;
                    }
                }
                Action::Verify { test_command } => {
                    let mut cmd = Command::new("sh");
                    cmd.arg("-c").arg(test_command);
                    cmd.current_dir(work_dir);

                    let output = cmd.output()?;
                    let success = output.status.success();
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                    primitives.push(Primitive::Verification {
                        test_command: test_command.clone(),
                        success,
                        output: stdout.clone(),
                    });

                    primitives.push(Primitive::ExecutionStep {
                        action_ref,
                        status: if success {
                            "Success".to_string()
                        } else {
                            "Failure".to_string()
                        },
                        stdout,
                        stderr,
                        timestamp,
                    });

                    if !success {
                        all_success = false;
                        break;
                    }
                }
            }
        }
        Ok((primitives, all_success))
    }
}
