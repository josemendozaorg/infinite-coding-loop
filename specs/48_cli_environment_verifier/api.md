# F48 - CLI Environment Verifier - API

## Public Interface

### `EnvironmentVerifier` (Worker or Service)

```rust
pub trait EnvironmentVerifier {
    /// Runs checks for all defined tools asynchronously
    async fn check_all(&self) -> EnvironmentReport;

    /// Checks a specific tool
    async fn check_tool(&self, tool_name: &str) -> ToolCheckStatus;
    
    /// Generates a plan/mission to install missing tools
    fn create_install_mission(&self, missing_tools: Vec<String>) -> Mission;
}
```

## Configuration

Tools to verify should be configurable via a `config/tools.toml` or defined in constant arrays, allowing for easy addition of new AI CLI tools.
