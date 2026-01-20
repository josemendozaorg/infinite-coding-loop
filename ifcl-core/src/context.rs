use crate::Event;
use serde::{Deserialize, Serialize};

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
pub trait ContextPruner: Send + Sync {
    fn prune(&self, events: &[Event], max_tokens: usize, counter: &dyn TokenCounter) -> ManagedContext;
}

/// A pruner that uses a sliding window (removes oldest events first).
pub struct SlidingWindowPruner;

impl ContextPruner for SlidingWindowPruner {
    fn prune(&self, events: &[Event], max_tokens: usize, counter: &dyn TokenCounter) -> ManagedContext {
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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use chrono::Utc;

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

    #[test]
    fn test_context_pruning() {
        let counter = SimpleTokenCounter;
        let pruner = SlidingWindowPruner;
        
        let events = vec![
            create_mock_event("Old event"),
            create_mock_event("Middle event"),
            create_mock_event("Recent event"),
        ];

        // Set a limit that only fits 2 events (each ~8-10 tokens)
        let managed = pruner.prune(&events, 25, &counter);
        
        assert_eq!(managed.events.len(), 2);
        assert_eq!(managed.pruned_count, 1);
        // Should keep recent events
        assert_eq!(managed.events[0].payload, "Middle event");
        assert_eq!(managed.events[1].payload, "Recent event");
    }
}
