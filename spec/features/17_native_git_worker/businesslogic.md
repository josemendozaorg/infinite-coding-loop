# Logic: 17_native_git_worker

## User Logic
- Automated commits, branching, and PR creation.
- Persistent history of the project being built.

## Technical Logic
- Native worker (compiled into binary) that wraps `git` CLI calls.
- High-level methods: `commit_all(msg)`, `create_branch(name)`, `merge_branch(name)`.

## Implementation Strategy
1. Implement `GitWorker` using `tokio::process::Command`.
2. Map orchestrator missions (e.g., "Save Progress") to Git worker commands.
