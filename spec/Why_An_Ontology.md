# Why Use an Ontology for the Autonomous Coding Loop?

This document outlines the architectural reasoning behind using a formal Ontology (Schema) to drive the Infinite Coding Loop, rather than relying on hardcoded logic or free-form LLM execution.

## 1. Separation of Policy from Mechanism

The most significant advantage is the decoupling of the *rules* of software development from the *engine* that executes them.

*   **The Engine (Mechanism):** The Rust code (`dass-engine`) is agnostic to the domain. It focuses on graph traversal, CLI tool invocation, and template rendering. It does not need to know what a "Feature" or a "Risk" is implies.
*   **The Ontology (Policy):** The schema (`metamodel.schema.json`) defines the strict rules of the SDLC (e.g., "An Engineer must create a Plan before writing Source Code").

**Benefit:** We can modify the software development process—such as adding a mandatory "Security Review" step or changing the hierarchy of requirements—by simply editing the JSON schema. This requires no recompilation of the engine core.

## 2. Contextual Grounding for AI Agents

One of the primary challenges in Agentic Engineering is "Context Stuffing"—providing too much or irrelevant information for the task at hand. The ontology solves this by providing precise semantic boundaries.

*   **Semantic Context:** The engine uses the ontology's defined relationships (edges) to look up specific prompt templates (e.g., `ontology/prompts/Engineer_creates_Plan.md`).
*   **Focused Execution:** When an agent acts, it is not just "writing code"; it is acting as a specific Node (e.g., `Engineer`) fulfilling a specific Relation (e.g., `creates`) for a specific Target (e.g., `Plan`).

This drastically reduces hallucinations because the agent is constrained to the specific definition of that relationship.

## 3. Determinism in a Probabilistic System

Large Language Models (LLMs) are inherently probabilistic and non-deterministic. However, functional Software Engineering requires a deterministic outcome (e.g., compiling code, passing tests).

*   **State Machine Skeleton:** The Ontology acts as the deterministic skeleton of the application.
*   **Constrained Generation:** While the *content* of a "Plan" node is generated probabilistically by the AI, the *existence* of that node and its required connection to a "SourceFile" node is enforced deterministically by the graph.

This ensures the system does not degrade into chaos; it forces the probabilistic intelligence to fill specific, validated boxes within a strict engineering execution graph.

## 4. Comparison to Alternatives

| Approach | Description | Drawback |
| :--- | :--- | :--- |
| **Hardcoded Logic** | Writing `if role == "Engineer" { create_plan() }` directly in Rust. | Brittle, hard to change, tightly coupled. |
| **Free-form Chat** | An agent simply "deciding" what to do next based on conversation history. | Prone to loops, skipped steps (e.g., skipping tests), and inconsistency. |
| **Ontology/Graph** | **Current Approach.** A dynamic system flexible enough for AI, but with the safety guarantees of a compiler. | **Optimal balance of flexibility and control.** |

## 5. Conclusion

We use the ontology as a **Domain-Specific Language (DSL) for Agency**. It serves as the "Brain" structure that the orchestrator traverses, ensuring that the autonomous coding loop adheres to strict engineering principles while leveraging the creative power of AI.
