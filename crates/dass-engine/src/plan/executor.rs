use crate::domain::primitive::Primitive;
use crate::plan::action::{Action, ImplementationPlan};
use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct PlanExecutor;

impl PlanExecutor {
    pub fn execute(plan: &ImplementationPlan, work_dir: &Path) -> Result<Vec<Primitive>> {
        let mut primitives = Vec::new();
        for step in &plan.steps {
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
                    if !output.status.success() && *must_succeed {
                        return Err(anyhow::anyhow!(
                            "Command '{}' failed: {}",
                            command,
                            String::from_utf8_lossy(&output.stderr)
                        ));
                    }
                }
                Action::Verify { test_command } => {
                    let mut cmd = Command::new("sh");
                    cmd.arg("-c").arg(test_command);
                    cmd.current_dir(work_dir);

                    let output = cmd.output()?;
                    primitives.push(Primitive::Verification {
                        test_command: test_command.clone(),
                        success: output.status.success(),
                        output: String::from_utf8_lossy(&output.stdout).to_string(),
                    });
                    if !output.status.success() {
                        return Err(anyhow::anyhow!(
                            "Verification failed: {}",
                            String::from_utf8_lossy(&output.stderr)
                        ));
                    }
                }
            }
        }
        Ok(primitives)
    }
}
