use crate::{Mission, TaskStatus};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Represents the progress of a mission.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProgressStats {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub progress_percentage: f32,
    pub is_stalled: bool,
    pub last_update_timestamp: DateTime<Utc>,
}

/// Trait for calculating and managing mission progress.
pub trait ProgressManager: Send + Sync {
    /// Calculates the progress stats for a given mission.
    fn calculate_progress(&self, mission: &Mission, last_event_timestamp: DateTime<Utc>) -> ProgressStats;
}

/// A basic implementation of the ProgressManager.
pub struct BasicProgressManager;

impl ProgressManager for BasicProgressManager {
    fn calculate_progress(&self, mission: &Mission, last_event_timestamp: DateTime<Utc>) -> ProgressStats {
        let total_tasks = mission.tasks.len();
        let completed_tasks = mission.tasks.iter().filter(|t| t.status == TaskStatus::Success).count();
        
        let progress_percentage = if total_tasks == 0 {
            100.0
        } else {
            (completed_tasks as f32 / total_tasks as f32) * 100.0
        };

        // Determine if stalled: No progress update in the last 5 minutes
        let now = Utc::now();
        let duration_since_last_update = now.signed_duration_since(last_event_timestamp);
        let is_stalled = duration_since_last_update.num_minutes() >= 5 && progress_percentage < 100.0;

        ProgressStats {
            total_tasks,
            completed_tasks,
            progress_percentage,
            is_stalled,
            last_update_timestamp: last_event_timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Task, TaskStatus};
    use uuid::Uuid;
    use chrono::{Duration, Utc};

    #[test]
    fn test_calculate_progress_empty() {
        let manager = BasicProgressManager;
        let mission = Mission {
            id: Uuid::new_v4(),
            name: "Empty Mission".to_string(),
            tasks: vec![],
        };
        let stats = manager.calculate_progress(&mission, Utc::now());
        assert_eq!(stats.progress_percentage, 100.0);
        assert_eq!(stats.total_tasks, 0);
    }

    #[test]
    fn test_calculate_progress_partial() {
        let manager = BasicProgressManager;
        let mission = Mission {
            id: Uuid::new_v4(),
            name: "Partial Mission".to_string(),
            tasks: vec![
                Task { id: Uuid::new_v4(), name: "T1".to_string(), description: "".to_string(), status: TaskStatus::Success, assigned_worker: None },
                Task { id: Uuid::new_v4(), name: "T2".to_string(), description: "".to_string(), status: TaskStatus::Pending, assigned_worker: None },
            ],
        };
        let stats = manager.calculate_progress(&mission, Utc::now());
        assert_eq!(stats.progress_percentage, 50.0);
        assert_eq!(stats.completed_tasks, 1);
        assert_eq!(stats.total_tasks, 2);
    }

    #[test]
    fn test_stall_detection() {
        let manager = BasicProgressManager;
        let mission = Mission {
            id: Uuid::new_v4(),
            name: "Stalled Mission".to_string(),
            tasks: vec![
                Task { id: Uuid::new_v4(), name: "T1".to_string(), description: "".to_string(), status: TaskStatus::Pending, assigned_worker: None },
            ],
        };
        
        // Mock a timestamp from 10 minutes ago
        let old_timestamp = Utc::now() - Duration::minutes(10);
        let stats = manager.calculate_progress(&mission, old_timestamp);
        
        assert!(stats.is_stalled);
        assert_eq!(stats.progress_percentage, 0.0);
    }

    #[test]
    fn test_not_stalled_if_completed() {
        let manager = BasicProgressManager;
        let mission = Mission {
            id: Uuid::new_v4(),
            name: "Completed Mission".to_string(),
            tasks: vec![
                Task { id: Uuid::new_v4(), name: "T1".to_string(), description: "".to_string(), status: TaskStatus::Success, assigned_worker: None },
            ],
        };
        
        let old_timestamp = Utc::now() - Duration::minutes(10);
        let stats = manager.calculate_progress(&mission, old_timestamp);
        
        assert!(!stats.is_stalled); // 100% complete should not be stalled
    }
}
