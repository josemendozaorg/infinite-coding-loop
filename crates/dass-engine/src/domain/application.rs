use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareApplication {
    pub id: String,
    pub name: String,
    pub work_dir: Option<String>,
}

impl SoftwareApplication {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            work_dir: None,
        }
    }
}
