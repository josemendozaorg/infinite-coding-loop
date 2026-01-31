# Logic: 37_marketplace_sharing

## Core Logic

### 1. Export
- **Package**: Bundle `WorkerProfile` (JSON) + `Prompts` (Markdown) + `Icon` (ASCII).
- **Format**: `.zip` or simple directory copy to `marketplace/`.

### 2. Import
- **Browser**: TUI file picker for `marketplace/` directory.
- **Validation**: Check schema version compatibility.

## Data Flow
WorkerProfile -> Exporter -> FileSystem
