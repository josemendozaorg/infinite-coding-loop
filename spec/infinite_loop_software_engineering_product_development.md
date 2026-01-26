# Infinite Loop Software Engineering Product Development

This document describes the **Deterministic Software Process** we will use to build the application.
Think of this not as "writing code," but as running a **Software Factory**. We define inputs, machines, and quality checks to ensure the final product (the feature) is correct every time.

## 1. The Core Idea

We don't just "ask AI to write code." We build a **pipeline** where AI does the work, but a strict process ensures the work is right.

**The Golden Rule**: No step moves forward until it passes a specific check (a "Gate").

## 2. The Building Blocks (Primitives)

Everything we build must fit into one of these boxes.

### 2.1. The Constitution (Rules of the Road)
*   **What is it?**: The unchangeable laws of our project.
*   **Examples**: "Code must be in Rust," "No unwraps used," "Tests must exist."
*   **Where it lives**: `.constitution/` folder.

### 2.2. The Atomic Requirement (One Specific Thing)
*   **What is it?**: A single, testable statement of what we want.
*   **Rule**: It must be small enough to say "Yes it works" or "No it doesn't."
*   **Bad Example**: "Make a secure login system." (Too big, vague).
*   **Good Example**: "Reject login if the password is less than 8 characters."
*   **Artifact**: `product/requirements.yaml`.

### 2.3. The Specification (The Blueprint)
*   **What is it?**: The detailed technical plan before we write any code.
*   **Components**:
    *   **UI**: What it looks like.
    *   **Data**: What the database tables look like.
    *   **Logic**: The rules (e.g., "If X happens, do Y").
    *   **Test Plan**: How we *will* prove it works.
*   **Artifact**: `specs/features/<feature_name>/`.

### 2.4. The Plan (The To-Do List)
*   **What is it?**: A step-by-step list of instructions for the coder.
*   **Rule**: Small steps. "Create file," "Add function," "Run test."
*   **Artifact**: `implementation_plan.json`.

### 2.5. The Implementation (The Code)
*   **What is it?**: The actual Rush/Python code.
*   **Rule**: It must compile and match the Spec.

### 2.6. The Verification (The Proof)
*   **What is it?**: The evidence that the code works.
*   **Types**:
    *   **Unit Tests**: Does this function work?
    *   **Property Tests**: Does it work for *weird* inputs (e.g., empty lists, negative numbers)?
    *   **Integration**: Does it work with the database?

## 3. The Process (The Factory Line)

We follow this loop for every single feature.

| Step | Who Does It? | Input | The Gate (Check) | If it Fails? |
| :--- | :--- | :--- | :--- | :--- |
| **1. Define** | Product Manager | Your Idea | **Is it clear?** Can I write a test for it? | Clarify with you. |
| **2. Design** | Architect | Requirements | **Is it complete?** Do limits & errors exist? | Fix the Spec. |
| **3. Plan** | Planner | Spec | **Is it safe?** Do files exist? | Fix the Plan. |
| **4. Build** | Coder | Plan | **Does it compile?** | Fix Syntax. |
| **5. Verify** | QA / Verifier | Code | **Do tests pass?** (Strict Mode) | Fix Logic. |

## 4. How We Work (The Loop)

1.  **You** give an idea (e.g., "Add a dark mode").
2.  **The System** breaks it down into **Atomic Requirements**.
3.  **The System** writes a **Spec** and asks you to confirm it.
4.  **The System** generates a **Plan**.
5.  **The System** writes code and **Loops on Verification** until tests pass.
6.  **The System** presents the finished, tested feature for merge.

## 5. Folder Structure

We organize our files to match this process:

```
/
├── .constitution/    # The Rules
├── product/          # The Requirements (YAML)
├── specs/            # The Blueprints (Markdown)
├── src/              # The Code (Rust)
└── verification/     # The Proofs (Tests)
```
