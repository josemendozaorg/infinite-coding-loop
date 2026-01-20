# Logic: 05_ai_cli_workers

## Core Logic

### 1. LLM Client Abstraction
- **Trait**: `LLMClient` with `complete()`, `stream()`.
- **Implementations**:
  - `ClaudeClient` (Anthropic API)
  - `GeminiClient` (Google Vertex/Studio)
  - `OllamaClient` (Local)

### 2. Worker Actor
- **Message Handling**: Receive `Task`, Process with LLM, Send `Result`.
- **Rate Limiting**: Handle 429 errors with exponential backoff.
- **Context Management**: Prune history if it exceeds limits.

## Data Flow
TaskQueue -> WorkerActor -> ContextBuilder -> LLMClient -> ResponseParser -> ResultBus
