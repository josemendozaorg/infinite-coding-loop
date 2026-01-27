# UI: 03_worker_team_roster

## Visual Components

### 1. Roster List
- **Layout**: Sidebar or dedicated "Team" tab.
- **Content**: List of active workers with icons.
  - "🤖 Planner (Claude-3)" [IDLE]
  - "🤓 Coder (Gemini-Pro)" [BUSY]
  - "🕵️ Researcher (Perplexity)" [BUSY]

### 2. Worker Details Pane
- Select a worker to see:
  - **Current Task**: "Implementing fn calculate_pi()"
  - **Queue Size**: 3 pending tasks
  - **Uptime**: 1h 20m
  - **Tokens Used**: 145k

## User Interactions
- **Add Worker**: Button to spawn a new worker instance.
- **Kill Worker**: Button to terminate a stuck worker.
- **Reassign**: Drag-and-drop tasks (advanced).