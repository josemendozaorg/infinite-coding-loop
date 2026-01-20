# Logic: 04_mission_control_grid
*(Note: This feature covers the Active Grid visualization, but implies the Engine logic backing it).*

## User Logic
The "Window" must show what every Worker is doing in real-time.
The system runs an **Infinite Loop** of self-improvement.

## Core Logic: The Infinite Cycle
The Engine implements the following state machine for each "Quest":

1.  **Plan:** Breakdown high-level goal.
2.  **Socratic Phase (Mandatory):**
    - *Action:* The **Researcher** and **Planner** must challenge assumptions.
    - *Example:* "Is this architecture scalable? Let's check docs."
    - *Fallback:* If unsure, ask User (God Mode). If Timeout, allow Research Worker to decide.
3.  **Implement:** Assign tasks to Coders.
4.  **Test:** Run Linter/Compiler/Test Runners.
5.  **Review:** Cross-Check Worker validates code.
6.  **Deploy:** Deployment Runner executes.
7.  **Learn:** Learning Manager records metrics.
8.  **Repeat:** Pick next improvement or bugfix.

## Foundational Logic (Technical)
- **State Projection:** The TUI Grid is a read-only projection of `WorldState`.
- **Tick Rate:** The engine ticks at 30Hz or event-driven speed.
- **Worker Status:**
  - `Idle` (Grey)
  - `Thinking` (Amber Pulse)
  - `Working` (Green)
  - `Error` (Red Flash)

## Implementation Strategy
1.  **State Struct:** `WorldState` holds a `HashMap<WorkerID, WorkerStatus>`.
2.  **Event Consumption:** `SystemStarted`, `TaskAssigned`, `TaskCompleted` events update the status colors.
3.  **Socratic Interruption:** The engine must support a "Blocking" state where it waits for Research or User Input before proceeding to Implementation.