# Feature 38: Planner Worker (Periodic & Recovery)

## Goal Description
Transition planning from a hardcoded startup step and direct verification hook into a standard `Worker` task.

## User Review Required
> [!IMPORTANT]
> **Architecture Change**: The Loop Engine will now recognize specific JSON output from the `PlannerWorker` to create new missions dynamically.

## Proposed Changes

### ifcl-core
#### [NEW] [planner_worker.rs](file:///home/dev/repos/infinite-coding-loop/ifcl-core/src/planner_worker.rs)
- Create `PlannerWorker` struct in new file.
- Implement `Worker` trait.

#### [MODIFY] [lib.rs](file:///home/dev/repos/infinite-coding-loop/ifcl-core/src/lib.rs)
- Export `PlannerWorker`.

### tui
#### [MODIFY] [main.rs](file:///home/dev/repos/infinite-coding-loop/tui/src/main.rs)
- Update Loop Logic (Failure Handling) to use "Replan Task <ID>" task.
- Update `Task` creation logic to assign this task to "Planner".
- Update execution logic to handle `PlannerWorker` output (JSON missions) and add them to Orchestrator.

## Verification Plan
### Automated Tests
- Unit Test `PlannerWorker` in `ifcl-core`.

### Manual Verification
- Crash Test again, confirming "PlannerWorker" is used for the retrying step.
