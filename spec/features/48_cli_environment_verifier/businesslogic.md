# F48 - CLI Environment Verifier - Business Logic

## 1. Core Verification Logic

The system needs to perform non-blocking checks for a list of essential and optional CLI tools.

### 1.1 Target Tools
The verifier should check for the availability of the following tools:
- **Version Control:** `git`, `gh` (GitHub CLI)
- **Languages/Runtimes:** `rustc`, `cargo`, `node`, `npm`, `python3`, `pip`
- **Containers:** `docker`, `docker-compose`
- **AI/Coding Assistants:**
    - `opencode`
    - `gemini`
    - `copilot`
    - `claude`
    - `codex`
    - `cursor`
    - `aider`
- **Utilities:** `jq`, `curl`, `wget`, `ripgrep` (rg), `fd`

### 1.2 Verification Process
- **Silent Check:** The check must NOT throw exceptions or halt execution if a tool is missing.
- **Output:** It should returns a status object for each tool:
    - `Installed`: Boolean
    - `Version`: String (parsed from `--version` output if possible)
    - `Path`: String (result of `which` or equivalent)
- **Timeout:** Checks should have a short timeout to avoid hanging the UI.

## 2. The "Mission Only" Installation

If tools are missing, the system does NOT auto-install them silently. Instead:
1. It aggregates the list of missing tools.
2. It generates a dynamic **Mission** (e.g., "Setup Dev Environment").
3. This Mission, if accepted by the user, will perform the installation steps (using system package managers or specific install scripts).

## 3. Integration with Startup
- This verification should run asynchronously on startup (or on demand via "Check Env" command).
- Results are cached in the `Context` or `GlobalState`.
