# Logic: 19_token_cost_tracking

## User Logic
- Monitor real-world cost of AI usage.
- Set hard limits to prevent runaway bills.

## Technical Logic
- Track input/output tokens in `AiWorker`.
- Multiply by `cost_per_token` from `WorkerProfile`.
- Halt loop if total cost > budget.

## Implementation Strategy
1. Update `Event` payload to include token usage stats.
2. Aggregator in `Bank` or `MetricsCollector` to sum costs.
