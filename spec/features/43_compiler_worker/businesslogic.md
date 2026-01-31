# Logic: 43_compiler_worker

## Core Logic

### 1. Build Execution
- **Command**: `cargo build`, `npm build`.
- **Stream Output**: Capture stdout/stderr for Activity Feed.

### 2. Error Analysis
- Parse compiler errors (file, line, message).
- Create `FixTask` for `CodeModifier`.

## Data Flow
CodeChange -> Compiler -> Success/Failure(Errors)
