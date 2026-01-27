# Logic: 18_sandboxed_executor

## Core Logic

### 1. Isolation
- **Technology**: Docker (default) or bubblewrap (Linux).
- **Volume Mounts**: Mount only necessary project files.
- **Network**: Restrict network access (allow only specific domains for `cargo build`).

### 2. Execution
- Run `cargo test` inside container.
- Capture Exit Code.

## Data Flow
Command -> Sandbox -> Result
