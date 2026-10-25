# Verify report: engine-pathops (C4), deliveries D1-D4, round 1

- Commit verified: `2a9f1a7` (HEAD of `feat/engine-pathops-1`), plus clean `git archive` exports of every commit boundary: `cd2c44c` (D1), `9a760e0` (D2), `754099b` (spec amendment), `f198f6a` (D3), `15f4b30` (D4), `a8d2c0a` (test round), `2a9f1a7` (merge of `origin/main`).
- Flow: full | tier T3 | depth V3 (from `apply-progress.md`). Each export had its own `CARGO_TARGET_DIR`; the worktree was not modified except for these two files.
- No path to a `mutants` report was supplied. The mutation figures below come from my own T3 reproduction on a clean HEAD export.

**Verdict: FAIL.** 9 CRITICAL, 9 WARNING and 6 SUGGESTION items. By class: ambiguity 4, claim 1, implementation 13 (counting CRITICAL and WARNING items only).

next: specifier (ambiguity), then explorer (claim), then tdd-apply (implementation)

## Gates

| Export | fmt | clippy | test | forbid-unsafe | ARQ-07 | V5 | V12 | V13 | deny (bans, licenses, sources) | deny advisories |
|---|---|---|---|---|---|---|---|---|---|---|
| cd2c44c D1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 9a760e0 D2 | 0 | 0 | 0 (flaky, see C8) | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 754099b docs | 0 | 0 | **101** (`prop_boolean::the_difference_is_contained_in_its_first_operand`) | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| f198f6a D3 | 0 | 0 | 0 (flaky) | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 15f4b30 D4 | 0 | 0 | 0 (flaky) | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| a8d2c0a | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 2a9f1a7 HEAD | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

How the CI policy steps were run: each one was extracted from that export's own `ci.yml` and run under `bash --noprofile --norc -eo pipefail`. V13 ran inside a scratch git repository.

Negative runs on scratch exports of HEAD:
- P-N1 (row reduced to `gafira-pathops -`): rc 1, "must not depend on gafira-geom".
- P-N2 (row deleted): rc 1, "has clippy.toml but no V5 row".
- P-N3 (`gafira-doc` dependency added): rc 1, "must not depend on gafira-doc" (it also reports `gafira-color`).
- A scratch `Instant::now()` test fails clippy with `disallowed_methods`, note ARQ-02.
- A scratch `HashMap` test fails clippy with `disallowed_types`, note ARQ-10.

Merge check (git 2.34, so I used `merge-tree` and a trial merge in a shared scratch clone):
- `origin/main` is already merged at HEAD, so there is no conflict.
- `feat/edit-history-10b` merges cleanly; `ci.yml` auto-merges and adds V14.
- `feat/domain-time-1` merges `ci.yml`, `Cargo.toml` and `Cargo.lock` cleanly. Its only conflict is one line in `docs/specs/README.md`.
- On the triple merge, V5, V12 and V14 return rc 0 and `cargo metadata --locked` returns rc 0. The merged V5 table holds both the `gafira-time` and the `gafira-pathops` rows.

## The implementer's three reported findings

1. **Spec-first correction (`754099b`).** The ordering is correct: a separate `docs` commit before the D3 code commit. The content is also correct: my probes confirm that simplification can change the element count and that it replaces straight segments with cubics. It is still a delta-spec change made by the implementer after the first implementation commits (`cd2c44c`, `9a760e0`), so it needs specifier ratification (C1).
2. **Flaky property.** The fix in `a8d2c0a` is genuine. HEAD had 0 failures in 80 runs (40 runs of all three property binaries, plus 40 fresh runs of `prop_boolean` with no persisted regressions). The defect is still present in D2, D3 and D4: fresh failure rates were 9/40 at `9a760e0` and 6/40 at `15f4b30`, and the gate run at `754099b` failed. `apply-progress.md` says it was "fixed before the D2 commit"; that is false (C8).
3. **Invalid first mutants run.** It was discarded. My clean reproduction gave the same numbers as the recorded run, with per-mutant rebuilds in the log.

## Mutation (T3, independent reproduction)

