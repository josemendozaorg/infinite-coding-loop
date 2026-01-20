# Logic: 40_learning_manager

## Core Logic

### 1. Post-Mortem Analysis
- **Trigger**: Mission End or Major Error.
- **Input**: Activity Log + Outcome.
- **Analysis**: Identify patterns (e.g., "30% of compilation errors were due to missing imports").

### 2. Optimization
- **Suggestion**: "Add 'check imports' to Coder System Prompt".
- **Action**: Update `assets/prompts/coder.md` or `config.toml`.

## Data Flow
MissionHistory -> LearningManager -> Insights -> ConfigUpdate
