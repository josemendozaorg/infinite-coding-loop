# Logic: 23_structured_tracing

## User Logic
- See the "causality chain" of a mission (e.g., why a task was created).
- Follow a specific request across different workers and machines.

## Technical Logic
- Add `trace_id: Uuid` to the `Event` struct.
- Propagation of `trace_id` from parent events to children (e.g., `MissionCreated` -> `TaskAssigned`).
- Filtering and visualization of traces in the TUI/API.

## Implementation Strategy
1. Update `Event` struct and DB schema with `trace_id`.
2. Update `Orchestrator` to preserve and propagate trace IDs.
3. Add a "Trace View" to the TUI to isolate events by `trace_id`.
