
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Event {
    pub id: Uuid,
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
pub struct WorkerGroup {
    pub name: String,
    pub workers: Vec<WorkerProfile>,
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
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Mission {
    pub id: Uuid,
    pub name: String,
    pub tasks: Vec<Task>,
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

pub trait Worker: Send + Sync {
    fn id(&self) -> &str;
    fn role(&self) -> WorkerRole;
    fn metadata(&self) -> &WorkerProfile;
}

#[async_trait::async_trait]
pub trait Orchestrator: Send + Sync {
    async fn create_mission(&self, name: &str, tasks: Vec<(String, String)>) -> anyhow::Result<Mission>;
    async fn update_task_status(&self, mission_id: Uuid, task_id: Uuid, status: TaskStatus) -> anyhow::Result<()>;
    async fn get_missions(&self) -> anyhow::Result<Vec<Mission>>;
}

#[async_trait::async_trait]
pub trait EventBus: Send + Sync {
    async fn publish(&self, event: Event) -> anyhow::Result<()>;
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Event>;
}

pub struct InMemoryEventBus {
    tx: tokio::sync::broadcast::Sender<Event>,
}

impl InMemoryEventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(capacity);
        Self { tx }
    }
}

#[async_trait::async_trait]
impl EventBus for InMemoryEventBus {
    async fn publish(&self, event: Event) -> anyhow::Result<()> {
        let _ = self.tx.send(event);
        Ok(())
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}

#[async_trait::async_trait]
pub trait EventStore: Send + Sync {
    async fn append(&self, event: Event) -> anyhow::Result<()>;
    async fn list(&self) -> anyhow::Result<Vec<Event>>;
}

pub struct InMemoryEventStore {
    events: tokio::sync::RwLock<Vec<Event>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: tokio::sync::RwLock::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl EventStore for InMemoryEventStore {
    async fn append(&self, event: Event) -> anyhow::Result<()> {
        let mut events = self.events.write().await;
        events.push(event);
        Ok(())
    }

    async fn list(&self) -> anyhow::Result<Vec<Event>> {
        let events = self.events.read().await;
        Ok(events.clone())
    }
}

pub struct BasicOrchestrator {
    missions: tokio::sync::RwLock<Vec<Mission>>,
}

impl BasicOrchestrator {
    pub fn new() -> Self {
        Self {
            missions: tokio::sync::RwLock::new(Vec::new()),
        }
    }
}

pub struct CliExecutor {
    pub binary: String,
}

impl CliExecutor {
    pub fn new(binary: String) -> Self {
        Self { binary }
    }

    pub async fn execute(&self, prompt: &str) -> anyhow::Result<String> {
        let output = tokio::process::Command::new(&self.binary)
            .arg(prompt)
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            anyhow::bail!(
                "CLI Error: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )
        }
    }
}

#[async_trait::async_trait]
impl Orchestrator for BasicOrchestrator {
    async fn create_mission(&self, name: &str, tasks: Vec<(String, String)>) -> anyhow::Result<Mission> {
        let task_list = tasks.into_iter().map(|(n, d)| Task {
            id: Uuid::new_v4(),
            name: n,
            description: d,
            status: TaskStatus::Pending,
            assigned_worker: None,
        }).collect();

        let mission = Mission {
            id: Uuid::new_v4(),
            name: name.to_string(),
            tasks: task_list,
        };

        let mut missions = self.missions.write().await;
        missions.push(mission.clone());
        Ok(mission)
    }

    async fn update_task_status(&self, mission_id: Uuid, task_id: Uuid, status: TaskStatus) -> anyhow::Result<()> {
        let mut missions = self.missions.write().await;
        if let Some(mission) = missions.iter_mut().find(|m| m.id == mission_id) {
            if let Some(task) = mission.tasks.iter_mut().find(|t| t.id == task_id) {
                task.status = status;
                return Ok(());
            }
        }
        anyhow::bail!("Mission or Task not found")
    }

    async fn get_missions(&self) -> anyhow::Result<Vec<Mission>> {
        let missions = self.missions.read().await;
        Ok(missions.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = Event {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            worker_id: "test-worker".to_string(),
            event_type: "TestEvent".to_string(),
            payload: "{}".to_string(),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&serialized).unwrap();

        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_loop_config_serialization() {
        let config = LoopConfig {
            goal: "Build a game".to_string(),
            max_coins: Some(100),
        };

        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: LoopConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_worker_profile_yaml_serialization() {
        let group = WorkerGroup {
            name: "Full Stack Swarm".to_string(),
            workers: vec![
                WorkerProfile {
                    name: "Git Master".to_string(),
                    role: WorkerRole::Git,
                    model: None,
                },
                WorkerProfile {
                    name: "Lead Coder".to_string(),
                    role: WorkerRole::Coder,
                    model: Some("claude-3-5".to_string()),
                },
            ],
        };

        let yaml = serde_yaml::to_string(&group).unwrap();
        let deserialized: WorkerGroup = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(group, deserialized);
    }

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let bus = InMemoryEventBus::new(10);
        let mut rx = bus.subscribe();

        let event = Event {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            worker_id: "test-worker".to_string(),
            event_type: "TestEvent".to_string(),
            payload: "{}".to_string(),
        };

        bus.publish(event.clone()).await.unwrap();
        let received = rx.recv().await.unwrap();

        assert_eq!(event, received);
    }

    #[tokio::test]
    async fn test_event_store_append_list() {
        let store = InMemoryEventStore::new();
        let event = Event {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            worker_id: "test-worker".to_string(),
            event_type: "TestEvent".to_string(),
            payload: "{}".to_string(),
        };

        store.append(event.clone()).await.unwrap();
        let events = store.list().await.unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[tokio::test]
    async fn test_orchestrator_mission_creation() {
        let orch = BasicOrchestrator::new();
        let mission = orch.create_mission("Initial Setup", vec![
            ("Init Repo".to_string(), "Set up git repository".to_string()),
        ]).await.unwrap();

        assert_eq!(mission.tasks.len(), 1);
        assert_eq!(mission.tasks[0].status, TaskStatus::Pending);
    }

    #[tokio::test]
    async fn test_orchestrator_task_updates() {
        let orch = BasicOrchestrator::new();
        let mission = orch.create_mission("Alpha", vec![
            ("Task 1".to_string(), "Desc 1".to_string()),
        ]).await.unwrap();

        let task_id = mission.tasks[0].id;
        orch.update_task_status(mission.id, task_id, TaskStatus::Running).await.unwrap();

        let missions = orch.get_missions().await.unwrap();
        assert_eq!(missions[0].tasks[0].status, TaskStatus::Running);
    }

    #[tokio::test]
    async fn test_cli_executor_echo() {
        let executor = CliExecutor::new("echo".to_string());
        let result = executor.execute("hello world").await.unwrap();
        assert_eq!(result, "hello world");
    }

    #[tokio::test]
    async fn test_cli_executor_error() {
        let executor = CliExecutor::new("false".to_string());
        let result = executor.execute("").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_bank_progression() {
        let mut bank = Bank::default();
        bank.deposit(100, 50);
        assert_eq!(bank.xp, 100);
        assert_eq!(bank.coins, 50);
        
        bank.deposit(50, 25);
        assert_eq!(bank.xp, 150);
        assert_eq!(bank.coins, 75);
    }
}
