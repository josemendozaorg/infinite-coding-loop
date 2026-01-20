# Business Logic: 05_ai_cli_workers

## Goal
Enable the **Infinite Coding Loop** to invoke real shell commands for AI agents (`claude`, `opencode`, `gemini`, `copilot`).

## Execution Logic
1.  **Shell Context**: Each CLI worker runs in the project root.
2.  **Command Templates**:
    - **Claude**: `claude -p "{prompt}"`
    - **Gemini**: `gemini "{prompt}"`
    - **OpenCode**: `opencode run "{prompt}"`
    - **Copilot**: `gh copilot suggest -t shell "{prompt}"` (Placeholder status)
3.  **Wait Mode**: The loop must await the CLI process completion (async).
4.  **Buffer Capture**: Standard output is captured and emitted as an `AiResponse` event.
5.  **Error Handling**: Non-zero exit codes emit a `WorkerError` event.

## Integration
- **Worker Registry**: Mapping `WorkerRole` to a specific CLI tool.
- **Task Linkage**: When a `Task` is set to `Running`, the assigned worker triggers its CLI template.
