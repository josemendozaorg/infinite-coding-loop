# UI Specification

## Workspace Selection (Wizard Step 3)
- **Input**: User selects a local directory path.
- **Validation**: Checks if path exists, is writable, and is empty (warning if not).

## AI Terminal (Panel 5)
- **Real-Time Output**: Displays raw stdout/stderr from executed commands.
- **Color Coding**: 
    - `stdout`: Green/White
    - `stderr`: Red/Yellow
    - `System`: Blue (e.g. "Starting task...")

## Mission Control (Panel 3)
- **Task Status**: Updates immediately based on command exit code (0 = Success, Non-0 = Failure).