- Command: `cargo mutants -p gafira-pathops --file` for each of the 7 source files, `--jobs 4`, on a clean HEAD export.
- Result: found 73, caught 60, missed 1, timeout 0, unviable 12, critical_missed 0, wall 45 s. This matches the implementer's claim (73 / 60 / 12 / 1).
- Survivor: `hittest.rs:69:25` (`<` to `<=` in `distance_to_perimeter`). It is **equivalent**:
  - Each `distance` is `Point::distance`, which is `hypot` and is never `-0.0`. Equal values are therefore bit-identical, and assigning an equal value changes nothing.
  - A NaN fails both comparisons.
  - An `INFINITY` distance with `closest = INFINITY` stores the same value.
  - No input distinguishes the two versions.
- `.cargo/mutants.toml` is unchanged; adding the exclusion is left to archive, as the implementer stated (W9).

## Hand mutants (5, each on its own scratch export and target)

| # | Mutation (rule order or a deleted guard) | Result |
|---|---|---|
| HM1 | empty-operand short-circuit moved before the validation in `boolean_op` | killed (`an_invalid_tolerance_is_rejected`) |
| HM2 | `validate_closed` before `validate_finite` in `boolean_op` | **survived**: NaN in an open path gives `OpenSubpath` instead of `NonFiniteCoordinate` (W1) |
| HM3 | `contains` decides inside or outside before the boundary check | killed (`a_point_on_the_perimeter_is_on_the_boundary`) |
| HM4 | `validate_finite` deleted from `simplify` | **survived**: `simplify` then loops forever on a NaN path (more than 10 min at 98% CPU; killed by hand) (C9) |
| HM5 | trailing `close_path` deleted from the stroke's `close_all` | killed (2 stroke tests) |

## Independent probes

I wrote 33 probe tests from the acceptance criteria before reading the implementer's tests, plus later diagnostics. They live in `vdp/x-probe` in the session scratchpad.

**Passed:**
- Rectangles (overlapping, disjoint, identical, shared edge, nested with a hole, coordinates around 1e6): bounding boxes, contour counts, closedness and exact areas for all four operations.
- Circles: area identities hold for the generic pairs.
- Bowtie under both fill rules; zero-length segment.
- Open subpath named with its index in either operand; NaN, +inf and -inf named, in either operand.
- Determinism: 100 runs equal, and an FNV digest of the result bits (`c05ccad842c6299d`) is identical across processes and between the debug and release builds.
- Stroke: caps and joins differ as expected, a ring's area is correct, the midpoint distance is w/2, a doubled width gives a wider outline, and degenerate widths are rejected.
- Simplify: a collinear point is removed; square, triangle and open L come back unchanged; the deviation check passes on 300 random polylines with sampled points, and on a circle at 4 tolerances.
- Hit testing: inside, outside, edges, corners, the boundary band at plus or minus 0.5 and 2 times the tolerance, the inclusive bound at exactly the tolerance, clockwise paths, holes, circle distances, and the union and intersection containment property over 300 random rectangle and circle pairs.
- Fuzz: 500 random integer-grid polygons with no panic.

**Failed:** the cases recorded in C3, C4, C5, C6 and C7.

## Findings

### CRITICAL

- **C1** class: ambiguity, ref `754099b`, path-operations "Path simplification". The implementer changed the delta spec after the first implementation commits. The amendment is sound (it drops "MUST NOT contain more segments" and adds "MAY replace straight segments with fitted curves") and came before the D3 code, but the specifier must ratify it. "Already minimal" stays undefined: a circle goes from 17 elements to 6.
- **C2** class: ambiguity, ref path-operations "Boolean operations", "Stroke outlining" and "Determinism". Requirements that are false or contradictory:
  - (a) "A boolean operation over an empty path MUST return the other operand unchanged" is false for intersection (`R ∩ ∅ = ∅`) and for `∅ \ B`. The code returns empty there.
  - (b) The property "the bounding boxes of `A \ B` and `B \ A` do not overlap" is false. Counterexample: A = (0,-10,2,10) and B = (-10,0,10,2) give bounding boxes that overlap by an area of 4. The implementer dropped this clause without recording a deviation.
  - (c) "`A ∪ B` and `A ∩ B` are each other's complements with respect to `A ∪ B`" has no meaning.
  - (d) "Wider than the line by the stroke width on each side" and "the distance from the midpoint to the outline is the stroke width" are both wrong: the geometry gives w/2. The test uses w/2 silently, and "Result within tolerance" has no test at all.
