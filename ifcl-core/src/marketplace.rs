use crate::{WorkerProfile, Mission, WorkerGroup};
use std::path::Path;
use std::fs;

#[derive(Debug)]
pub struct MarketplaceLoader;

impl MarketplaceLoader {
    pub fn load_workers<P: AsRef<Path>>(path: P) -> Vec<WorkerProfile> {
        let mut profiles = Vec::new();
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(profile) = serde_json::from_str::<WorkerProfile>(&content) {
                        profiles.push(profile);
                    }
                }
            }
        }
        profiles
    }

    pub fn load_missions<P: AsRef<Path>>(path: P) -> Vec<Mission> {
        let mut missions = Vec::new();
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(mission) = serde_json::from_str::<Mission>(&content) {
                        missions.push(mission);
                    }
                }
            }
        }
        missions
    }

    pub fn load_groups<P: AsRef<Path>>(path: P) -> Vec<WorkerGroup> {
        let mut groups = Vec::new();
        if let Ok(entries) = fs::read_dir(path) {
             for entry in entries.flatten() {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(group) = serde_json::from_str::<WorkerGroup>(&content) {
                        groups.push(group);
                    }
                }
            }
        }
        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_marketplace_load_groups() {
        let temp_dir = std::env::temp_dir().join(Uuid::new_v4().to_string());
        std::fs::create_dir_all(&temp_dir).unwrap();

        let group_json = r#"{
            "name": "Web Dev Team",
            "description": "A team for web development",
            "workers": [
                {
                    "name": "Frontend Bot",
                    "role": "Coder",
                    "capabilities": ["React", "CSS"],
                    "xp": 100
                }
            ]
        }"#;

        std::fs::write(temp_dir.join("web_team.json"), group_json).unwrap();

        let groups = MarketplaceLoader::load_groups(&temp_dir);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "Web Dev Team");
        assert_eq!(groups[0].workers.len(), 1);

        std::fs::remove_dir_all(temp_dir).unwrap();
    }
}
