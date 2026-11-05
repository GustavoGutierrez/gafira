# Design: layer-3b — the four deliveries the layer-3 cut deferred

Status: ready to execute. Mode: standard (the design already exists; nothing here needs `architect` again).
Date: 2026-11-05.

## What this change is

The layer-3 cut closed with six deliveries and deferred four, by the owner's decision **O6** of 2026-11-04. This change builds those four and nothing else.

The design of record is `specs/history/2026-11-04-layer-3/design.md`, which already carries their scope, their dependencies, their size forecast and the critical code each one has to mutate. This document only restates what applying it now requires.

| Delivery | Crate | Scope | Requirements | Depends on | Forecast |
|---|---|---|---|---|---|
| `RE4` | `gafira-render` | frame budget, deadline, backend selection, CPU degradation, progressive refinement | 02 §6.2, §6.3; ARQ-22, ARQ-41, ARQ-43 | `RE2`, `RE3` | 460 (645) |
| `RE5` | `gafira-render` | `PageRenderer`, preview and export quality, export scope | 02 §4.5, §6.2; ARQ-42, FD-57 (partial) | `RE2`, `RE4` | 440 (615) |
| `ME4` | `gafira-media` | playback clock, A/V sync, frame dropping | FV-11; 05 FV-10 (c); 02 §6.5 | `SC1` | 420 (590) |
| `ME5` | `gafira-media` | video exporter (`ports::Exporter`), decode → compose → encode → mux, progress, cancellation, audio track | FV-12, FV-24 (partial); ARQ-43; CAL-16, CAL-17 (partial); D-02 | `ME2`, `ME4` | 500 (700) |

Two streams, `RE` and `ME`, run beside each other: ADR 0015 gives them no compile-time edge, and `ME5` verifies against the testkit double, so it does not wait for `RE5`. Inside a stream the deliveries are serial.

Ready now: **`RE4`** and **`ME4`**, both with their dependencies already in `main`. `RE5` needs `RE4` and `ME5` needs `ME4`.

## Owner decisions

### O8. The rigor of layer 3 carries over to layer-3b — decided by the owner on 2026-11-05

`tdd-apply` applies the hand mutants of the critical code of every delivery and kills each one before committing, and runs the full gate suite on every commit. `verify` runs once per stream at **V1**. The crate-wide T4 at close does **not** run.

This is the owner's answer, given for a second cut: it takes O4 (no T4 at close) and O5 (V1) from `specs/history/2026-11-04-layer-3/design.md` and extends them to these four deliveries.

Reason: the same as in layer 3, time. The gates and the hand mutants carry the correctness; the crate-wide mutation run is the part the owner judged not worth its cost.

What it costs: a defect that only an independent probe of the whole crate would find can reach `develop`. The V2 round of `ME1` found exactly that class once, in the keyframe seek.

Reversible: yes, and the only thing it needs is time. Any delivery can be verified at V2 later, and the reports of layer 3 are the reference for what that costs.

## What is inherited and what is not

- `mutation: off`, tier `T0`, `verify: V1` in every Slice card, `decided-by: owner`, with the reason naming O8.
- The streams keep the file layout that worked in layer 3: one change folder per stream (`layer-3b-render`, `layer-3b-media`), so the two agents never edit the same `apply-progress.md`.
- ADRs `0015` and `0016` are still `proposed` in `docs/adr/`. This change builds on them as designed; the owner's acceptance of those two records is still open and is not part of this change.
- The debt of layer 3 (items 90 to 99 of `specs/README.md`) is not paid here unless a delivery has to touch the same code.
