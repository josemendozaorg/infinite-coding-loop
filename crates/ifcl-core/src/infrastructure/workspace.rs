use std::fs;
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub enum WorkspaceStatus {
    Valid,
    NotEmpty,
    Invalid(String),
}

pub fn validate_workspace_path(path_str: &str) -> WorkspaceStatus {
    let path = Path::new(path_str);

    if !path.exists() {
        // Try to create it to see if it's writable
        if let Err(e) = fs::create_dir_all(path) {
            return WorkspaceStatus::Invalid(format!("Cannot create directory: {}", e));
        }
    }

    if !path.is_dir() {
        return WorkspaceStatus::Invalid("Path is not a directory".to_string());
    }

    // Check if writable
    let test_file = path.join(".ifcl_write_test");
    if fs::write(&test_file, "test").is_err() {
        return WorkspaceStatus::Invalid("Directory is not writable".to_string());
    }
    let _ = fs::remove_file(test_file);

    // Check if empty
    match fs::read_dir(path) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                WorkspaceStatus::NotEmpty
            } else {
                WorkspaceStatus::Valid
            }
        }
        Err(e) => WorkspaceStatus::Invalid(format!("Cannot read directory: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_validate_new_workspace() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("new_proj");
        let status = validate_workspace_path(sub.to_str().unwrap());
        assert_eq!(status, WorkspaceStatus::Valid);
        assert!(sub.exists());
    }

    #[test]
    fn test_validate_not_empty_workspace() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("existing.txt"), "hello").unwrap();
        let status = validate_workspace_path(dir.path().to_str().unwrap());
        assert_eq!(status, WorkspaceStatus::NotEmpty);
    }

    #[test]
    fn test_validate_invalid_path() {
        // Using a path that likely doesn't have permission
        let status = validate_workspace_path("/root/no_access_pls");
        match status {
            WorkspaceStatus::Invalid(_) => (),
            _ => panic!("Expected invalid status for /root"),
        }
    }
}
