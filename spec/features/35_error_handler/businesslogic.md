# Logic: 35_error_handler

## Core Logic

### 1. Recovery Strategies
- **Retry**: Simple exponential backoff for transient network errors.
- **Context Pruning**: If "Context Length Exceeded", trigger `ContextManager` to prune and retry.
- **Planner Escalation**: If "Logic Error" or loop detected, ask `PlannerWorker` to replan.

### 2. Human Escalation
- If configured, pause and await User input in `08_god_mode_intervention` before failing completely.

## Data Flow
ErrorEvent -> ErrorHandler -> Action (Retry/Replan/Stop)
