# Logic: 20_web_api_server

## User Logic
- Monitor the loop status from a web browser.
- View live activity feed and metrics remotely.
- Access historical session data via a REST API.

## Technical Logic
- `axum` or `actix-web` server.
- JSON endpoints for `Events` and `State`.
- WebSocket support for real-time feed streaming.

## Implementation Strategy
1. Add `axum` and `tower-http` dependencies.
2. Implement REST endpoints for `list_events` and `get_state`.
3. Add a WebSocket handler to broadcast bus events.
