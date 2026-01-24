# Feature 38: Planner Worker (Walkthrough)

## Overview
The **Planner Worker** (F38) transforms the planner from a static infrastructure component into an active participant.

## Changes
- **New Component**: `PlannerWorker` in `ifcl-core`.
- **Worker Trait**: Handles "Replan" tasks by invoking the underlying Planner.
- **Loop Engine**:
    - Replaced direct `planner.replan()` call with a **Delegation Pattern**.
    - Creates a transient `Task` (assigned to "Planner") containing the failure context.

## Verification
Verified using Headless CLI mode with "Crash Test". Logs confirmed delegation to Planner Worker.
