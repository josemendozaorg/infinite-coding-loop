# AI Agents in DASS

This document outlines the responsibilities and tool-usage requirements for AI Agents participating in the DASS orchestration loop.

## Persistence Responsibility

In DASS, the **AI Agent** is solely responsible for persisting artifacts to disk when a `work_dir` is provided. The Rust Engine (Orchestrator) manages the graph and ensures artifacts match their schemas, but it does **not** perform file writes for the agents.

### Tool Usage
- Agents must use their available tools (e.g., `write_file`, `create_directory`) to save JSON artifacts and source code.
- Persistence should target the specified `work_dir`.

### Output Constraints
- Agents must provide exactly **one** JSON code block in their final response.
- This output serves as the "Semantic Result" that the engine parses and validates against the ontology.
- Agents must avoid preambles, summaries, or multiple markdown blocks to prevent parsing errors in the engine.

## Agent Architecture
- **Role-Based**: Each agent is initialized with a specific `AgentRole` (e.g., `Architect`, `Engineer`, `ProductManager`).
- **Context-Aware**: Agents receive the full JSON context of all previously generated artifacts in the current session.
- **Strictly Scoped**: Agents should only perform the relationship action assigned to them (e.g., `Architect creates DesignSpec`).
