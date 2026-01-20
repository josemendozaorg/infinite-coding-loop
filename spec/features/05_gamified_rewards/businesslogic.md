# Logic: 05_gamified_rewards

## User Logic
The interface behaves like a "Factory Tycoon" or "RTS Game".
- **Coins:** The currency of successful engineering.
- **XP:** Experience points for individual workers.
- **Quests:** High-level tasks (e.g., "Refactor Auth Module").

## Core Logic: The Reward Function
The system must deterministically award points based on events:

### Reward Table
| Event Trigger | Reward (Coins) | XP Gain |
|---|---|---|
| `TestPassed` | +50 | +10 (Coder) |
| `BugFixed` | +100 | +20 (Debugger) |
| `RefactorComplete` | +30 | +5 (Architect) |
| `Deploymentsuccess` | +200 | +50 (Ops) |
| `SocraticDiscovery` | +25 (For finding a flaw in plan) | +15 (Researcher) |

### Penalty Table
| Event Trigger | Cost (Coins) |
|---|---|
| `TestFailed` | -10 |
| `BuildError` | -5 |
| `RevertCommit` | -15 |

## Implementation Strategy
1.  **Bank System:** `struct Bank { balance: u64 }` stored in `WorldState`.
2.  **Event Listener:** A system that listens for specific `EventTypes` on the Bus.
3.  **UI Feedback:** When coins are earned, trigger a visual "Pop-up" animation in the TUI (e.g., `+50` floating text).