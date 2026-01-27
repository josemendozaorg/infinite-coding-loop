# Logic: 41_progress_manager

## Core Logic

### 1. Tracking
- **Task completion**: % of tasks in `Done` state.
- **Stuck Detection**: If no tasks complete in X minutes, flag as "Stalled".

### 2. Estimation
- Use average task duration to estimate remaining time.

## Data Flow
TaskUpdate -> ProgressManager -> ProgressStats -> MissionControlGrid
