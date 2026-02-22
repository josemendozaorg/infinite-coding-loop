# 0001: Monorepo Layout: Apps vs Packages

## Status
Accepted

## Context
As the `infinite-coding-loop` repository grows to support multiple applications across different languages (Rust, TypeScript), the initial flat directory structure (where every project, application, and utility shared the root directory) proved difficult to scale and navigate. 

We initially migrated away from a `crates/`-only structure to a Service-Oriented (flat) directory structure, bringing UI clients and engine libraries to the same top level. However, this didn't convey the intent or role of each module. For instance, `dass-engine` (a core library), `ontology-visualizer` (a deployable web application), and `ontology-schema` (NPM types) were all peers. 

## Decision
We have decided to adopt the **Apps vs. Packages** monorepo pattern. This is an industry-standard practice championed by build systems like Nx, Turborepo, and Lerna, and is natively supported by generic toolchains like Cargo Workspaces and NPM Workspaces.

The repository is now structurally partitioned by the **role** a module plays in the system:

1.  **`apps/`**: Contains deployable, executable endpoints (e.g., CLI tools, web frontends, desktop clients). These modules *consume* packages but are never consumed as dependencies by other projects in the monorepo.
2.  **`packages/`**: Contains shared libraries, core business logic, components, and utilities. These are non-deployable on their own.
3.  **`tests/`**: Contains standalone, cross-boundary testing suites (like end-to-end integration tests) that validate the orchestration between apps and packages.
4.  **`ontologies/`**: (Domain Specific) Contains the core schema data and source-of-truth JSON/TTL files that power the system's reasoning engine.

## Consequences
*   **Clearer Onboarding**: New engineers can immediately identify which folders contain runnable features (`apps/`) and which contain shared utilities (`packages/`).
*   **Simpler Workspace Configurations**: Both `Cargo.toml` and `package.json` can now use clean wildcard globs (`"apps/*"`, `"packages/*"`) instead of explicitly maintaining a list of every single folder.
*   **Pathing Complexity**: Moving folders deeper introduces slightly more complex relative pathing (e.g., `../../../`) for internal static file inclusions, which requires careful maintenance. 
*   **Ecosystem Agnostic**: Language boundaries do not dictate structure. A Rust CLI and a TypeScript Web Client can both peacefully coexist inside `apps/`.
