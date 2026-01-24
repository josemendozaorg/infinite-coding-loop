
use serde::{Deserialize, Serialize};
use crate::LoopConfig;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiProvider {
    Gemini,
    Claude,
    OpenCode,
    Basic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WizardStep {
    Goal,
    Stack,
    Workspace,
    Provider, // New step for AI Provider selection
    Team,
    Budget,
    Summary,
}

pub struct SetupWizard {
    pub current_step: WizardStep,
    pub goal: String,
    pub stack: String,
    pub workspace_path: String,
    pub provider: AiProvider, // New field
    pub team_size: usize,
    pub budget_coins: u64,
    pub selected_group_index: usize,
}

impl SetupWizard {
    pub fn new() -> Self {
        Self {
            current_step: WizardStep::Goal,
            goal: String::new(),
            stack: "Rust".to_string(), 
            workspace_path: ".".to_string(),
            provider: AiProvider::Basic, // Default
            team_size: 2,
            budget_coins: 100,
            selected_group_index: 0,
        }
    }
}

impl Default for SetupWizard {
    fn default() -> Self {
        Self::new()
    }
}

impl SetupWizard {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<(), String> {
        match self.current_step {
            WizardStep::Goal => {
                if self.goal.trim().is_empty() {
                    return Err("Goal cannot be empty".to_string());
                }
                self.current_step = WizardStep::Stack;
            }
            WizardStep::Stack => self.current_step = WizardStep::Workspace,
            WizardStep::Workspace => {
                if self.workspace_path.trim().is_empty() {
                    return Err("Workspace path cannot be empty".to_string());
                }
                self.current_step = WizardStep::Provider;
            }
            WizardStep::Provider => self.current_step = WizardStep::Team,
            WizardStep::Team => self.current_step = WizardStep::Budget,
            WizardStep::Budget => self.current_step = WizardStep::Summary,
            WizardStep::Summary => (),
        }
        Ok(())
    }

    pub fn prev(&mut self) {
        match self.current_step {
            WizardStep::Goal => (),
            WizardStep::Stack => self.current_step = WizardStep::Goal,
            WizardStep::Workspace => self.current_step = WizardStep::Stack,
            WizardStep::Provider => self.current_step = WizardStep::Workspace,
            WizardStep::Team => self.current_step = WizardStep::Provider,
            WizardStep::Budget => self.current_step = WizardStep::Team,
            WizardStep::Summary => self.current_step = WizardStep::Budget,
        }
    }

    pub fn build_config(&self) -> LoopConfig {
        LoopConfig {
            goal: self.goal.clone(),
            max_coins: Some(self.budget_coins),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wizard_navigation() {
        let mut wizard = SetupWizard::new();
        assert_eq!(wizard.current_step, WizardStep::Goal);

        // Fail without goal
        assert!(wizard.next().is_err());

        wizard.goal = "Build a web app".to_string();
        assert!(wizard.next().is_ok());
        assert_eq!(wizard.current_step, WizardStep::Stack);

        wizard.next().unwrap();
        assert_eq!(wizard.current_step, WizardStep::Workspace);
        wizard.workspace_path = "/tmp".to_string();
        wizard.next().unwrap();
        
        // New Provider step
        assert_eq!(wizard.current_step, WizardStep::Provider);
        wizard.next().unwrap();

        assert_eq!(wizard.current_step, WizardStep::Team);
        wizard.next().unwrap(); // To Budget
        wizard.next().unwrap(); // To Summary
        assert_eq!(wizard.current_step, WizardStep::Summary);

        wizard.prev();
        assert_eq!(wizard.current_step, WizardStep::Budget);
        wizard.prev();
        assert_eq!(wizard.current_step, WizardStep::Team);
        wizard.prev();
        assert_eq!(wizard.current_step, WizardStep::Provider); // Back directly to Provider
        wizard.prev();
        assert_eq!(wizard.current_step, WizardStep::Workspace);
    }
}
