# Logic: 03_worker_team_roster

## User Logic
The user sees a "Barracks" or "Roster" view showing their active team.
They can define distinct **Worker Group Profiles** (e.g., "Full Stack Web Swarm").
The roster is composed of a mix of **AI Workers** (LLM) and **Native Workers** (Rust).

## Native Worker Definitions
The system includes these mandatory deterministic agents:
1.  **Git Worker:** Manages branching, commits, and strictly clean history.
2.  **Research Worker:** Performs web search and documentation verification.
3.  **Code Modifier Worker:** safe I/O operations (Edit, Delete, Move).
4.  **Planner Worker:** Maintains the high-level roadmap and breakdown.
5.  **Progress Manager:** Tracks % complete and time-boxing.
6.  **Memory Manager:** Handles long-term retrieval (RAG).
7.  **Learning Manager:** Records patterns for self-improvement.
8.  **Context Manager:** Optimizes prompt context window usage.
9.  **Context Enricher:** Adds relevant metadata to prompts.
10. **Code Search Worker:** Grep/LSP functionality.
11. **Ongoing Work Saver:** Snapshots state to disk for crash recovery.
12. **Infinite Cycle Resumer:** Bootstraps the system from the last snapshot.
13. **Error Handler:** Catch-all for crash reporting.
14. **Cross-Check Worker:** Validates output of other workers (The "Critic").
15. **Deployment Runner:** CI/CD execution.

## Implementation Strategy
1.  Define `WorkerKind` enum: `Native(NativeRole)` vs `Agent(LlmConfig)`.
2.  Implement `WorkerGroup` struct in `core/profiles.rs`.
3.  Load profiles from `group.yaml`.
4.  Render the "Roster" widget in TUI showing these specific roles.