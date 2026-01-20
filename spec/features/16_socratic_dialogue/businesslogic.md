# Logic: 16_socratic_dialogue

## User Logic
- AI workers challenge each other's assumptions.
- User review required when a Socratic Dialogue triggers a "Blocker".
- Enhances code quality by forcing a rethink phase.

## Technical Logic
- New event type: `SocraticQuestion`.
- Worker role `Critique` that monitors `AiResponse` events and publishes challenges.

## Implementation Strategy
1. Define `SocraticQuestion` event schema.
2. Implement `SocraticWorker` archetype.
3. Update `Orchestrator` to pause tasks when a high-severity question is raised.
