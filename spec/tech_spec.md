# Technical Architecture Specification

## 1. Technology Stack
- **Language:** Rust (2024 Edition)
- **Runtime:** `tokio` (Async I/O)
- **TUI Framework:** `ratatui` + `crossterm`
- **Database:** `sqlx` (SQLite) for Event Log, `sled` or `redb` for KV caching.
- **Serialization:** `serde` (JSON/YAML configs), `prost` (Protobuf messages).
- **RPC:** `tonic` (gRPC) for inter-worker communication.

## 2. High-Level Architecture
The system is a **Modular Monolith** designed to be split into Microservices later.

### 2.1. Layer 1: The Core (`/core`)
- **Event Bus:** The "Spine" of the application. Receives `Events`, writes them to disk, and broadcasts them to Subscribers.
- **Profile Manager:** Loads/Saves `WorkerGroupProfile` and `LoopProfile`.
- **State Engine:** Reconstructs the current world state from the Event Log.

### 2.2. Layer 2: The Worker Grid (`/workers`)
- **Worker Trait:** A standard Rust Trait that all workers must implement.
  ```rust
  trait Worker {
      async fn handle_message(&self, msg: ProtocolMessage) -> Result<ProtocolMessage>;
      async fn tick(&self, ctx: Context); // For game loop updates
  }
  ```
- **Native Workers:** compiled directly into the binary (Git, Linter, Compiler).
- **AI Workers:** wrappers around HTTP Clients (Claude, Gemini) that adapt responses to the Protocol.

### 2.3. Layer 3: The Interface (`/tui` & `/api`)
- **TUI:** A `ratatui` application running on a separate thread, consuming a read-only projection of the State.
- **API Server:** A `warp` or `axum` server exposing the Event Stream and State Query endpoints for future web clients.

## 3. Security & Sandboxing
- **Network:** AI Workers interact with external APIs (Ollama, Anthropic). Native Workers (like Git) have restricted file system access.
- **Docker:** (Future) Code execution should happen inside ephemeral Docker containers controlled by the `ExecutionWorker`.
