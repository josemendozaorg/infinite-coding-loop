# Safety Policy & Operational Limits

**Enforcement**: Sandboxed Runtime

## 1. File System Access

### 1.1. Scope
*   **Allowed**: Read/Write only within the repository root.
*   **Forbidden**: Access to `/etc`, `/var`, `/usr`, or user home directories outside the repo.

### 1.2. Destructive Operations
*   **Rule**: `rm`, `delete`, or `overwrite` commands must require an explicit "Safety Score" check by the Planner.
*   **Backup**: Modifications to existing files should ideally be preceded by a git snapshot or backup.

## 2. Network Access

### 2.1. External APIs
*   **Allowed**:
    *   LLM Providers (OpenAI, Anthropic).
    *   Package Managers (crates.io).
    *   Git Remotes (GitHub/GitLab).
*   **Forbidden**: Arbitrary `curl`/`wget` to unknown domains.

## 3. Resource Usage

### 3.1. LLM Loops
*   **Limit**: Max 10 consecutive retry loops for a single Agent Task. If 10 failures occur, escalate to Human.
*   **Cost Control**: Max tokens per Mission must be strictly capped.
