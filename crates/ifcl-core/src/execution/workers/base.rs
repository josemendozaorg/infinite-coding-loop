use async_trait::async_trait;
use uuid::Uuid;
use std::sync::Arc;
use anyhow::Result;
use crate::domain::{WorkerRole, WorkerProfile, Task};
use crate::infrastructure::events::bus::EventBus;

#[async_trait]
pub trait Worker: Send + Sync {
    fn id(&self) -> &str;
    fn role(&self) -> WorkerRole;
    fn metadata(&self) -> &WorkerProfile;
    async fn execute(
        &self,
        bus: Arc<dyn EventBus>,
        task: &Task,
        workspace_path: &str,
        session_id: Uuid,
    ) -> Result<String>;
}
