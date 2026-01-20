# Logic: 19_token_cost_tracking

## Core Logic

### 1. Token Counter
- **Library**: `tiktoken-rs`.
- **Calculation**: Intercept every LLM request/response and count tokens.

### 2. Cost Estimator
- **Pricing Model**: Configurable per model (e.g., GPT-4o input/output costs).
- **Ledger**: Update total spend. If `spend > budget`, pause/stop mission.

## Data Flow
LLMClient -> TokenCounter -> CostTracker -> EventBus (BudgetUpdate)
