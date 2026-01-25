use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Represents a single unit of memory (e.g., a summarized interaction, a code snippet context).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: Uuid,
    pub content: String,
    pub metadata: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

/// Trait definition for a Memory Store.
/// This allows us to start with an in-memory version and swap to Qdrant/Postgres later.
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Stores a new memory entry.
    async fn store(&self, content: String, metadata: serde_json::Value) -> Result<Uuid>;

    /// Searches for memories similar to the query.
    /// In a real vector DB, this uses embeddings.
    /// primarily keyword/substring matching for the in-memory version.
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>>;
}

/// An in-memory implementation of the MemoryStore.
/// Useful for testing, early development, or small-scale context retention.
pub struct InMemoryMemoryStore {
    memories: Arc<Mutex<Vec<MemoryEntry>>>,
}

impl InMemoryMemoryStore {
    pub fn new() -> Self {
        Self {
            memories: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for InMemoryMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MemoryStore for InMemoryMemoryStore {
    async fn store(&self, content: String, metadata: serde_json::Value) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let entry = MemoryEntry {
            id,
            content,
            metadata,
            timestamp: Utc::now(),
        };

        let mut lock = self.memories.lock().unwrap();
        lock.push(entry);

        Ok(id)
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>> {
        let lock = self.memories.lock().unwrap();

        // Naive search: returns entries that contain the query string (case-insensitive)
        // In a real system, this would be cosine similarity on embeddings.
        let mut results: Vec<MemoryEntry> = lock
            .iter()
            .filter(|m| m.content.to_lowercase().contains(&query.to_lowercase()))
            .cloned()
            .collect();

        // Sort by recency (newest first) as a simple heuristic for relevance in this naive version
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(results.into_iter().take(limit).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_search() {
        let store = InMemoryMemoryStore::new();

        let _ = store
            .store(
                "The user wants to build a Rust TUI application.".to_string(),
                serde_json::json!({"type": "goal"}),
            )
            .await;

        let _ = store
            .store(
                "We decided to use Ratatui for the frontend.".to_string(),
                serde_json::json!({"type": "decision"}),
            )
            .await;

        let _ = store
            .store(
                "The weather is nice today.".to_string(),
                serde_json::json!({"type": "chitchat"}),
            )
            .await;

        let results = store.search("Rust", 5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("Rust"));

        let results = store.search("Ratatui", 5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("Ratatui"));

        let results = store.search("weather", 5).await.unwrap();
        assert_eq!(results.len(), 1);
    }
}
