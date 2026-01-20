# Logic: 38_planner_worker

## Core Logic

### 1. Decomposition Strategy
- **Input**: High-level Goal ("Build a login system").
- **Process**: LLM recurses to break down into smaller sub-tasks.
- **Output**: A Directed Acyclic Graph (DAG) of tasks.

### 2. Replanning
- **Trigger**: When a task fails or user feedback changes scope.
- **Action**: Invalidate downstream tasks, generate new sub-plan.

## Data Flow
Goal -> Planner -> Task DAG -> TaskQueue
