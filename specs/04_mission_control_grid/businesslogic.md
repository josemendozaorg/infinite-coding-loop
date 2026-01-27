# Logic: 04_mission_control_grid

## Core Logic

### 1. Dashboard Controller
- **Layout Manager**: Handles sizing of TUI Constraints based on terminal size.
- **Aggregator**: Pulls data from `TokenCostTracker`, `ProgressManager`, and `WorkerManager` to update the top-level widgets.

### 2. View Mode State
- **Modes**: `Dashboard`, `FocusMode(WidgetID)`.
- **Handling Resizes**: React to terminal resize events to re-calculate layout.

## Data Flow
ProgressManager/WorkerManager -> DashboardController -> TUI Draw -> Screen