- **C3** class: ambiguity, ref `boolean.rs:47-87`, "Boolean operations" and "Containment agrees with the boolean result". The spec says each operation "MUST take the fill rule to apply", but each path carries its own rule and the spec never says how the two combine. The code applies the first operand's rule to both operands without saying so.
  - Probe: A is a NonZero rectangle; B is EvenOdd nested squares, wound the same way. B covers (5,5)? Outside. A? Outside. The union? Inside.
  - The design (Q6 and the API shape) does not settle this either.
- **C4** class: claim, ref design "Tolerance policy", `boolean.rs:74-80`. The tolerance does not bound the boolean result.
  - Case: circle r=10 at the origin against rect (0,-20,20,20). At tol 1e-3, `A \ B` deviates 7.7e-2 from the exact boundary (77 times the tolerance) and reaches x=0.078 inside B. The area errors are 0.78 and 0.029.
  - The deviation grows roughly as the square root of the tolerance: 311 times it at 1e-4, 3107 times at 1e-6.
  - This breaks "A result MUST be within the stated tolerance" and the crate docs. The design never measured what linesweeper's `eps` actually bounds.
- **C5** class: implementation, ref `boolean.rs:92-101`. `validate_tolerance` accepts any finite tolerance above 0. On `circle(1000,1000,300) ∩ rect(900,900,1500,1500)`:
  - tolerances of 1e-13 to 1e-15 overflow linesweeper's stack in release (SIGABRT, which cannot be caught);
  - in debug they trip a `debug_assert` at `curve/mod.rs:1301/1305`.
  This breaks "returns either a complete result or an error". Linesweeper's own `binary_op` clamps `eps` to `max(m*64*EPSILON, 1e-6)`; the crate does not.
- **C6** class: implementation, ref `stroke.rs:48`, `stroke.rs:107-108`. The outline inherits the input path's fill rule. Under EvenOdd, overlapping stroke areas turn into holes: in an X path and in a self-crossing polyline, the centre comes out Outside. The outline is then not "the outline path that the stroke covers". The stray `const _: FillRule = FillRule::NonZero;` shows the intended rule, which is never applied.
- **C7** class: implementation, ref `boolean.rs:59-61`, "Union with an empty path". `rect ∪ ∅` and `∅ ∪ rect` come back with a rotated start point and an extra `LineTo`, and `circle ∪ ∅` comes back re-segmented. The scenario says "the result equals the rectangle path", but the test checks only the bounding box.
- **C8** class: implementation, ref `9a760e0`, `754099b`, `f198f6a`, `15f4b30`. Those commits do not pass `cargo test --workspace --locked` reliably: the flaky property fails about 15 to 22% of runs, and the gate run at `754099b` failed. The fix landed only in `a8d2c0a`, after D4. So the D2-D4 prefixes do not pass the gates alone, and the note in `apply-progress.md` (D2, "fixed before the D2 commit") is false.
- **C9** class: implementation, ref `simplify.rs:17-18`, HM4. The finiteness guard in `simplify` is critical code: without it, `simplify` hangs on a NaN path. No test covers it, and none covers `stroke_outline` with a non-finite path either.

### WARNING

- **W1** class: implementation, ref `boolean.rs:54-57`, HM2. No test pins the rule that a non-finite coordinate is reported before an open subpath (Q2). The spec only implies that order.
- **W2** class: implementation, ref tests. Weak tests:
  - the non-finite test covers only NaN and uses a `{ .. }` pattern;
  - "Error leaves no partial result" has no test where the second operand fails;
  - `the_simplification_keeps_the_path_within_the_tolerance` is vacuous (bounding-box checks only, plus dead `let (_, _, _, _)` lines);
  - the property generators produce only rectangles (no curves, no mixed fill rules), which is why C3 and C4 went undetected;
  - the simplify property checks vertices only.
