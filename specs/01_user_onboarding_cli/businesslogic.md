# Logic: 01_user_onboarding_cli

## Core Logic

### 1. Initialization
- **Config Loading**: Load global settings from `~/.infinite-loop/config.toml` (API keys, default profiles).
- **Args Parsing**: Use `clap` to handle command line arguments.
  - Flags: `--goal "..."`, `--profile "..."`, `--headless`, `--budget 10`.
- **Mode Selection**: 
  - If args present -> Headless Mode (Start immediately).
  - If no args -> TUI Mode (Launch Wizard).

### 2. Wizard Controller
- **State Management**: Maintain a temporary `WizardState` struct getting populated step-by-step.
- **Validation**: Ensure Goal is not empty, Budget is positive, selected Profile exists.
- **Profile Loader**: Read available profiles from `assets/profiles/`.

### 3. Mission Bootstrap
- **Context Creation**: Create the initial `MissionContext`.
- **Event Emission**:
  - Emit `SystemInfoEvent` (OS, Resources).
  - Emit `MissionStartedEvent` (Goal, ID, Timestamp).
- **Worker Instantiation**: based on proper `WorkerTeamProfile`, spawn the initial set of `Worker` actors.

## Technical Components
- `clap`: For CLI parsing.
- `ratatui`: For the TUI Wizard.
- `config`: For loading TOML configuration.
- `uuid`: For generating unique Mission IDs.

## Data Flow
User Input -> CLI/TUI Parser -> WizardState -> Validator -> MissionConfig -> EventBus -> Engine Start