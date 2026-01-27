# Data Model: 02_live_activity_feed

## Structs

### LogEntry
```rust
struct LogEntry {
    timestamp: DateTime<Utc>,
    level: LogLevel, // Info, Warn, Error, Success
    source_worker: String, // "Planner-1"
    message: String,
    payload_snapshot: Option<String>, // JSON string or trimmed content
}
```

## Enums
```rust
enum LogLevel {
    Info,
    Warn,
    Error,
    Success,
    Thinking, // For chain-of-thought updates
}
```