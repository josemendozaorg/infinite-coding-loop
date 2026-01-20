# UI: 04_mission_control_grid

## Visual Components

### 1. The Grid (Dashboard Layout)
- **Concept**: A tiling window manager style layout within the Terminal.
- **Default Layout**:
  - **Top Left**: Project Status (Goal, Progress %, Budget Used).
  - **Top Right**: Resource Monitor (Tokens/sec, Cost).
  - **Center**: Active View (Tabbed: Activity Feed, File Tree, Worker Graph).
  - **Bottom**: Event Log (if not in Center).

### 2. Status Widgets
- **Progress Bar**: Overall mission completeness estimation.
- **Budget Badge**: "$2.50 / $10.00" (changes color as it nears limit).
- **Time Badge**: "00:05:23 elapsed".

### 3. Worker Avatars (Game-like)
- Small blocks representing active workers.
- **State Indicators**: 
  - 🟢 Idle
  - 🔵 Thinking
  - 🟠 Writing Code
  - 🟣 Testing
- **Animation**: Simple 2-frame ASCII animation or pulsing color when active.

## User Interactions
- **Tab Switching**: `Tab` / `Shift+Tab` to cycle focus.
- **Layout Toggle**: `F1`, `F2`, `F3` presets (e.g., Focus Logs, Focus Code, Focus Graph).