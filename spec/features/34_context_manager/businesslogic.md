# Logic: 34_context_manager

## Core Logic

### 1. Context Assembly
- **Input**: List of relevant files, chat history, current task.
- **Constraint**: Max context window (e.g., 200k tokens).
- **Priority**:
  1. System Prompt
  2. Current Task
  3. Active File Content
  4. Recent History
  5. Related Snippets

### 2. Pruning
- Remove oldest history.
- Summarize "middle" history.
- Truncate large files (view only relevant chunks).

## Data Flow
WorkerRequest -> ContextManager -> EnrichedPrompt -> LLM
