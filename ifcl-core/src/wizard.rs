
use serde::{Deserialize, Serialize};
use crate::LoopConfig;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WizardStep {
    Goal,
    Stack,
    Team,
    Budget,
    Summary,
}

pub struct SetupWizard {
    pub current_step: WizardStep,
    pub goal: String,
    pub stack: String,
    pub team_size: usize,
    pub budget_coins: u64,
}

impl SetupWizard {
    pub fn new() -> Self {
        Self {
            current_step: WizardStep::Goal,
            goal: String::new(),
            stack: "Rust".to_string(), // Default
            team_size: 2,
            budget_coins: 100,
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
            WizardStep::Stack => self.current_step = WizardStep::Team,
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
            WizardStep::Team => self.current_step = WizardStep::Stack,
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
        wizard.next().unwrap();
        wizard.next().unwrap();
        assert_eq!(wizard.current_step, WizardStep::Summary);

        wizard.prev();
        assert_eq!(wizard.current_step, WizardStep::Budget);
    }
}
