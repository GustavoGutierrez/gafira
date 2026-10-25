# Apply progress: engine-pathops (C4)

Flow: full | pack: new-capability-or-crate | tier: T3 | verify: V3 | decided-by: owner | reason: new crate and a new dependency edge, so ADR 0001 floors the mode at full

## D1: crate skeleton, path model, error type

Status: in progress. Base: `origin/main` e95e56d. Branch `feat/engine-pathops-1`.

### Commits and gates

Pending.

### Scenario to test

| Scenario | Evidence |
|---|---|
| Dependency set (command) | pending |
| No document or edit dependency (command) | pending |
| Clock call rejected (integration) | pending |
| HashMap use rejected (integration) | pending |
| Path carries subpaths and a fill rule (unit) | pending |
| Fill rule set is exactly two variants (unit) | pending |
| Points come from gafira-geom (unit) | pending |
| V5 row exists (command) | pending |
| Clippy copies stay in sync (command) | pending |
| Workspace gates pass (command) | pending |

### RED evidence

Pending.

### Deviations and notes

- The owner approved `linesweeper 0.5` for the boolean operations and the default
  tolerance of 1e-3 px on 2026-10-26 (`design.md` Q1, Q4).
