# Logic: 02_live_activity_feed

## Core Logic

### 1. Event Subscription
- **Listener**: The generic `ActivityFeed` component subscribes to the global `EventBus`.
- **Filtering**: Discards events not relevant to user (debug traces vs user-facing progress) unless in Debug mode.

### 2. State Management
- **Circular Buffer**: Keep last N (e.g., 1000) events in memory for display.
- **Scroll State**: Track `scroll_offset` and `auto_scroll_enabled`.

### 3. Formatting
- **Message Parsing**: Format structured events (e.g., `CodeGenerated`) into human-readable strings.
- **Time Formatting**: Relative time (e.g., "+5s") vs Absolute time.

## Data Flow
EventBus -> ActivityFeed Listener -> Filter -> Circular Buffer -> TUI Render Loop