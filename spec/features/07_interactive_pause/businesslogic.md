# Logic: 06_interactive_pause & 07_god_mode_intervention

## User Logic
- **Pause:** User presses `SPACE`. The infinite loop halts *safely*.
- **Feedback:** User types "Change DB to Postgres".
- **Impact Analysis:** System calculates which pending tasks need to be cancelled/refactored.
- **Resume:** loop continues with new context.

## Core Logic
1.  **Signal Handling:** The Engine checks an `AtomicBool` flag `IS_PAUSED` every tick.
2.  **Safe Halt:** Workers are not killed; they finish their current *micro-step* (e.g. current stream chunk) and then wait.
3.  **Instruction Injection:**
    - User input creates an `InterventionEvent`.
    - **Planner Worker** receives this event first.
    - Planner executes a "Re-Plan" phase.
    - Planner issues `TaskCancellation` events for obsolete tasks.
    - Planner issues new `TaskAssignment` events.

## Implementation Strategy
1.  **Input Handling:** TUI captures `Key::Space`.
2.  **Overlay:** Render "PAUSED" modal over the grid.
3.  **Event Injection:** `EventBus::publish(Event::Intervention { ... })`.