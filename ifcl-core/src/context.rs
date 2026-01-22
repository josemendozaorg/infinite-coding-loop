use crate::{Event, MemoryStore};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use std::sync::Arc;

/// Basic trait for counting tokens in a string or list of events.
pub trait TokenCounter: Send + Sync {
    fn count_tokens(&self, text: &str) -> usize;
    fn estimate_event_tokens(&self, event: &Event) -> usize;
}

/// A simple token counter that uses a rough character-to-token ratio (e.g., 4 chars per token).
pub struct SimpleTokenCounter;

impl TokenCounter for SimpleTokenCounter {
    fn count_tokens(&self, text: &str) -> usize {
        // Rough heuristic: 1 token per 4 characters
        text.len() / 4
    }

    fn estimate_event_tokens(&self, event: &Event) -> usize {
        let payload_tokens = self.count_tokens(&event.payload);
        let id_tokens = 8; // UUIDs and metadata
        payload_tokens + id_tokens
    }
}

/// A struct representing a managed context for an AI worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedContext {
    pub events: Vec<Event>,
    pub estimated_tokens: usize,
    pub pruned_count: usize,
}

/// Trait for pruning context history to fit within a limit.
#[async_trait]
pub trait ContextPruner: Send + Sync {
    async fn prune(&self, events: &[Event], max_tokens: usize, counter: &dyn TokenCounter) -> ManagedContext;
}

/// A pruner that uses a sliding window (removes oldest events first).
pub struct SlidingWindowPruner;

#[async_trait]
impl ContextPruner for SlidingWindowPruner {
    async fn prune(&self, events: &[Event], max_tokens: usize, counter: &dyn TokenCounter) -> ManagedContext {
        let mut selected = Vec::new();
        let mut current_tokens = 0;
        let mut pruned = 0;

        // Iterate backwards to keep the most recent events
        for event in events.iter().rev() {
            let tokens = counter.estimate_event_tokens(event);
            if current_tokens + tokens <= max_tokens {
                selected.push(event.clone());
                current_tokens += tokens;
            } else {
                pruned += 1;
            }
        }

        // Reverse back to chronological order
        selected.reverse();

        ManagedContext {
            events: selected,
            estimated_tokens: current_tokens,
            pruned_count: pruned,
        }
    }
}

/// A pruner that uses RAG (Retrieval-Augmented Generation) via a MemoryStore.
pub struct VectorPruner {
    pub store: Arc<dyn MemoryStore>,
}

#[async_trait]
impl ContextPruner for VectorPruner {
    async fn prune(&self, events: &[Event], max_tokens: usize, counter: &dyn TokenCounter) -> ManagedContext {
        // Simple heuristic: Take the last event as the query for relevance
        let query = events.last().map(|e| e.payload.as_str()).unwrap_or("");
        
        // Search memory for relevant snippets
        let memories = self.store.search(query, 5).await.unwrap_or_default();
        
        let mut selected = Vec::new();
        let mut current_tokens = 0;
        let mut pruned = 0;

        // 1. Always prioritize the most recent few events for immediate continuity
        for event in events.iter().rev().take(5) {
            let tokens = counter.estimate_event_tokens(event);
            if current_tokens + tokens <= max_tokens / 2 {
                selected.push(event.clone());
                current_tokens += tokens;
            }
        }

        // 2. Add relevant memories as "virtual events" or just collect their tokens
        // For simplicity, we'll just use the events that match the memories' content
        // In a real system, memories themselves would be part of the context.
        for memory in memories {
            // Find events that match the memory content
            if let Some(event) = events.iter().find(|e| e.payload.contains(&memory.content)) {
                if !selected.iter().any(|s| s.id == event.id) {
                    let tokens = counter.estimate_event_tokens(event);
                    if current_tokens + tokens <= max_tokens {
                        selected.push(event.clone());
                        current_tokens += tokens;
                    }
                }
            }
        }

        // 3. Fallback: If we still have room, add recent events
        for event in events.iter().rev() {
            if !selected.iter().any(|s| s.id == event.id) {
                let tokens = counter.estimate_event_tokens(event);
                if current_tokens + tokens <= max_tokens {
                    selected.push(event.clone());
                    current_tokens += tokens;
                } else {
                    pruned += 1;
                }
            }
        }

        // Sort chronologically
        selected.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        ManagedContext {
            events: selected,
            estimated_tokens: current_tokens,
            pruned_count: pruned,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use chrono::Utc;
    use crate::InMemoryMemoryStore;

    fn create_mock_event(payload: &str) -> Event {
        Event {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            worker_id: "test".to_string(),
            event_type: "Test".to_string(),
            payload: payload.to_string(),
        }
    }

    #[test]
    fn test_token_estimation() {
        let counter = SimpleTokenCounter;
        let text = "Hello world"; // 11 chars -> ~2 tokens
        assert!(counter.count_tokens(text) >= 2);
        
        let event = create_mock_event("Large payload with some data...");
        let tokens = counter.estimate_event_tokens(&event);
        assert!(tokens > 8);
    }

    #[tokio::test]
    async fn test_sliding_window_pruning() {
        let counter = SimpleTokenCounter;
        let pruner = SlidingWindowPruner;
        
        let events = vec![
            create_mock_event("Old event"),
            create_mock_event("Middle event"),
            create_mock_event("Recent event"),
        ];

        let managed = pruner.prune(&events, 25, &counter).await;
        
        assert_eq!(managed.events.len(), 2);
        assert_eq!(managed.pruned_count, 1);
        assert_eq!(managed.events[0].payload, "Middle event");
        assert_eq!(managed.events[1].payload, "Recent event");
    }

    #[tokio::test]
    async fn test_vector_pruning() {
        let counter = SimpleTokenCounter;
        let store = Arc::new(InMemoryMemoryStore::new());
        let _ = store.store("Important info".to_string(), serde_json::json!({})).await;
        
        let pruner = VectorPruner { store };
        
        let events = vec![
            create_mock_event("Old irrelevant event"),
            create_mock_event("Important info"),
            create_mock_event("Another irrelevant"),
            create_mock_event("Current query content"),
        ];

        let managed = pruner.prune(&events, 40, &counter).await;
        
        // Should include "Important info" even if it's older, and the recent ones
        assert!(managed.events.iter().any(|e| e.payload == "Important info"));
        assert!(managed.events.iter().any(|e| e.payload == "Current query content"));
    }
}
