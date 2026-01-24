# Feature 35: Error Handler (Self-Healing) Implementation Plan

## Goal Description
Implement a self-healing mechanism that automatically retries failed tasks and escalates to a planner for replanning if retries are exhausted.

## User Review Required
> [!IMPORTANT]
> **Data Model Change**: `Task` struct in `ifcl-core` will be modified to include `retry_count`.

## Proposed Changes

### ifcl-core
#### [MODIFY] [lib.rs](file:///home/dev/repos/infinite-coding-loop/ifcl-core/src/lib.rs)
- Add `pub retry_count: u32` to `Task` struct.
- Default to 0.

### tui
#### [MODIFY] [main.rs](file:///home/dev/repos/infinite-coding-loop/tui/src/main.rs)
- Update main loop to handle `Err` from `execute_task`.
- Implement retry logic (max 3 retries).
- Implement replan trigger (mock/stub if F38 not ready).

### ifcl-core
#### [MODIFY] [planner.rs](file:///home/dev/repos/infinite-coding-loop/ifcl-core/src/planner.rs)
- Add `replan` method to `Planner` trait.
- Implement stub in `BasicPlanner`.

## Verification Plan

### Automated Tests
- `cargo test -p ifcl-core` to verify struct changes.
- New test in `planner.rs` for replan.

### Manual Verification
- Run `cargo run --bin tui -- --headless --goal "Fail Test"` with a failing task.
- Verify retry logs.
