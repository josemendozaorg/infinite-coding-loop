# Data Model: 35_error_handler

## New Fields

### Task (struct)
- `retry_count`: `u32` - Number of times this task has been retried.

## Enums

### TaskStatus
- No changes proposed yet, but `Retrying` could be considered in future. using `Pending` for now.

## Events
- No new event types strictly required, but `Log` events will carry the retry information.
