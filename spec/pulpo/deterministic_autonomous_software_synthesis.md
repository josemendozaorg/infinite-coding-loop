# Deterministic Autonomous Software Synthesis (DASS) Framework

This document serves as the **Rigorous Technical Reference** for the Infinite Loop application's engineering process. It is derived from the research on correct-by-construction engineering and expands on the simplified `infinite_loop_software_engineering_product_development.md` guide.

## 1. Core Philosophy: Deterministic Synthesis

We reject the notion of the LLM as a creative author, repositioning it as a semantic translator within a strictly governed verification loop.

-   **Verifiable Atomic Units**: Every artifact is atomic, unambiguous, and verifiable.
-   **Neuro-Symbolic Architecture**: Neural networks provide intuition; Symbolic solvers/compilers provide guarantees.
-   **Code-as-Proof**: Code is strictly partially-valid until accompanied by a formal or empirical proof of correctness.

## 2. The Taxonomy of Verifiable Atomic Units

The lifecycle is transforming "Primitives" through strict gates.

### 2.1. The Constitution (The Law)
*   **Nature**: Immutable, Global Constraint.
*   **Artifacts**: `Constitution.md` (Ethical bounds), `StyleGuide.md` (Coding standards), `SafetyPolicy.md`.
*   **Enforcement**: Constrained Decoding (Grammar/FSM) in the inference engine prevents invalid syntax generation.

### 2.2. The Atomic Requirement (The Truth)
*   **Nature**: Singular, Independent, Verifiable.
*   **Source**: User Goal -> Product Manager Agent -> Atomization.
*   **Constraint**: Must imply a Test Oracle (True/False).
*   **Artifact**: `product/requirements.yaml` (List of Atomic Requirements).
    *   *Example*: "The system shall validate user credentials against the secure store" (Atomic).
    *   *Counter-Example*: "Validate and update dashboard" (Compound/Ambiguous).

### 2.3. The Specification (The Contract)
*   **Nature**: Executable, Formal.
*   **Source**: Atomic Requirements -> Architect Agent.
*   **Constraint**: **Clover Consistency** (Code <-> Doc <-> Spec).
*   **Artifact Suite**:
    *   **UI Spec**: Component hierarchy & interaction state machine.
    *   **Data Spec**: Schema definitions (SQL/Structs).
    *   **Logic Spec**: Formal invariants (Pre-conditions, Post-conditions).
    *   **Test Plan**: **Property-Based Testing (PBT)** definitions (Invariants, not just examples).
*   **Tools**: TLA+, Dafny, or strict Markdown templates.

### 2.4. The Plan (The Blueprint)
*   **Nature**: Imperative, Directed Acyclic Graph (DAG).
*   **Source**: Spec -> Planner Agent.
*   **Constraint**: Topological sort validity, Resource existence.
*   **Artifact**: `implementation_plan.json`.

### 2.5. The Implementation (The Proof)
*   **Nature**: Deterministic, Compilable.
*   **Source**: Plan -> Engineer Agent.
*   **Constraint**: Must pass the **Neuro-Symbolic Verifier**.
*   **Artifact**: Source Code + Inline Formal Annotations (Docstrings/Asserts).

### 2.6. The Verification (The Oracle)
*   **Nature**: Boolean, Comprehensive.
*   **Source**: QA Agent + System.
*   **Mechanisms**:
    *   **Symbolic Execution**: Mathematical proof of logic (where possible).
    *   **Property-Based Testing (PBT)**: Fuzzing for edge cases (Hypothesis/QuickCheck).
    *   **Regression**: Existing test suite.

## 3. The Process: The Neuro-Symbolic Loop

The "Loop" is a state machine enforcing **Standard Operating Procedures (SOPs)**.

| State | Artifact Input | Agent Role | Gate (Deterministic Validator) | Failure Action |
| :--- | :--- | :--- | :--- | :--- |
| **1. Atomization** | User Prompt | Product Manager | **Ambiguity Check**: Are requirements atomic & verifiable? | Clarify with User |
| **2. Design** | Requirements | Architect | **Consistency Check**: Do Specs cover all Requirements? | Refine Spec |
| **3. Plan** | Spec | Planner | **Dependency Check**: Are resources available? | Refine Plan |
| **4. Synthesis** | Plan + Spec | Engineer | **Compiler/Linter**: Does it build? | Fix Syntax |
| **5. Verify** | Implementation | QA Engineer | **PBT Oracle**: Do invariants hold for N inputs? | **Shrink Input** -> Fix Logic |
| **6. Merge** | Verified Code | System | **Integration Check**: No regression? | Revert |

## 4. The Quality & Feedback Model

Quality is measured continuously, not post-hoc.

-   **Clover Loop**: Enforces consistency between Docstrings, Code, and Specs. If they diverge, the artifact is rejected.
-   **Counter-Example Driven Refinement**: When verification fails, the system provides a specific failing input (e.g., `input=[-1]`) to the agent.
-   **Infinite Context (MemGPT)**: The agent explicitly "pages in" relevant historical errors (e.g., "Recall the error trace from last failed deployment") to prevent regression loops.

## 5. Directory Structure Implication

This structure supports the rigorous separation of concerns:

```
/
├── .constitution/    # The immutable laws
├── product/          # Atomic Requirements (YAML)
├── specs/            # Verifiable Specifications (The "Prompt" for code)
│   ├── feature_x/
│   │   ├── ui.md
│   │   ├── logic.md
│   │   ├── data.md
│   │   └── invariants.pbt # Property definitions
├── src/              # Implementation
├── verification/     # PBT definitions and Proofs
└── memory/           # Agent long-term memory (MemGPT store)
```
