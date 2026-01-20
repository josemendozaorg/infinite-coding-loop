# Logic: 39_memory_manager

## Core Logic

### 1. Memory Store
- **Vector DB**: Interface for `qdrant` or `pgvector`.
- **Operations**:
  - `store(content, metadata)`: Embed and save.
  - `search(query, top_k)`: RAG retrieval.

### 2. Context Manager Integration
- **Summarization**: When context fills up, summarize oldest memories into a higher-level abstract memory and store in Long-Term Memory.
- **Session Context**: Rolling window of recent interactions.

## Data Flow
Worker -> ContextManager -> MemoryManager -> VectorDB
