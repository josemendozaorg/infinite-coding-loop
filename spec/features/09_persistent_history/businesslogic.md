# Logic: 09_persistent_history

## Core Logic

### 1. Event Store
- **Append-Only Log**: All state changes are immutable events.
- **Backend**: SQLite (local) or PostgreSQL (server).
- **Traits**: `EventStore` with `append()`, `read_stream()`.

### 2. Replay Mechanism
- **Time Travel**: Rebuild state by replaying events from t=0 to t=N.
- **Snapshotting**: Periodically save aggregations (e.g., every 1000 events) to speed up load times.

## Data Flow
EventBus -> PersistActor -> Database