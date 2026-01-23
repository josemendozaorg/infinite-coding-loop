# Browser Testing with ttyd

This document describes how the Infinite Coding Loop TUI is tested and verified within a browser environment.

## Overview

Since the application is a Terminal User Interface (TUI), standard browser testing tools cannot interact with it directly. To bridge this gap, we use **ttyd** to serve the terminal session over HTTP.

## Local Testing Setup

A pre-compiled `ttyd` binary is included in the repository root for convenience. This eliminates the need to install `ttyd` separately on your system.

### Using the Local Binary
```bash
# From the repository root
./ttyd -p 7682 cargo run --bin tui
```

### Binary Details
- **Location**: `/ttyd` (repository root)
- **Type**: Statically linked ELF 64-bit executable (x86-64 Linux)
- **Size**: ~1.3 MB

> **Note**: If you're on a different architecture or OS, you may need to download the appropriate `ttyd` binary from the [official releases](https://github.com/tsl0922/ttyd/releases).

---

## Workflow

### 1. Start ttyd
The TUI is launched through `ttyd`, mapping the terminal session to a web port:
```bash
# Using system-installed ttyd
ttyd -p 7682 cargo run --bin tui

# Or using the local binary
./ttyd -p 7682 cargo run --bin tui
```

### 2. Browser Access
The TUI is then accessible at `http://localhost:7682`. The browser renders the terminal using Xterm.js, providing a full-featured terminal emulator in the web page.

### 3. Automated Interaction
The AI agent uses a browser subagent to:
- **Navigate**: Open the `ttyd` URL.
- **Inspect**: Observe the visual state of the TUI (layout, colors, widgets).
- **Control**: Send keyboard and mouse events to the terminal.
- **Verify**: Confirm that missions, progress bars, and logs appear as expected.

## Advantages
- **Visual Verification**: Allows the agent to verify complex TUI layouts (like the Progress Bar or Mental Map).
- **Interactive Demos**: Facilitates capturing walkthrough recordings and screenshots directly from the terminal.
- **Remote Testing**: Enables testing the TUI on remote dev machines without direct SSH/Terminal access.
