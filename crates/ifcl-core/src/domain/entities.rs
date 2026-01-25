use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Event {
    pub id: Uuid,
    pub session_id: Uuid,
    pub trace_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub worker_id: String,
    pub event_type: String,
    pub payload: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct LoopConfig {
    pub goal: String,
    pub max_coins: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum WorkerRole {
    Git,
    Researcher,
    Coder,
    Planner,
    Critic,
    Architect,
    Ops,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct WorkerProfile {
    pub name: String,
    pub role: WorkerRole,
    pub model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct WorkerOutputPayload {
    pub content: String,
    pub is_stderr: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum TaskStatus {
    Pending,
    Running,
    Success,
    Failure,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Task {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: TaskStatus,
    pub assigned_worker: Option<String>,
    #[serde(default)]
    pub retry_count: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy, Default)]
pub enum LoopStatus {
    #[default]
    Running,
    Paused,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Mission {
    pub id: Uuid,
    pub session_id: Uuid,
    pub name: String,
    pub tasks: Vec<Task>,
    pub workspace_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Bank {
    pub xp: u64,
    pub coins: u64,
}

impl Bank {
    pub fn deposit(&mut self, xp: u64, coins: u64) {
        self.xp += xp;
        self.coins += coins;
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThoughtPayload {
    pub confidence: f32,
    pub reasoning: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogPayload {
    pub level: String,
    pub message: String,
}
