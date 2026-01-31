# Progress: 01_user_onboarding_cli

## Checklist

### Initialization
- [x] Set up `clap` for CLI argument parsing.
- [x] Implement `Config` loader for API keys and defaults. ('clap' used in main.rs, 'LoopConfig' in lib.rs)

### UI Implementation
- [x] Create `WelcomeScreen` component in Ratatui. (Implemented in `AppMode::MainMenu`)
- [x] Create `WizardStep` trait/enum for multi-step navigation. (Implemented in `ifcl-core/wizard.rs`)
- [x] Implement "Goal Input" step. (Implemented in `AppMode::Setup`)
- [x] Implement "Team Selection" step (mocked data initially). (Implemented)
- [x] Implement "Budget & Constraints" step. (Implemented)

### Logic & Wiring
- [x] Implement `WizardState` to collect inputs. (`SetupWizard` struct)
- [x] Implement `start_mission(config)` function. (Simulated in main.rs `tokio::spawn` loop)
- [x] Define and emit `MissionStarted` event. (`LoopStarted` and `MissionCreated` events exist)

### Tests
- [x] Unit test: CLI arg parsing.
- [x] Unit test: Config loading defaults.
- [x] Test: `MissionConfig` validation logic. (`wizard.rs` tests exist)