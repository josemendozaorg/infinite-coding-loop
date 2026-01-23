# F48 - Validation Progress

- [ ] **Specs**
    - [x] Business Logic
    - [x] UI/UX
    - [x] Data Model
    - [x] API

- [ ] **Implementation**
    - [ ] `ToolCheckStatus` struct
    - [ ] `EnvironmentVerifier` service/worker
    - [ ] Integration with TUI display
    - [ ] "Setup Mission" generation logic

- [ ] **Tests**
    - [ ] Unit tests for `which` wrapper (mocked)
    - [ ] Unit tests for version parsing
    - [ ] Integration test: Detect `git` (should always be present)
    - [ ] Integration test: Detect fake tool (should return missing)
