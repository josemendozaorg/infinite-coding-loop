# Logic: 21_ai_output_display

## User Logic
- See the actual AI response from Gemini workers in a dedicated panel.
- Scroll through historical AI responses for reference.

## Technical Logic
- Filter events by type `AiResponse` from the event store.
- Render responses in a scrollable, word-wrapped text area.

## Implementation Strategy
1. Add `ai_responses: Vec<String>` to `AppState`.
2. Extract AI responses from incoming `AiResponse` events.
3. Create a dedicated TUI panel to display the latest responses.
