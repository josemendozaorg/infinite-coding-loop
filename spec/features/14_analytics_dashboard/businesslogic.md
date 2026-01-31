# Logic: 14_analytics_dashboard

## User Logic
- View real-time resource usage (CPU/RAM).
- Track mission success vs. failure rates.
- Monitor token expenditure and efficiency.

## Technical Logic
- Periodic collection of system metrics using `sysinfo` or similar.
- State projection that aggregates `RewardEarned` and `MissionCreated` events into a `MetricsRegistry`.

## Implementation Strategy
1. Add `sysinfo` dependency.
2. Implement `MetricsCollector` loop.
3. Wire metrics to a read-only State Projection.