- **W3** class: implementation, ref `apply-progress.md`. Two scenarios have no evidence row: "Result within tolerance" (and no test; my probe p13 proves the w/2 form) and "Tolerance is visible in the API" (satisfied by inspection: all seven operations take a tolerance).
- **W4** class: implementation, ref `error.rs`, `stroke.rs:95-105`. A zero or negative width or tolerance is reported as "not a finite positive number: NaN", the misleading kind of error Q2 set out to avoid. The design specified an `f64` payload.
- **W5** class: implementation, ref public docs. Errors in the docs and edge cases:
  - the `union` doc says "covered by both paths";
  - the `simplify` doc still says "never holds more segments", which contradicts the amended spec;
  - `is_empty` says a path with only a move is empty, yet the boolean operations reject that path as `OpenSubpath`;
  - drawing after a `ClosePath` with no `MoveTo` is reported as closed and fails with the library's "non-closed path" text instead of `OpenSubpath`.
- **W6** class: implementation, ref commits. Commit hygiene:
  - all 8 commits are authored as `el Gentleman <agent@gafira.local>`, not the repo identity;
  - `cd2c44c` (feat) rewrites the design so Q1 and Q4 read "CLOSED by owner, 2026-10-26", a date after the commits themselves (2026-10-25);
  - `a8d2c0a` (test) also edits `docs/specs/README.md`.
  No AI attribution lines were found.
- **W7** class: ambiguity, ref path-operations. Smaller spec defects:
  - the "Dependency set" scenario expects the library to appear in a `gafira-*` filter;
  - "Crate purity" limits normal dependencies to gafira-geom, the library and std, but the crate depends on `kurbo` directly (design Q6 overrode the spec);
  - the scenario titled "Distance is zero inside" asserts 10.
- **W8** class: ambiguity, ref spec Q4 and Q5. The open-questions block of the delta spec still lists Q1-Q5 as recommendations, while the design marks them closed. Archive must not merge that block as requirements.
- **W9** class: implementation, ref `hittest.rs:69`. The equivalent mutant survives (proof above). Its `.cargo/mutants.toml` exclusion, with a reason, is still to be added at archive.

### SUGGESTION

- **S1**: `validate_tolerance` is duplicated in four modules; keep one copy.
- **S2**: declare `linesweeper` in `[workspace.dependencies]`, as `kurbo` is.
- **S3**: two design claims need correcting:
  - the lockfile adds only two crates (`linesweeper` and `rustc-hash`, at 0.5.0 and 2.1.3), not four;
  - linesweeper is by Joe Neeman, so "the same author" as kurbo is unverified.
  On the dependency itself: MIT OR Apache-2.0, `cargo deny` passes, it does not leak into the public API (errors map to `Geometry(String)`), and its `FxHashMap` is deterministic.
- **S4**: D1 is 745 changed lines (670 in code), at the ~700 ceiling, against a forecast of 420. D2 is 552, D3 589 and D4 379.
- **S5**: resolve the one-line `docs/specs/README.md` conflict with `feat/domain-time-1` when the second of the two branches merges.
- **S6**: `validate_finite` and `validate_closed` are public but not in the design's API; make them private or document them.

## Criteria matrix (48 scenarios: 40 path-operations, 8 workspace-tooling)

Covered 43. Not covered 5:
- "Union with an empty path": failing (C7).
- "Set identities hold": one clause is false and was omitted (C2b).
- "Containment agrees with the boolean result": failing with mixed fill rules (C3).
- "Result within tolerance": spec wrong and no test (C2d, W3).
- "Stroke of a straight line is a closed capsule": the assertion differs from the spec text (C2d).

Every other scenario has a passing implementer test, or a command I re-ran, and an agreeing probe. That includes the four D1 commands, the clippy scratch scenarios, P-N1 to P-N3, and the clean-tree and V12 checks. At requirement level, "Determinism and numeric tolerance" fails its "within the stated tolerance" clause (C4).

next: specifier (ambiguity: C1, C2, C3, W7, W8), then explorer (claim: C4), then tdd-apply (implementation: C5-C9, W1-W6, W9)
