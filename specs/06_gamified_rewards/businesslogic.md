# Logic: 06_gamified_rewards

## Core Logic

### 1. Economy
- **XP (Experience Points)**: Earned by successful compiles, passing tests, and resolving errors. Used to level up Profile (Cosmetic).
- **Coins (Credits)**: The "Budget". Consumed by LLM calls. Earned (optionally) by completing quests if "F2P" mode (simulated).

### 2. Rewards System
- **Event Listener**: Listen for `TaskSuccess`, `TestPassive`.
- **Payout**:
  - Task Success: +50 XP
  - Test Pass: +10 XP
  - Bug Fix: +100 XP

## Data Flow
EventBus -> RewardSystem -> Bank -> UI Update