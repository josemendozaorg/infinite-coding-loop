# F48 - CLI Environment Verifier - UI/UX

## 1. Dashboard Widget
A new panel or widget in the **Mission Control Grid** (or a dedicated "System Status" tab).

**Display:**
- Lists tools in categories (Core, AI, Utils).
- Icons/Color Coding:
    - ✅ Green Check: Installed & Version OK.
    - ⚠️ Yellow Warning: Installed but outdated (optional).
    - ❌ Red/Grey: Missing.
- **Columns:** Tool Name | Version | Status

## 2. Interactive "Fix It" Prompt
If missing tools are detected, a prominent but non-intrusive notification appears:
> "5 recommended tools are missing. [Run Setup Mission]"

**Mission Preview:**
When "Run Setup Mission" is selected, show a preview:
```
PROPOSED MISSION: Install Missing Tools
-------------------------------------
- Install 'gh' (GitHub CLI) via brew/apt
- Install 'opencode' via npm
- Install 'ripgrep' via cargo

[ Execute ]  [ Cancel ]
```

## 3. Console/Log Output
On startup, a clean summary table is printed to the log (visible in the Log Panel):
```
[ENV] Checking Environment...
[ENV] git ...... 2.40.1 [OK]
[ENV] docker ... Missing [FAIL]
[ENV] gemini ... 1.0.0  [OK]
...
```
