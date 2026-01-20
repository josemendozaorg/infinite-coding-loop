# Logic: 16_socratic_dialogue

## Core Logic

### 1. Questioning Phase
- **Trigger**: Before finalized Plan or Critical Code Change.
- **Process**: LLM generates 3 critical questions:
  1. "Is this the simplest way?"
  2. "What are the edge cases?"
  3. "Does this align with the original Goal?"
- **Answer**: LLM (or User) answers them.

### 2. Refinement
- If answers invoke doubt, trigger Replan.

## Data Flow
PlanCandidate -> SocraticModule -> Q&A -> VerifiedPlan
