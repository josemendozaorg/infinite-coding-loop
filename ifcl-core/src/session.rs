use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::LoopConfig;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Session {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub config: LoopConfig,
}

impl Session {
    pub fn new(name: String, config: LoopConfig) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            created_at: now,
            last_active: now,
            config,
        }
    }
}
