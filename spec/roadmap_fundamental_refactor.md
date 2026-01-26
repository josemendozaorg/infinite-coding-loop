# Roadmap: Fundamental Refactoring (DASS Implementation)

This document serves as the master plan for refactoring the Infinite Loop repository to align with the **Deterministic Autonomous Software Synthesis (DASS)** framework.

**Goal**: Transform the codebase from a standard Rust project into a "Software Factory" with strict Gates, Primitives, and Neuro-Symbolic verification.

## Phase 1: Structural Foundations (The Taxonomy)
*Objective: Establish the directory structure and static artifacts defined in the Constitution.*

- [ ] **Initialize Constitution**
    - [ ] Create `.constitution/` directory.
    - [ ] Create `Constitution.md` (Core axioms).
    - [ ] Create `StyleGuide.md` (Rust/Agent standards).
    - [ ] Create `SafetyPolicy.md` (Operational limits).
- [ ] **Restructure Repository Directories**
    - [ ] Create `product/` (for Atomic Requirements).
    - [ ] Create `verification/` (for PBT benchmarks/proofs).
    - [ ] Rename/Move `spec/features/` to `specs/` (if aligning strictly) or strictly enforce `specs/` structure.
- [ ] **Migrate Existing Specs**
    - [ ] Audit `spec/features/` against the new 4-part Spec Standard (UI, Logic, Data, Test Plan).
    - [ ] Mark non-compliant specs as "Legacy" or "Draft".

## Phase 2: Core Primitives Implementation (Rust)
*Objective: Define the data structures that represent the "Verifiable Atomic Units" in `ifcl-core`.*

- [ ] **Define Requirement Primitive**
    - [ ] Create `Requirement` struct (ID, UserStory, AcceptanceCriteria).
    - [ ] Implement YAML parser for `product/requirements.yaml`.
- [ ] **Define Spec Primitive**
    - [ ] Create `FeatureSpec` struct.
    - [ ] Implement Markdown parser to extract UI, Logic, Data from files.
- [ ] **Define Plan Primitive**
    - [ ] Refactor existing `Plan` struct to fully support the DAG and atomic steps defined in DASS.
- [ ] **Implement "Clover" Traits**
    - [ ] Define traits for `ConsistencyCheck` (matching Code to Spec).

## Phase 3: The Enforcement Engine (The Gates)
*Objective: Implement the deterministic validators that block invalid state transitions.*

- [ ] **Gate 1: Ambiguity Checker** (Product Manager)
    - [ ] Build a validator that checks if a Requirement has a verifiable Oracle.
- [ ] **Gate 2: Spec Consistency Checker** (Architect)
    - [ ] Build a validator that ensures Spec covers all Requirements.
    - [ ] Implement `check_links` (no dead wiki-links).
- [ ] **Gate 3: Safe Planner** (Planner)
    - [ ] Implement `DependencyCheck` (resources exist).
- [ ] **Gate 4: Neuro-Symbolic Verifier** (QA)
    - [ ] Integrate a PBT library (e.g., `proptest` for Rust) into the pipeline.
    - [ ] Create a "Verifier" worker that runs these tests autonomously.

## Phase 4: Agent Integration & SOPs
*Objective: Update the AI Agents to respect the new Gates.*

- [ ] **Update Planner Agent**
    - [ ] Modify prompt to output strictly formatted Plans.
    - [ ] Feed Spec Primitives as context (not just raw text).
- [ ] **Update Coder Agent**
    - [ ] Enforce "Code-as-Proof" (comments/docstrings must match Spec).
- [ ] **Implement Feedback Loops**
    - [ ] Wire up the "Counter-Example" feedback (if Gate fails, feed error back to Agent).

## Phase 5: Self-Hosting (The Infinite Loop)
*Objective: The system uses DASS to build DASS.*

- [ ] **Bootstrapping**
    - [ ] Use the new process to add a new feature (e.g., "Web Interface").
    - [ ] Verify that the Loop correctly blocks invalid artifacts and guides the agent to repair them.
