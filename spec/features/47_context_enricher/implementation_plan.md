# Feature 47: Context Enricher

## Goal Description
Enhance the Planner's decision-making by automatically gathering relevant workspace context.

## Proposed Changes

### ifcl-core
#### [NEW] [enricher.rs](file:///home/dev/repos/infinite-coding-loop/ifcl-core/src/enricher.rs)
- **Struct `ContextEnricher`**
- **Methods**:
    - `get_file_tree(path: &str, max_depth: usize) -> String`
    - `get_git_status(path: &str) -> String`
    - `collect(path: &str) -> String`

#### [MODIFY] [planner_worker.rs](file:///home/dev/repos/infinite-coding-loop/ifcl-core/src/planner_worker.rs)
- Update `execute` method to Call `ContextEnricher::collect(workspace_path)`.

#### [MODIFY] [lib.rs](file:///home/dev/repos/infinite-coding-loop/ifcl-core/src/lib.rs)
- Export `enricher`.

## Verification Plan
### Automated Tests
- Unit tests for `enricher.rs`.

### Manual Verification
- Run a mission (Headless) and check logs.
