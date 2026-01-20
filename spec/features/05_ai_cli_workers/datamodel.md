# Data Model: 05_ai_cli_workers

## Entities
### `CliExecutor`
- `binary_path: String`
- `args_template: String`
- `timeout_ms: u64`

### `AiResponse` event
- `worker_id: String`
- `stdout: String`
- `stderr: String`
- `exit_code: i32`
