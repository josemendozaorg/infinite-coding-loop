use anyhow::Result;
use async_trait::async_trait;
use crate::domain::Event;
use uuid::Uuid;
use chrono::Utc;

#[async_trait]
pub trait EventStore: Send + Sync {
    async fn append(&self, event: Event) -> Result<()>;
    async fn list(&self, session_id: Uuid) -> Result<Vec<Event>>;
    async fn list_all_sessions(&self) -> Result<Vec<Uuid>>;
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

#[async_trait]
impl EventStore for InMemoryEventStore {
    async fn append(&self, event: Event) -> Result<()> {
        let mut events = self.events.write().await;
        events.push(event);
        Ok(())
    }

    async fn list(&self, session_id: Uuid) -> Result<Vec<Event>> {
        let events = self.events.read().await;
        Ok(events
            .iter()
            .filter(|e| e.session_id == session_id)
            .cloned()
            .collect())
    }

    async fn list_all_sessions(&self) -> Result<Vec<Uuid>> {
        let events = self.events.read().await;
        let mut sessions: Vec<Uuid> = events.iter().map(|e| e.session_id).collect();
        sessions.sort();
        sessions.dedup();
        Ok(sessions)
    }
}

pub struct SqliteEventStore {
    pool: sqlx::SqlitePool,
}

impl SqliteEventStore {
    pub async fn new(database_url: &str) -> Result<Self> {
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
            )",
        )
        .execute(&pool)
        .await?;

        sqlx::query("ALTER TABLE events ADD COLUMN session_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000'").execute(&pool).await.ok();
        sqlx::query("ALTER TABLE events ADD COLUMN trace_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000'").execute(&pool).await.ok();
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id)")
            .execute(&pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_trace ON events(trace_id)")
            .execute(&pool)
            .await?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl EventStore for SqliteEventStore {
    async fn append(&self, event: Event) -> Result<()> {
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

    async fn list(&self, session_id: Uuid) -> Result<Vec<Event>> {
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

    async fn list_all_sessions(&self) -> Result<Vec<Uuid>> {
        let rows = sqlx::query_as::<_, (String,)>("SELECT DISTINCT session_id FROM events")
            .fetch_all(&self.pool)
            .await?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(Uuid::parse_str(&row.0)?);
        }
        Ok(sessions)
    }
}
