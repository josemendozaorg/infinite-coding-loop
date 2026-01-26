# Roadmap: Fundamental Refactoring (DASS Implementation)

This document serves as the master plan for refactoring the Infinite Loop repository to align with the **Deterministic Autonomous Software Synthesis (DASS)** framework.

**Goal**: Transform the codebase by building a **Parallel Stack** ("Software Factory") with strict Gates, Primitives, and Neuro-Symbolic verification, leaving the legacy simulation code untouched until migration.

## Phase 0: The Parallel Stack Strategy
*Objective: Create the clean slate environments.*

- [ ] **Create New Crates**
    - [ ] Initialize `crates/dass-engine` (The Logic Core).
    - [ ] Initialize `crates/dass-tui` (The Frontend).
    - [ ] Update `Cargo.toml` workspace members.

## Phase 1: Structural Foundations (The Taxonomy)
*Objective: Establish the directory structure and static artifacts defined in the Constitution.*

- [x] **Initialize Constitution**
    - [x] Create `.constitution/` directory.
    - [x] Create `Constitution.md` (Core axioms).
    - [x] Create `StyleGuide.md` (Rust/Agent standards).
    - [x] Create `SafetyPolicy.md` (Operational limits).
- [x] **Restructure Directories**
    - [x] Create `product/` (for Atomic Requirements).
    - [x] Create `verification/` (for PBT benchmarks/proofs).
    - [x] Rename/Move `spec/features/` to `specs/` (if aligning strictly) or strictly enforce `specs/` structure.
- [ ] **Migrate Existing Specs**
    - [ ] Audit `specs/` against the new 4-part Spec Standard (UI, Logic, Data, Test Plan).
    - [ ] Mark non-compliant specs as "Legacy" or "Draft".

## Phase 2: Core Primitives Implementation (Rust)
*Objective: Define the data structures that represent the "Verifiable Atomic Units" in `ifcl-core`.*

- [x] **Define Requirement Primitive**
    - [x] Create `Requirement` struct (ID, UserStory, AcceptanceCriteria).
    - [x] Implement YAML parser for `product/requirements.yaml`.
- [x] **Define Spec Primitive**
    - [x] Create `FeatureSpec` struct.
    - [x] Implement Markdown parser to extract UI, Logic, Data from files.
- [x] **Define Plan Primitive**
    - [x] Refactor existing `Plan` struct to fully support the DAG and atomic steps defined in DASS.
- [x] **Implement "Clover" Traits**
    - [x] Define traits for `ConsistencyCheck` (matching Code to Spec).

## Phase 3: The Enforcement Engine (The Gates)
*Objective: Implement the deterministic validators that block invalid state transitions.*

- [x] **Gate 1: Ambiguity Checker** (Product Manager)
    - [x] Build a validator that checks if a Requirement has a verifiable Oracle.
- [x] **Gate 2: Spec Consistency Checker** (Architect)
    - [x] Build a validator that ensures Spec covers all Requirements.
    - [x] Implement `check_links` (no dead wiki-links).
- [x] **Gate 3: Safe Planner** (Planner)
    - [x] Implement `DependencyCheck` (resources exist).
- [x] **Gate 4: Neuro-Symbolic Verifier** (QA)
    - [x] Integrate a PBT library (e.g., `proptest` for Rust) into the pipeline.
    - [x] Create a "Verifier" worker that runs these tests autonomously.

## Phase 4: Agent Integration & SOPs
*Objective: Update the AI Agents to respect the new Gates.*

- [x] **Define Agent Interfaces**
    - [x] Create `Agent` trait.
    - [x] Create `AiCliClient` trait.
- [x] **Implement Product Manager Agent**
    - [x] Logic Loop: User Request -> Requirements -> Ambiguity Gate -> Refine.
- [x] **Implement Architect Agent**
    - [x] Logic Loop: Requirements -> Specs -> Consistency Gate -> Refine.
- [x] **Implement Engineer Agent** (Planner)
    - [x] Logic Loop: Specs -> Plan -> Safety Gate -> Refine.

## Phase 5: Self-Hosting (The Infinite Loop)
*Objective: The system uses DASS to build DASS.*

- [x] **Create Factory Dashboard (TUI)**
    - [x] Build a `ratatui` interface visualizing the Pipeline (Requirements -> Spec -> Plan -> Code).
    - [x] Integrate Agent SOPs into the TUI event loop.
