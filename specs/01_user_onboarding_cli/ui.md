# UI: 01_user_onboarding_cli

## Visual Components

### 1. Welcome Screen
- **ASCII Art Logo**: Display "INFINITE CODING LOOP" in a stylized ASCII font.
- **Tagline**: "The Autonomous Self-Evolving Software Engine"
- **Status Bar**: "System: ONLINE | Profile: Default | Mode: CLI/TUI"

### 2. Setup Wizard (Interactive Mode)
If no CLI arguments are provided, launch the TUI Wizard.

#### Step 1: Mission Definition
- **Prompt**: "What is your high-level goal?"
- **Input Field**: Multiline text area.
- **Example Placeholder**: "Build a personal blog with Rust and HTMX..."

#### Step 2: Worker Team Selection
- **List View**: Select from available "Worker Group Profiles".
  - [x] Default Team (1 Planner, 1 Coder, 1 Reviewer)
  - [ ] Research Heavy (3 Researchers, 1 Writer)
  - [ ] Custom...
- **Details Panel**: Shows the workers in the highlighted profile.

#### Step 3: Constraints & Budget
- **Budget Input**: "Max Spend ($):" (Default: $10.00)
- **Time Limit**: "Max Duration:" (e.g., "Forever", "24h")
- **Stopping Condition**: Optional natural language description.

### 3. CLI Output (Headless Mode)
- Standard stdout/stderr logging for CI/CD usage.
- Progress bars for initialization steps.

## User Interactions
- **Navigation**: Arrow keys to move between fields/steps.
- **Confirmation**: Enter to confirm, Esc to go back/exit.
- **Shortcuts**: `?` for help, `Ctrl+C` to abort.