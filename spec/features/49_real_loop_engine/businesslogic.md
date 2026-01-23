# Feature 49: Real Loop Engine

## Status: In Progress
- [ ] Implement `workspace_path` logic
- [ ] Implement `CliWorker`
- [ ] Pipe real-time stdout/stderr
- [ ] Verify execution

## Overview
The "Real Loop" engine transitions the system from a simulation to a real-world execution environment. It enables the system to:
1.  **Understand Workspace**: Operate within a real file system directory.
2.  **Execute Commands**: Run actual shell commands (`git`, `cargo`, `npm`, etc.).
3.  **Stream Feedback**: Provide real-time `stdout`/`stderr` feedback to the user via the TUI.

## Goals
- Move beyond "simulated" logs to real execution logs.
- Empower `Worker`s to effect real change on the host system.
- Securely contain execution within the specified workspace.
