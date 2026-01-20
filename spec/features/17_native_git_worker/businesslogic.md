# Logic: 17_native_git_worker

## Core Logic

### 1. Git Operations
- **Library**: `git2` (libgit2 bindings).
- **Actions**: `init`, `add`, `commit`, `checkout`, `branch`, `push`.
- **Safety**: Ensure unclean state doesn't block critical ops (stash if needed).

### 2. Commit Message Gen
- **Auto-Commit**: If not specified, LLM generates Conventional Commits message based on diff.

## Data Flow
FileChange -> GitWorker -> Commit
