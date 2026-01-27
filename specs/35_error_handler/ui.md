# UI Spec: 35_error_handler

## User Interface Elements

### 1. Log Output
- **Retry Notices**: "⚠️ Task Failed. Retrying (1/3)..." (Yellow/Warning color).
- **Escalation Notices**: "❌ Retries Exhausted. Escalating to Planner..." (Red/Error color).
- **Recovery Success**: "✅ Retry Successful!" (Green).

### 2. Task List (TUI)
- Failed tasks that are retrying should show status `Running` (or a specific `Retrying` if added, but `Running` is fine for now) or `Pending`.
- If status is `Pending` and `retry_count > 0`, maybe append "(Retry N)" to the name or status display?
- For now, we stick to standard statuses.

## Interactions
- User can see the retry loop happening in the "Activity Feed".
- User can intervene via "God Mode" (F08) if the loop gets stuck (already implemented).
