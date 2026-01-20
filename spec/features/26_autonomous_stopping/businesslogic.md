# Logic: 26_autonomous_stopping

## Core Logic

### 1. Criteria Evaluation
- **Manual**: Hard limit (Budget exceeded, User pressed Stop).
- **Goal Check**: Periodically run an "Evaluator Agent" with the Goal + Current Codebase state.
  - Prompt: "Does the application fulfill the goal 'X'? Yes/No/Partial"

### 2. Shutdown Sequence
- Stop all actors.
- Persist final state.
- Generate Report.

## Data Flow
ProgressUpdate -> Evaluator -> StopSignal
