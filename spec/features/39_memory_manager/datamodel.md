# Data Model: 39_memory_manager

## Structs

### MemoryNode
```rust
struct MemoryNode {
    id: Uuid,
    content: String,
    embedding: Vec<f32>,
    tags: Vec<String>,
    created_at: DateTime<Utc>,
    importance: f32, // 0.0 - 1.0 (for decay)
}
```
