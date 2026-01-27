# The Constitution of Infinite Loop

**Version**: 1.0.0
**Status**: Active

This document establishes the immutable Axioms and Laws that govern the **Deterministic Autonomous Software Synthesis (DASS)** process. All Agents and Humans interacting with this repository must adhere to these principles.

## I. The Axiom of Determinism
> "Code is not creative writing; it is a logical proof."

1.  **Strict Causality**: Every line of code must be traceable to a specific **Atomic Requirement**.
2.  **No Magic**: The output of any step must be reproducible given the same input and context.
3.  **Gate-Keeping**: No artifact shall transition to the next state without passing its deterministic **Gate** (Validation).

## II. The Axiom of Atomicity
> "Divide until it cannot be divided."

1.  **Atomic Requirements**: A Requirement must describe exactly one logic path or constraint.
2.  **Atomic Steps**: A Plan Step must perform exactly one state change.
3.  **Atomic Commits**: A Git Commit must address exactly one Plan Step.

## III. The Axiom of Verification
> "Trust but Verify is insufficient; Verify then Trust."

1.  **Code-as-Proof**: Code is considered invalid until accompanied by a passing **Test Oracle** (Code + Proof = Valid Artifact).
2.  **Test First**: The Test Oracle (Verification Plan) must be defined before the Implementation.
3.  **Property-Based Integrity**: Verification must prioritize Invariants (Property-Based Testing) over Examples (Unit Testing) whenever possible.

## IV. The Axiom of Isolation
> "A clean environment yields clean results."

1.  **Sandboxing**: All Agent execution must occur within a sandboxed environment (Docker/Wasm).
2.  **Context Hygiene**: Agents must explicit "Page In" and "Page Out" context to prevent hallucinations driven by stale memory.
