# UI: 02_live_activity_feed

## Visual Components

### 1. Activity Log Panel
- **Position**: Usually the bottom 1/3 of the screen or a dedicated "Logs" tab.
- **Columns**: Timestamp | Worker | Type | Message
- **Color Coding**:
  - `INFO`: White/Gray
  - `WARN`: Yellow
  - `ERROR`: Red
  - `SUCCESS`: Green
  - `THINKING`: Blue

### 2. Filters & Controls
- **Toggle Visibility**: `[x] Planner [ ] Research [x] Coder`
- **Auto-Scroll**: Toggle on/off (defaults to on).
- **Search/Grep**: `/` to input search regex to filter logs live.

## User Interactions
- **Scroll**: Up/Down arrows to pause auto-scroll and review history.
- **Expand**: Enter on a log line to see full details (e.g., if message is truncated or has a large payload).