# Logic: 15_settings_configuration

## User Logic
- Modify global app settings: Keybindings, Theme, API Keys.
- Persist changes to `config.toml`.

## Technical Logic
- `ConfigStore` struct using `toml` serialization.
- Dynamic keybinding remapping during runtime.

## Implementation Strategy
1. Define `AppConfig` struct.
2. Implement load/save logic.
3. Wire `AppState` to use values from `AppConfig`.
