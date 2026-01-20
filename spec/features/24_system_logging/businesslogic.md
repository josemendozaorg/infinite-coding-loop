# Logic: 24_system_logging

## User Logic
- View raw stdout/stderr from CLI workers for deep debugging.
- Access a persistent "Audit Log" of all system actions.

## Technical Logic
- Capture `Output` from `tokio::process::Command` in `CliExecutor`.
- Emit detailed `Log` events for internal transitions.
- Store logs in the `EventStore` with level (DEBUG, INFO, WARN, ERROR).

## Implementation Strategy
1. Extend `Event` types to include `Log` and `ExecutionLog`.
2. Update `CliExecutor` to stream logs to the event bus.
3. Add a log rotation policy to the `SqliteEventStore`.
