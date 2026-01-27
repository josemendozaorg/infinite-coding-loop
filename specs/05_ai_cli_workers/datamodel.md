# Data Model: 05_ai_cli_workers

## Enums

### WorkerState
```rust
enum WorkerState {
    Booting,
    Idle,
    Processing(TaskID),
    BackingOff(Duration),
    Error(String),
}
```

### LLMProvider
```rust
enum LLMProvider {
    Anthropic,
    Google,
    OpenAI,
    Ollama,
}
```

## Structs

### LLMRequest
```rust
struct LLMRequest {
    system_prompt: String,
    user_message: String,
    temperature: f32,
    max_tokens: u32,
}
```
