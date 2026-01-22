
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

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
pub mod ui_state;
pub mod wizard;
pub mod session;
pub mod groups;
pub mod marketplace;
pub mod orchestrator;
pub mod context;
pub mod planner;

pub use ui_state::{AppMode, MenuAction, MenuState};
pub use wizard::{SetupWizard, WizardStep};
pub use session::Session;
pub use groups::WorkerGroup;
pub use marketplace::MarketplaceLoader;
pub use orchestrator::{Orchestrator, BasicOrchestrator, WorkerRequest};
pub use context::*;
pub use planner::*;
pub use memory::*;

pub mod memory;
pub mod progress;

pub use progress::{ProgressStats, ProgressManager, BasicProgressManager};

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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy, Default)]
pub enum LoopStatus {
    #[default]
    Running,
    Paused,
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
    async fn list(&self, session_id: Uuid) -> anyhow::Result<Vec<Event>>;
    async fn list_all_sessions(&self) -> anyhow::Result<Vec<Uuid>>;
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

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl EventStore for InMemoryEventStore {
    async fn append(&self, event: Event) -> anyhow::Result<()> {
        let mut events = self.events.write().await;
        events.push(event);
        Ok(())
    }

    async fn list(&self, session_id: Uuid) -> anyhow::Result<Vec<Event>> {
        let events = self.events.read().await;
        Ok(events.iter().filter(|e| e.session_id == session_id).cloned().collect())
    }

    async fn list_all_sessions(&self) -> anyhow::Result<Vec<Uuid>> {
        let events = self.events.read().await;
        let mut sessions: Vec<Uuid> = events.iter().map(|e| e.session_id).collect();
        sessions.sort();
        sessions.dedup();
        Ok(sessions)
    }
}

// BasicOrchestrator moved to orchestrator module

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

#[derive(Debug)]
pub struct CliResult {
    pub stdout: String,
    pub stderr: String,
    pub status: std::process::ExitStatus,
}

pub struct CliExecutor {
    pub binary: String,
}

impl CliExecutor {
    pub fn new(binary: String) -> Self {
        Self { binary }
    }

    pub async fn execute(&self, prompt: &str) -> anyhow::Result<CliResult> {
        let child = tokio::process::Command::new(&self.binary)
            .arg(prompt)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let timeout = tokio::time::Duration::from_secs(60);
        
        match tokio::time::timeout(timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => {
                Ok(CliResult {
                    stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
                    status: output.status,
                })
            }
            Ok(Err(e)) => anyhow::bail!("Execution failed: {}", e),
            Err(_) => anyhow::bail!("CLI execution timed out after 60s"),
        }
    }
}

// BasicOrchestrator implementation moved to orchestrator module

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = Event {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            trace_id: Uuid::new_v4(),
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
            description: "A solid team".to_string(),
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
            session_id: Uuid::new_v4(),
            trace_id: Uuid::new_v4(),
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
            session_id: Uuid::new_v4(),
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            worker_id: "test-worker".to_string(),
            event_type: "TestEvent".to_string(),
            payload: "{}".to_string(),
        };

        store.append(event.clone()).await.unwrap();
        let events = store.list(event.session_id).await.unwrap();

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
        assert_eq!(result.stdout, "hello world");
    }

    #[tokio::test]
    async fn test_cli_executor_error() {
        let executor = CliExecutor::new("false".to_string());
        let result = executor.execute("").await.unwrap();
        assert!(!result.status.success());
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

    #[test]
    fn test_thought_serialization() {
        let payload = ThoughtPayload {
            confidence: 0.85,
            reasoning: vec!["Step 1".to_string(), "Step 2".to_string()],
        };
        let serialized = serde_json::to_string(&payload).unwrap();
        let deserialized: ThoughtPayload = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.confidence, 0.85);
        assert_eq!(deserialized.reasoning.len(), 2);
    }

    #[tokio::test]
    async fn test_loop_status_event_serialization() {
        let event = Event {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            worker_id: "system".to_string(),
            event_type: "LoopStatusChanged".to_string(),
            payload: serde_json::to_string(&LoopStatus::Paused).unwrap(),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&serialized).unwrap();
        let status: LoopStatus = serde_json::from_str(&deserialized.payload).unwrap();

        assert_eq!(status, LoopStatus::Paused);
    }

    #[tokio::test]
    async fn test_manual_command_serialization() {
        let event = Event {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            worker_id: "user".to_string(),
            event_type: "ManualCommandInjected".to_string(),
            payload: "force success".to_string(),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.event_type, "ManualCommandInjected");
        assert_eq!(deserialized.payload, "force success");
    }
}

pub struct SqliteEventStore {
    pool: sqlx::SqlitePool,
}

impl SqliteEventStore {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let pool = sqlx::SqlitePool::connect(database_url).await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                trace_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                worker_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                payload TEXT NOT NULL
            )"
        )
        .execute(&pool)
        .await?;
        
        sqlx::query("ALTER TABLE events ADD COLUMN session_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000'").execute(&pool).await.ok();
        sqlx::query("ALTER TABLE events ADD COLUMN trace_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000'").execute(&pool).await.ok();
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id)").execute(&pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_trace ON events(trace_id)").execute(&pool).await?;

        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl EventStore for SqliteEventStore {
    async fn append(&self, event: Event) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO events (id, session_id, trace_id, timestamp, worker_id, event_type, payload)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(event.id.to_string())
        .bind(event.session_id.to_string())
        .bind(event.trace_id.to_string())
        .bind(event.timestamp.to_rfc3339())
        .bind(event.worker_id)
        .bind(event.event_type)
        .bind(event.payload)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list(&self, session_id: Uuid) -> anyhow::Result<Vec<Event>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String, String, String)>(
            "SELECT id, session_id, trace_id, timestamp, worker_id, event_type, payload FROM events WHERE session_id = ? ORDER BY timestamp ASC"
        )
        .bind(session_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut events = Vec::new();
        for row in rows {
            events.push(Event {
                id: Uuid::parse_str(&row.0)?,
                session_id: Uuid::parse_str(&row.1)?,
                trace_id: Uuid::parse_str(&row.2)?,
                timestamp: chrono::DateTime::parse_from_rfc3339(&row.3)?.with_timezone(&Utc),
                worker_id: row.4,
                event_type: row.5,
                payload: row.6,
            });
        }
        Ok(events)
    }

    async fn list_all_sessions(&self) -> anyhow::Result<Vec<Uuid>> {
        let rows = sqlx::query_as::<_, (String,)>(
            "SELECT DISTINCT session_id FROM events"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(Uuid::parse_str(&row.0)?);
        }
        Ok(sessions)
    }
}

#[cfg(test)]
mod sql_tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlite_event_store_persistence() {
        let store = SqliteEventStore::new("sqlite::memory:").await.unwrap();
        let session_id = Uuid::new_v4();
        let event = Event {
            id: Uuid::new_v4(),
            session_id,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            worker_id: "test".to_string(),
            event_type: "TestEvent".to_string(),
            payload: "test payload".to_string(),
        };

        store.append(event.clone()).await.unwrap();
        let events = store.list(session_id).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, event.id);
        assert_eq!(events[0].payload, event.payload);
    }
}
pub mod learning;
mod session_replay_tests;
