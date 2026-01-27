# Data Model: 01_user_onboarding_cli

## Structs

### MissionConfig
```rust
struct MissionConfig {
    id: Uuid,
    goal: String,
    budget_cap: Decimal,
    max_duration: Option<Duration>,
    stopping_criteria: String,
    worker_profile_id: String,
    created_at: DateTime<Utc>,
}
```

### WorkerProfile
```rust
struct WorkerProfile {
    name: String,
    description: String,
    workers: Vec<WorkerDefinition>,
}

struct WorkerDefinition {
    role: String, // e.g., "Planner", "FullStackDev"
    engine: String, // "Claude-3-Opus", "GPT-4o"
    count: u8,
}
```

## Events

### `MissionStarted`
- **Source**: `UserOnboarding`
- **Payload**: `MissionConfig`
- **Purpose**: Signals the start of the loop. Listeners (Activity Feed, Persistence) will react.

## Persistence
- **Config Storage**: `config.toml` for user prefs.
- **Mission Storage**: An initial entry in the `missions` table/file (handled by `09_persistent_history`, but triggered here).