# Feature 24: System Logging (Real-time Console)

## Goal Description
Ensure operational visibility by routing all system logs through the Event Bus.

## Proposed Changes

### tui
#### [MODIFY] [main.rs](file:///home/dev/repos/infinite-coding-loop/tui/src/main.rs)
- **Loop Runner**:
    - Replace `println!("✅ Task Success...")` with `Event::Log(INFO)`.
    - Replace `println!("❌ Task Execution Failed...")` with `Event::Log(ERROR)`.
    - Replace `println!("⚠️ Retrying...")` with `Event::Log(WARN)`.
    - Replace `println!("🧠 Delegating...")` with `Event::Log(INFO)`.
    - Replace `println!("❌ Retries Exhausted...")` with `Event::Log(ERROR)`.

## Verification Plan
### Manual Verification
- Run `tui` in headless mode.
- Verify that the standard output now shows the JSON-serialized Log events.
