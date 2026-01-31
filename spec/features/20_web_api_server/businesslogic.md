# Logic: 20_web_api_server

## Core Logic

### 1. Web Server
- **Framework**: `axum` (Rust).
- **State**: Shared reference to `AppState` (EventBus, WorkerManager).
- **Middleware**: CORS, Trace/Logging, Auth (optional).

### 2. Endpoints
- `GET /api/v1/mission`: Get current mission status.
- `GET /api/v1/events`: SSE (Server-Sent Events) stream of the EventBus.
- `POST /api/v1/instruction`: Send user feedback/command.

## Data Flow
External Client -> HTTP Req -> Axum Route -> AppState -> Internal Channel -> Actor System
