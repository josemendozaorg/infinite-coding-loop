# F48 - CLI Environment Verifier - Data Model

## Structures

### `ToolCheckStatus`
Represents the result of checking a single tool.

```rust
pub struct ToolCheckStatus {
    pub name: String,
    pub category: ToolCategory,
    pub is_installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub error: Option<String>, // if check failed for other reasons
}

pub enum ToolCategory {
    Core,       // git, cargo
    AI,         // gemini, copilot
    Container,  // docker
    Utility,    // jq, rg
}
```

### `EnvironmentReport`
Collection of all checks.

```rust
pub struct EnvironmentReport {
    pub timestamp: DateTime<Utc>,
    pub checks: Vec<ToolCheckStatus>,
    pub missing_count: usize,
}
```

## Events

- `EnvironmentCheckStarted`
- `EnvironmentCheckCompleted { report: EnvironmentReport }`
- `ToolInstallMissionStarted`
- `ToolInstallMissionCompleted`
