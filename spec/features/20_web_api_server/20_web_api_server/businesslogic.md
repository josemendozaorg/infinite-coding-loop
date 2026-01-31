# Logic: 20_web_api_server

## User Logic
- View loop status from a web browser.
- External tools can query the current state.

## Technical Logic
- `axum` web server running on a separate thread.
- JSON endpoints: `/api/status`, `/api/events`, `/api/workers`.
- SSE (Server-Sent Events) for live updates.

## Implementation Strategy
1. Add `axum` dependency.
2. Create router and handlers.
3. Bridge `AppState` to API responses.
