# Logic: 42_linter_worker

## Core Logic

### 1. Tool Integration
- **Rust**: `cargo fmt`, `cargo clippy`.
- **Python**: `black`, `ruff`.
- **JS**: `prettier`, `eslint`.

### 2. Auto-Fix
- Run tools with `--fix` where possible.
- Report remaining errors to `CodeModifier`.

## Data Flow
FileChange -> Linter -> Fix/Report
