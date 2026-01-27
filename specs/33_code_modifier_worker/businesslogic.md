# Logic: 33_code_modifier_worker

## Core Logic

### 1. Editing Strategies
- **Whole File Replace**: Safest for small files.
- **Search & Replace**: For specific blocks.
- **Unified Diff**: Apply standard diffs.

### 2. Verification
- **Post-Edit Check**: Run syntax check (parsing) to ensure file is not broken before committing.

## Data Flow
CodeTask -> CodeModifier -> FileSystem -> Validation -> Result
