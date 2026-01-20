# Master Product Specification: Infinite Coding Loop ("The Game")

## 1. Vision
The **Infinite Coding Loop** is an autonomous, self-evolving software development engine modeled as a **Real-Time Strategy (RTS) Game**. Users do not "run a script"; they "manage a factory" of AI workers (Units) that collaborate to solve engineering Quests.

## 2. Core Pillars
1.  **Gamification:** The interface must feel like a game. Workers are characters, tasks are quests, successful tests earn coins. This makes observability engaging.
2.  **Immutability:** The entire system is Event Sourced. "If it's not in the Event Log, it didn't happen." This ensures perfect reproducibility and time-travel debugging.
3.  **Modularity:** The system is composed of loose coupled Workers communicating via a strict Protocol. Workers can be local, remote, Native (Rust), or AI (LLM).
4.  **Self-Correction:** The cycle includes mandatory "Socratic Phases" and "Cross-Checks" to validate assumptions and code quality.
5.  **The Infinite Loop:** The engine operates on a continuous "Plan-Execute-Verify" cycle. It remembers state across sessions, allowing it to seamlessly resume complex, multi-day engineering tasks without human hand-holding.
6.  **Relational Knowledge Graph:** All entities (Missions, Tasks, Workers, Artifacts) are nodes in a persistent graph. This allows for clear visualization of dependencies ("Who is working on what?", "Which file caused this error?") and enables complex reasoning about the project structure.

## 3. System Scope
- **Inputs:** A high-level Goal ("Build a Blog"), a Worker Profile ("Team A"), and Constraints ("Max $10 budget").
- **Outputs:** A Git Repository with formulated code, tests, and deployment scripts. The system runs indefinitely to maintain and improve this output.
- **Platform:** Native Desktop Application (Rust) with TUI, exposing an API for future Web/Mobile clients.

## 4. Documentation Suite
This master document is supported by detailed technical specifications:
- **[Technical Architecture](tech_spec.md):** Rust internals, Module boundaries, API design.
- **[Data Model](data_spec.md):** Schema for Events, Profiles, and Logs.
- **[Protocol Specification](protocol_spec.md):** gRPC/Protobuf definitions for Worker communication.
- **[UX/UI Design Flow](ui_ux_spec.md):** Visual storyboards and screen sequences.
