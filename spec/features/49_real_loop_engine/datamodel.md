# Data Model

## Mission
- **workspace_path**: `Option<String>` - The absolute path to the project root.

## CliWorker
- **Implements**: `Worker` trait.
- **Capabilities**: Executes shell commands via `tokio::process::Command`.

## Events
- **WorkerOutputPayload**: New event type for streaming execution logs.
    - `task_id`: Uuid
    - `stream`: String ("stdout" | "stderr")
    - `line`: String
