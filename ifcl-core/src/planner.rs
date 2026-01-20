use crate::{Mission, Task, TaskStatus};
use uuid::Uuid;

/// Trait for a planner that generates missions based on a goal.
pub trait Planner: Send + Sync {
    fn generate_initial_missions(&self, goal: &str) -> Vec<Mission>;
}

/// A basic, rule-based planner that generates a few standard missions for any goal.
pub struct BasicPlanner;

impl Planner for BasicPlanner {
    fn generate_initial_missions(&self, goal: &str) -> Vec<Mission> {
        let mut missions = Vec::new();

        // Mission 1: Analysis & Design
        missions.push(Mission {
            id: Uuid::new_v4(),
            name: format!("Phase 1: Analysis - {}", goal),
            tasks: vec![
                Task {
                    id: Uuid::new_v4(),
                    name: "Deconstruct Goal".to_string(),
                    description: format!("Identify core requirements for: {}", goal),
                    status: TaskStatus::Pending,
                    assigned_worker: Some("Architect".to_string()),
                },
                Task {
                    id: Uuid::new_v4(),
                    name: "Sketch Architecture".to_string(),
                    description: "Define module boundaries and data flow.".to_string(),
                    status: TaskStatus::Pending,
                    assigned_worker: Some("Architect".to_string()),
                },
            ],
        });

        // Mission 2: Initial Setup
        missions.push(Mission {
            id: Uuid::new_v4(),
            name: "Phase 2: Setup".to_string(),
            tasks: vec![
                Task {
                    id: Uuid::new_v4(),
                    name: "Initialize Repository".to_string(),
                    description: "Setup git and base file structure.".to_string(),
                    status: TaskStatus::Pending,
                    assigned_worker: Some("Git-Bot".to_string()),
                },
            ],
        });

        missions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planner_generation() {
        let planner = BasicPlanner;
        let missions = planner.generate_initial_missions("Build a Rust CLI");
        
        assert!(missions.len() >= 2);
        assert!(missions[0].name.contains("Build a Rust CLI"));
        assert_eq!(missions[0].tasks.len(), 2);
        assert_eq!(missions[1].tasks.len(), 1);
    }
}
