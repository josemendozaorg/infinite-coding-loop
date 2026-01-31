# Logic: 25_worker_transparency

## User Logic
- Understand *why* an AI worker made a specific decision.
- View confidence scores for proposed actions.
- Transparently track "Socratic" peer-review results.

## Technical Logic
- Formalize the `ThoughtEmitted` event with structured reasoning.
- UI components in TUI to expand/collapse worker reasoning.
- Linking the `Relationship Map` to actual reasoning traces.

## Implementation Strategy
1. Define a standard `Reasoning` schema in the `Event` payload.
2. Update AI prompts to include explicit reasoning tags.
3. Enhance the AI Terminal panel to support threaded reasoning views.
