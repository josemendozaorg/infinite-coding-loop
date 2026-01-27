# Logic: 08_god_mode_intervention

## Core Logic

### 1. Override Mechanism
- **Interrupt**: User presses `Space` or `Esc`. System pauses.
- **Command Injection**: User enters "Don't use unwrap() here".
- **Action**: Inject `HumanFeedbackEvent` into the context.

### 2. Direct Control
- **Edit**: User can edit the Plan DAG directly.
- **Force State**: User can force a task state from `InProgress` to `Done` or `Failed`.

## Data Flow
User Input -> Interrupt -> EventBus -> Planner/Worker Adjustment