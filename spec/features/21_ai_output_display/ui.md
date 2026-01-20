# UI: 21_ai_output_display

## Visual Components

### 1. Markdown Renderer
- **Bold/Italic**: Rendered with terminal formatting.
- **Headers**: Colored and bold.
- **Lists**: Bullet points.

### 2. Code Block Renderer
- **Syntax Highlighting**: Use `syntect` to colorize code blocks (Rust, Python, etc.).
- **Background**: Different background color for code blocks to distinguish from text.
- **Scroll**: Horizontal scroll for long lines.

### 3. Artifact Viewer
- View generated files with syntax highlighting.
