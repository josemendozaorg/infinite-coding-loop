# Progress: 09_persistent_history

## Checklist

### Core Storage
- [x] Define `Event` struct. (`lib.rs`)
- [x] Implement `EventStore` trait. (`lib.rs`)
- [x] Create `SqliteEventStore` implementation. (`lib.rs` - uses sqlx)
- [x] Implement `list(session_id)` retrieval.

### Integration
- [x] Async EventBus (`InMemoryEventBus`).
- [x] Connect Store to Bus in `main.rs`.

### Advanced (Pending)
- [ ] Implement Replay mechanism (Time Travel).
- [ ] Snapshotting for performance.