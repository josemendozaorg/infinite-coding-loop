# Logic: 44_test_runner

## Core Logic

### 1. Test Discovery
- Detect new test files.
- Command: `cargo test`, `npm test`, `pytest`.

### 2. TDD Cycle support
- Run tests *before* code change (expect fail).
- Run tests *after* code change (expect pass).

## Data Flow
CodeChange -> TestRunner -> TestReport
