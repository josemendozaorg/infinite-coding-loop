use anyhow::Result;
use async_trait::async_trait;
use crate::domain::Event;

#[async_trait]
pub trait EventBus: Send + Sync {
    async fn publish(&self, event: Event) -> Result<()>;
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

#[async_trait]
impl EventBus for InMemoryEventBus {
    async fn publish(&self, event: Event) -> Result<()> {
        let _ = self.tx.send(event);
        Ok(())
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}
