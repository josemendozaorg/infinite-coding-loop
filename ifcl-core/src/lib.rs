
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

#[async_trait::async_trait]
pub trait EventBus: Send + Sync {
    async fn publish(&self, event: Event) -> anyhow::Result<()>;
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Event>;
}

#[async_trait::async_trait]
pub trait EventStore: Send + Sync {
    async fn append(&self, event: Event) -> anyhow::Result<()>;
    async fn list(&self) -> anyhow::Result<Vec<Event>>;
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
}
