# Logic: 13_setup_wizard_ui

## User Logic
- Guided flow to creating a new Loop.
- Steps: Define Goal -> Select Stack -> Choose Team -> Set Budget.

## Technical Logic
- Multi-step form state management.
- Validation logic between steps (e.g., Team cannot be empty).
- Final output: `LoopProfile` struct.

## Implementation Strategy
1. Define `WizardStep` enum.
2. Implement TUI Screen overlay for the wizard.
3. Logic to reset inputs on back navigation.
