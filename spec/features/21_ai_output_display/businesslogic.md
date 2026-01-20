# Logic: 21_ai_output_display

## Core Logic

### 1. Markdown Parsing
- **Library**: `pulldown-cmark`.
- **Process**: Convert Markdown stream -> Events -> Ratatui Span/Text.

### 2. Syntax Highlighting
- **Library**: `syntect`.
- **Caching**: Cache theme/syntax sets for performance.
- **Streaming**: Handle streaming chunks (incomplete code blocks) gracefully without breaking formatting (e.g., look for backticks).

## Data Flow
LLM Stream -> Buffer -> Markdown Parser -> Highlighter -> TUI Widget
