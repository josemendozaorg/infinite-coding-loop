# Logic: 22_session_isolation

## User Logic
- Each "New Game" creates a completely isolated session.
- Sessions have their own event history, configuration, workers, missions, and results.
- Users can switch between sessions without data leaking.

## Technical Logic
- Generate unique `session_id: Uuid` for each new loop.
- Prefix database tables or use separate SQLite files per session.
- Store session metadata (goal, created_at, last_active) for the session picker.

## Implementation Strategy
1. Add `Session` struct with `id`, `name`, `config`, `created_at`.
2. Update `SqliteEventStore` to filter events by `session_id`.
3. Update `AppState` to track `current_session_id`.
4. Modify "Load Game" menu to list available sessions.
