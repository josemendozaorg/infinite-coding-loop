# UX/UI Design & Navigation Flow

## 1. Design Philosophy
- **Aesthetic:** "Factory Tycoon" meets "Cyberpunk Terminal". Dark mode, neon accents (Green=Good, Amber=Working, Red=Error).
- **Typography:** Nerd Fonts (Monospace with Icons).
- **Navigation:** Keyboard-driven (Vim keys + Hotkeys).

## 2. Navigation Flow (Storyboard)

### Screen 1: The Main Menu
**Purpose:** Entry point. Select or Resume a Loop.
- **Visuals:** Huge ASCII Logo "INFINITE LOOP".
- **Options:**
  - `[N] New Game (Loop)`
  - `[L] Load Profile`
  - `[M] Marketplace`
  - `[S] Settings`
  - `[Q] Quit`
- **Mockup:** ![Main Menu](./mockups/storyboard_01_main_menu.png)

### Screen 2: The Setup Configurator
**Purpose:** Configure the "Game" parameters.
- **Top:** "Quest Definition" (Input Goal).
- **Mid:** "Barracks" (Worker Group Selector).
  - Users can tweak counts: `Claude [+][-]`, `Gemini [+][-]`.
- **Bot:** "Difficulty" (Constraints).
- **Action:** `[START ENGINE]`
- **Mockup:** ![Setup](./mockups/storyboard_02_setup_config.png)

### Screen 3: The Game Scene (Active Loop)
**Purpose:** Real-time observability.
- **Top Bar:** GLOBAL STATUS | Uptime | Coins Earned | Active Quest.
- **Left Panel (The Factory):** Grid of Worker Cards.
  - *Card:* Avatar | Name | Status (Animated Spinner) | Current Action text.
  - *Gamification:* XP Bars increasing as they work.
- **Center Panel (The Feed):**
  - Scrolling log of Events.
  - Socratic Dialogues appear as "Chat Bubbles".
- **Right Panel (Command Center):**
  - Resource Graphs (CPU/RAM).
  - Active "Quests" list (To-Do).
- **Bottom Bar:** "Space: PAUSE | Tab: VIEW MAP | Esc: MENU"
- **Mockup:** ![Game Scene](./mockups/storyboard_03_active_game_scene.png)

### Screen 4: The Map (Relationship Graph)
**Purpose:** Visualize communication topology.
- **Visuals:** Nodes (Workers) connected by lines (Protocol Channels).
- **Animation:** Packets (dots) moving along lines to show data flow.

### Screen 5: Pause & Feedback
**Purpose:** Intervention.
- **Visuals:** Game Scene dims/blurs.
- **Overlay:** "SYSTEM PAUSED".
- **Input:** "God Mode" Text Input. "Rethink the database choice."
- **Preview:** "Impact: 3 workers will be recalled."
- **Mockup:** ![Pause](./mockups/storyboard_04_pause_feedback.png)
