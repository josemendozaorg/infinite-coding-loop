# Feature 47: Context Enricher (Walkthrough)

## Overview
**Context Enricher** (F47) automatically aggregates relevant workspace information to provide better context for the Planner.

## Changes
- **New Component**: `ContextEnricher`.
- **Integration**: `PlannerWorker` now calls `enricher.collect()`.
- **Dependencies**: Added `walkdir` to `ifcl-core`.

## Verification
Verified using Headless CLI mode. Confirmed logs contained Workspace Context block.
