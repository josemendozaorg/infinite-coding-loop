use serde::{Deserialize, Serialize};
use crate::WorkerProfile;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkerGroup {
    pub name: String,
    pub description: String,
    pub workers: Vec<WorkerProfile>,
}
