# Logic: 12_main_menu_interactive

## User Logic
- Select which "Game" (Loop session) to enter.
- Create new Loop profiles.
- Browse the Worker Marketplace.

## Technical Logic
- Navigation state machine for the TUI (Home -> Setup -> Active Loop).
- File system enumeration of `.ifcl` session files.

## Implementation Strategy
1. Refactor `main.rs` to support `Screen` variants.
2. Implement `Home` screen with menu choices.
3. Connect `New Game` to the existing Loop initialization logic.
