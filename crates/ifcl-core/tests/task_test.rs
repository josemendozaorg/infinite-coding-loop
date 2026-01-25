#[cfg(test)]
mod tests {
    use ifcl_core::{Task, TaskStatus};
    use uuid::Uuid;

    #[test]
    fn test_task_retry_count() {
        let task = Task {
            id: Uuid::new_v4(),
            name: "Test Task".to_string(),
            description: "A test task".to_string(),
            status: TaskStatus::Pending,
            assigned_worker: None,
            retry_count: 0, // This should fail to compile if field is missing
        };
        assert_eq!(task.retry_count, 0);
    }
}
