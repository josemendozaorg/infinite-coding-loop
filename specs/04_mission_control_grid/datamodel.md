# Data Model: 04_mission_control_grid

## Structs

### DashboardState
```rust
struct DashboardState {
    current_layout: LayoutMode,
    focused_pane: PaneId,
    mission_stats: MissionStatsSnapshot,
}
```

### MissionStatsSnapshot
```rust
struct MissionStatsSnapshot {
    completion_percentage: f32,
    budget_used: Decimal,
    active_workers_count: usize,
    uptime: Duration,
}
```