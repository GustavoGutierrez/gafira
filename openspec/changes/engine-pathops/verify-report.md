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

---

# Verify report: engine-pathops (C4), round 2 (after verify fix round 1)

- Commit verified: `b713ceb` (HEAD of `feat/engine-pathops-1`, `origin/main` `fea8b6b` merged). Clean `git archive` exports of every commit boundary of the fix round: `ec97f7b` (merge of main), `e3b9cbf`, `9c0d95d`, `dcb06fa`, `24cab46`, `f4438cd`, `fd81282`, `8b467c5`, `17a58c0`, `b713ceb`. Each export had its own `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL=0`; the build folders were freed with `cargo clean` afterwards. The worktree was not modified except for these two files.
- Flow: full | tier T3 | depth V3. Acceptance source: the delta specs as rewritten by the specifier (`d5ccf09`, `2370269`, `e447d07`): 11 requirements, 80 scenarios (path-operations 9 requirements and 72 scenarios, workspace-tooling 2 and 8).
- No `mutants` report was supplied. The independent T3 run is pending, so the mutation figures below are the implementer's and were not reproduced here.

**Verdict: FAIL.** 2 CRITICAL, 4 WARNING and 5 SUGGESTION items. By class (CRITICAL and WARNING): implementation 5, ambiguity 1. C8 from round 1 is carried and waits for an owner ruling on a waiver; it is not counted (see "Carried").

next: specifier (ambiguity: W1), then tdd-apply (implementation: C1, C2, W2, W3, W4)

## Gates (every commit boundary of the fix round)

| Export | fmt | clippy | test | install | ARQ-07 | V11 forbid-unsafe | V5 | V12 | V13 | V14 | deny (bans, licenses, sources) | deny advisories |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| ec97f7b merge | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| e3b9cbf | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 9c0d95d | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| dcb06fa | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 24cab46 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| f4438cd | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| fd81282 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 8b467c5 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 17a58c0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| b713ceb HEAD | 0 | 0 | 0 (720 tests) | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

- How the steps were run: each policy step was extracted from that export's own `ci.yml` and run under `bash --noprofile --norc -eo pipefail`. V13 and V14 ran inside a scratch git repository made in the export.
- V5 with xtrace at HEAD: rc 0. The trace shows `cargo tree -p gafira-edit`, `-p gafira-testkit` and `-p gafira-pathops`.
- Negative runs on fresh HEAD exports: P-N1 (row reduced to `gafira-pathops -`) rc 1, "must not depend on gafira-geom"; P-N2 (row deleted) rc 1, "has clippy.toml but no V5 row"; P-N3 (`gafira-doc` added) rc 1, "must not depend on gafira-doc" (it also reports `gafira-color`).
- A scratch `Instant::now()` test and a scratch `HashMap` test fail `cargo clippy -p gafira-pathops --all-targets -- -D warnings` with `disallowed_methods` (note ARQ-02) and `disallowed_types` (note ARQ-10).
- Dependency scenarios at HEAD:
  - the direct normal set is exactly `gafira-geom`, `kurbo 0.13.1` and `linesweeper 0.5.0`;
  - the `gafira-*` filter gives exactly `gafira-geom` and `gafira-pathops`;
  - none of `gafira-doc`, `-edit`, `-color`, `-time` or `-anim` appears.

## Dependency and lockfile (main merged)

- `linesweeper` is still encapsulated. It is used only in `src/boolean.rs` (`FillRule` and `topology::{BinaryWindingNumber, Topology}`). No public item names it, and its errors map to `PathOpError::Geometry(String)`.
- Lockfile delta against `origin/main`:
  - two new packages, `linesweeper 0.5.0` and `rustc-hash 2.1.3`, plus the `gafira-pathops` entry;
  - one new edge, `arrayvec -> serde` (see S3).
- `Cargo.toml` adds one workspace dependency line, and `ci.yml` adds the V5 row. `cargo deny` passes at every boundary.

## History and hygiene of the new commits (W6)

- All 10 new commits use the repository identity (`Gustavo Gutierrez`) as both author and committer. No commit carries an AI attribution line.
- Commit types and scopes are right:
  - `fd81282` and `17a58c0` (`test`) touch only `tests/`;
  - `8b467c5` (`docs`) changes only comment lines;
  - every `fix` commit carries its own tests;
  - the only `openspec` change in the round is `apply-progress.md`, in `b713ceb`.
- No implementer commit changed the delta specs: they were last changed by the specifier commits `d5ccf09`, `2370269` and `e447d07`, which ratify C1.

## Size of the fix round

- `git diff --shortstat ec97f7b..17a58c0`: 16 files, 1262 insertions and 385 deletions (net +877). With the docs commit `b713ceb`: 1441 insertions. Source files account for 255 insertions and 213 deletions.
- This confirms the implementer's figures. The round is over the ~700-line ceiling (W3).
- The whole change against main is 3132 insertions in `crates`, `.github`, `Cargo.toml` and `Cargo.lock`.

## Independent probes (written from the final spec before reading the implementer's tests)

The probes are in the session scratchpad under `vdp2/x-probe/crates/gafira-pathops/tests/` (`vp2.rs`, `vp2_fine.rs`). `vp2` has 21 probes; 20 pass in both the debug and the release build, and the one failure is C1.

- **Fill-rule oracle** (p01): 150 random pairs of 1-3 integer-grid rectangles, for all 4 rule combinations and all 4 operations, with 40 points per combination at least 0.25 from every edge. That is 24 000 points per operation. Each result's containment matched the definition computed from the counting oracle, `A \ B` and `B \ A` never overlapped, and every result carried `NonZero`.
  - Extra, beyond the spec (p01b): a `NonZero` operand with opposite windings, checked against a signed oracle. 2 400 points, all correct.
- **Result `NonZero`** (p02): for every rule pair and both orders, including three empty results (a disjoint intersection, `A \ A` and `A Δ A`). Pass.
- **Named mixed-rule scenarios** (p03): the union in both orders, intersection, difference and xor, and "rule of each operand in both directions". Pass.
- **Empty and moves-only operands** (p04-p06): pass.
  - Clones are equal in value for R, C and the ring G, with both rules of E, in every survivor position.
  - `∅ \ A`, `A ∩ ∅`, `∅ ∩ A` and every operation over two empty paths hold no element.
  - A moves-only path (and a moves-plus-closes path) is empty, takes the clone rule, and `contains` answers `Outside`.
  - Validation runs before the empty rule: tolerance 0 or NaN, a NaN move, and an open operand against E.
  - Move points count in `m`: `MoveTo(1e6, -2e6)` against R at 1e-9 gives `ToleranceTooFine { minimum: 2e-6 }` in both positions.
  - Two empty paths accept 5e-324.
- **`ToleranceTooFine` boundary** (p07, p08), straight and curved operands: the minimum is accepted by all four operations, and `next_down(minimum)` is rejected with exactly `1e-12 × m`. This holds for straight rectangles (m=15), for negative coordinates (m=40), for a control point at 5000, and for `circle(1000,1000,300) ∩ rect(900,900,1500,1500)` (m=1500).
- **Spec case at 1e-13, debug and release** (`vp2_fine` f1-f3, each run alone under `timeout`): rc 0 in both builds.
  - `ToleranceTooFine { minimum: 1.5e-9 }` at 1e-13, 1e-14, 1e-15 and 1e-300. There was no panic and no abort.
  - At the minimum, and at 2 and 10 times it, all four operations succeed in both operand orders, in release too.
  - Four other curved pairs (unit circle against rect, circle against circle, the large circles, half-plane cuts) succeed at their own minimum in release.
  - The round 1 abort (C5) is gone.
- **Error precedence** (p09): pass for every adjacent pair of the order.
  - Tolerance before a NaN or open path; non-finite A before B; B's +inf before A's open subpath; open A (index 1) before open B; open before too fine.
  - For the stroke: tolerance, then width, then coordinate. For `contains`: tolerance, then finite, then open.
- **Non-finite coordinates** (p10, p11): NaN, +inf and -inf in a curve control point, in either operand position, for all four boolean operations, the stroke, simplify, `contains` and the distance. Each kind is named, including in a move coordinate. Pass.
- **Not-positive and non-finite numbers** (p12): pass.
  - For 0, -0.0, -1, -1e-300 and `f64::MIN` across all seven calls, and for the width: `NotPositive(v)` with identical bits (-0.0 keeps its sign). The message contains "not positive" and the number, and no `NaN`.
  - NaN, +inf and -inf are named by kind, and the width error is distinguishable from the tolerance error.
- **Failure on the second operand** (p13): for all four operations, A is unchanged and B is unchanged, compared by text. Pass.
- **Stroke** (p14): mostly pass; the full-outline sample fails (C1).
  - The capsule bounding box is exactly (-5,-5)-(105,5). The midpoint distance is 5.
  - The X path and the self-crossing polyline are `Inside` at (5,5) with Butt, Round and Square caps, and the outline carries `NonZero`.
  - Every subpath is closed, and a doubled width gives a wider box.
  - **Failed:** sampling the whole capsule outline gives a worst deviation of 1.363e-3 from the exact w/2 boundary at tolerance 1e-3 (C1).
- **Simplify** (p15, p16): pass.
  - Redundant point removed: 4 segments down to fewer.
  - Square, triangle and rectangle come back equal in value. Two subpaths keep their order and `EvenOdd`.
  - On the open corner, every sampled point is within 1e-3.
  - Property: 300 random paths mixing lines, quadratics and cubics, open and closed, at tolerances 1e-3 to 1, sampled along every segment. The worst deviation/tolerance ratio is at most 1.
- **Simplify with NaN or inf never hangs**: p10 and p11 return immediately, under a 900 s `timeout` for the binary. With a finite coordinate of 1e154 or more, it hangs (W1).
- **Hit testing, determinism, tolerance visible, rectangle scenarios, box containment** (p17-p20): pass.
  - 100 equal unions.
  - All 8 calls succeed at 0.5 and at the default tolerance.
  - The crossing bars' boxes overlap by an area of 4, and (1,1) is in neither difference.
  - The xor box spans y 5..10 and the union box y 0..10.
  - Bowtie and zero-length segment accepted; open subpath named in either position.
- **Docs**: the doc comments of the four boolean operations, of `boolean_op` and of `DEFAULT_TOLERANCE` say the result is not guaranteed within the tolerance, list the measured behaviour and call it empirical. The manual scenario passes. `DEFAULT_TOLERANCE` still says the stroke outline stays within the tolerance, which C1 shows is false.

### Comparison with the implementer's tests

Every scenario has an implementer test or command, and every one agrees with my probes. Two weak spots remain:

- "Too fine ... in a debug and in a release build" is tested only in the debug profile; only my probe covers release (W2).
- No test samples the stroke outline beyond its bounding box and the midpoint, which is why C1 went undetected.

The tests use 1e-13 rather than `next_down(minimum)` for the too-fine boundary (S2). The `<=` mutant is still killed by the test at exactly the minimum.

## Flaky-prone properties and C8

- `cargo test -p gafira-pathops --test prop_boolean --test prop_hittest --test prop_stroke` on the HEAD export:
  - 40 consecutive runs, with 40 passes and no new regression file;
  - then 20 runs with `PROPTEST_CASES=1000` and the persisted regressions file moved aside, with 20 passes.
- **The claim is confirmed.** The C8 property `the_difference_is_contained_in_its_first_operand` no longer exists at HEAD. Its replacement `the_boxes_of_the_results_are_contained` guards empty results with `is_empty()`.

## Hand mutants (5, each on its own HEAD export and target)

| # | Mutation | Result |
|---|---|---|
| HM2 (re-run) | `validate_closed` before `validate_finite` in `boolean_op` | killed (`a_non_finite_coordinate_is_reported_before_an_open_subpath`, `a_non_finite_second_operand_is_reported_before_an_open_first_one`) |
| HM4 (re-run) | `validate_finite` deleted from `simplify` | killed by hang: `the_simplification_of_a_non_finite_path_returns_an_error` and `a_non_finite_coordinate_is_rejected_by_every_operation` run past 60 s, suite stopped by `timeout 300` (S4) |
| HM6 | fill-rule selection: `rule_b` taken from the first operand | killed (3 mixed-rule tests) |
| HM7 | tolerance check order: the too-fine check moved after the empty short-circuit | killed (`the_minimum_is_checked_before_the_empty_operand_rule`) |
| HM8 | empty short-circuit: `A op ∅` clone rebuilt with `NonZero` | killed (3 survivor-rule tests) |

## Mutation

No independent T3 report was available to this round. The implementer reports 76 mutants:

- caught 61, unviable 14, missed 1;
- the miss is the known equivalent `hittest.rs:69:25`, carried as W4;
- two real misses were fixed test-first in `17a58c0`.

I did not run cargo-mutants. `.cargo/mutants.toml` is unchanged by the change.

## Status of the round 1 findings

| Round 1 | Status |
|---|---|
| C1 implementer spec amendment | resolved: ratified and rewritten by the specifier (`d5ccf09`) |
| C2 false requirements | resolved in the spec; the new scenarios pass |
| C3 fill rule of the operands | resolved: OQ1 is implemented through `Topology`, and the oracle probe passes for all four combinations |
| C4 tolerance claim | resolved: the claim is removed and the measured behaviour documented |
| C5 abort below the safe tolerance | resolved: `ToleranceTooFine`, confirmed in release |
| C6 stroke fill rule | resolved |
| C7 union with empty | resolved: the clones are equal in value |
| C8 flaky property in D2-D4 | **carried**, see below |
| C9 / HM4 simplify guard | resolved: HM4 is killed |
| W1 / HM2 | resolved: HM2 is killed |
| W2 weak tests | resolved: mixed-rule generators, curve sampling, second-operand failure |
| W3 missing evidence | resolved: both tests exist |
| W4 non-positive named NaN | resolved |
| W5 docs and edge cases | docs and moves-only resolved; the tail after a close without a move is not, and is now C2 because the spec defines an open subpath |
| W6 hygiene | resolved for the new commits; the historical commits are unchanged |
| W7, W8 spec defects | resolved in the spec |
| W9 equivalent mutant exclusion | carried as W4 |

## Carried, awaiting an owner ruling (not counted)

- **C8** (round 1, class implementation): the flaky property in `9a760e0`, `754099b`, `f198f6a` and `15f4b30`, and the false statement in the D2 section of `apply-progress.md`. History is not rewritten. `apply-progress.md` now carries an append-only correction. It waits for the owner's waiver ruling, as was done for gafira-time, and is not re-raised here as a blocker.

## Findings

### CRITICAL

- **C1** class: implementation, ref `stroke.rs:30-35` (`kurbo::stroke` call), "Determinism and numeric tolerance" ("The stroke outline MUST lie within the tolerance of the exact stroke") and the `DEFAULT_TOLERANCE` doc. Round caps and round joins break the tolerance.
  - Cause: kurbo 0.13.1 `round_join` and `round_cap` (`stroke.rs:363-367`) fit a **unit** arc to `tolerance` and then scale it by w/2. The output error is therefore about `min(tolerance, 2.7e-4) × w/2`.
  - Measured on the line (0,0)-(100,0), width 10, round cap: worst deviation 1.363e-3 at tolerance 1e-3 (1.36 times), and 136 times at 1e-5 (the deviation does not shrink with the tolerance). The outer arc of a round join shows the same deviation: 1.363e-3, 1.36 times at 1e-3.
  - Offsets of curves are fine: a circle stroke deviates by 1.1e-9.
  - No test samples the outline beyond its bounding box and the midpoint.
  - A possible fix: pass `tolerance × min(1, 2 / width)` to `kurbo::stroke`, and add a test that samples the whole outline.
- **C2** class: implementation, ref `path.rs` `subpaths`/`validate_closed`, `hittest.rs:30-37`, "Degenerate and invalid inputs". A path that draws after a `ClosePath` without a new `MoveTo` (for example a closed triangle followed by `LineTo(5,20)` and `LineTo(0,20)`) ends with segments and no close, so the spec calls it open whichever way the subpaths are split. The code mishandles it:
  - `validate_closed` returns `Ok`;
  - `contains` returns `Ok(Outside)`, a result where the spec requires the open-subpath error;
  - the boolean operations return `Geometry("non-closed path starting at (0, 0) ...")` instead of `OpenSubpath`.
  This is validation, so it is critical code. It is the leftover of round 1 W5. The subpath index to report is pinned in S1.

### WARNING

- **W1** class: ambiguity, ref path-operations "Degenerate and invalid inputs" and "Path simplification". The spec bounds no coordinate magnitude, and "MUST NOT loop" covers only non-finite values. Measured on release builds, each call under a 20 s `timeout`:
  - `simplify` never returns for a finite polyline with coordinates of 1e154 or more (1e150 returns), in debug (over 60 s) and in release;
  - `distance_to_perimeter` returns `inf` from 1e155;
  - the boolean operations (at their minimum tolerance), `contains` and the stroke return normally up to 1e300.
  The specifier should decide between a coordinate limit with a named error and a termination requirement for every finite input.
- **W2** class: implementation, ref `tests/boolean.rs:473` and `apply-progress.md`. The scenario "Too fine a tolerance is rejected, never aborts" requires a debug **and** a release run. The test runs only under the default debug profile, and no release command is recorded as evidence. My probe (f1, f2 with `--release`) shows it holds. Record a release command, for example `cargo test -p gafira-pathops --release --test boolean`.
- **W3** class: implementation, ref the fix round `ec97f7b..17a58c0`. The round adds 1262 lines and deletes 385 (net +877; 1441 insertions with the docs commit). That is over the ~700-line ceiling, so it needs the owner's explicit OK, as the implementer flagged.
- **W4** class: implementation, ref `hittest.rs:69:25` (round 1 W9, carried). The mutant is equivalent (proof in round 1). The `.cargo/mutants.toml` exclusion, with its reason, is still to be added at archive.

### SUGGESTION

- **S1** (specifier): pin which subpath index a path reports when it draws after a close without a move. The SVG and kurbo convention starts a new subpath, which gives index 1 in the C2 example.
- **S2**: add tests at `next_down(minimum)` (rejected) and at exactly the minimum for straight operands. My probes p07 and p08 show both behave.
- **S3**: `linesweeper` turns on `arrayvec`'s `serde` feature (the lockfile gains `arrayvec -> serde`), so `serde` and `serde_derive` now sit in `gafira-pathops`'s normal tree. This is harmless for ARQ-02 and V5; it is noted for the dependency record. As in round 1, consider declaring `linesweeper` in `[workspace.dependencies]`.
- **S4**: HM4 is detected only by a hang, so a regression would stall CI rather than fail it. A deadline in the test (run the call on a thread and fail after N seconds) would make it fail fast.
- **S5**: `distance_to_perimeter` of an empty path returns `+inf`. Document it, or let the specifier define it.

## Criteria matrix (80 scenarios: 72 path-operations, 8 workspace-tooling)

- Covered: 80 of 80. Every scenario has a passing implementer test or a command I re-ran, and an agreeing probe. One clause is proven only by my probe: the release build of "Too fine a tolerance is rejected, never aborts" (W2).
- At requirement level, two clauses fail and are not exercised by any scenario:
  - "Determinism and numeric tolerance": the stroke outline is not within the tolerance (C1);
  - "Degenerate and invalid inputs": an open subpath after a close is not rejected (C2).

next: specifier (ambiguity: W1), then tdd-apply (implementation: C1, C2, W2, W3, W4)

# Verify report: engine-pathops (C4), round 3 (after verify fix rounds 2 and 3)

- Commit verified: `7814c10` (HEAD of `feat/engine-pathops-1`). Clean `git archive` exports of every commit boundary since round 2: `ea1b17f` (merge of `origin/main`), `aecd57d`, `b7e00ca`, `b711725`, `baa1a0f`, `21131b1`, `02e4ff5`, `c19eba8`, `ac68d6e`, `3839e6e`, `cecfa67`, `eefaf1e`, `7814c10`. The docs-only commits `9671ca1`, `ff5c29e`, `38ee579` and `61fb23a` leave `crates/`, `Cargo.*`, `.github/`, `.cargo/` and the lint configs byte-identical to their parents (`git diff --quiet`), so they share the gate result of `baa1a0f`.
- One `CARGO_TARGET_DIR` for the gate runs and one for probes and hand mutants, with `CARGO_INCREMENTAL=0`. `git archive` stamps every file with the commit time, so a shared target dir reuses stale artifacts. My first gate pass hit this (no `Compiling gafira-pathops` after the first export) and was thrown away. Every export was then `touch`ed, and every log below shows a rebuild of `gafira-pathops`. Disk: 102 GB free at the start.
- Flow: full | tier T3 | depth V3. Acceptance source: the delta specs at HEAD, 13 requirements and 98 scenarios (path-operations 11 requirements and 90 scenarios, 18 of them new since round 2; workspace-tooling 2 and 8).
- No `mutants` report was supplied, and the implementer ran no mutation in fix round 3. The mutation figures are pending (W3).
- Owner waivers applied: C8 (history not rewritten) and the flaky-property history. Neither is counted.

**Verdict: FAIL.** 2 CRITICAL, 5 WARNING and 4 SUGGESTION items. By class (CRITICAL and WARNING): implementation 5, ambiguity 2. This is round 3 of the 3-round cap, and the second FAIL in a row with an `implementation` CRITICAL, so the coordinator stops and asks the owner.

## Gates (every commit boundary, clean exports)

| Export | fmt | clippy | test | ARQ-07 | forbid-unsafe | V5 | V12 | V13 | V14 |
|---|---|---|---|---|---|---|---|---|---|
| ea1b17f (merge main) | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| aecd57d | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| b7e00ca | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| b711725 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| baa1a0f (+ 4 docs commits) | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 21131b1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 02e4ff5 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| c19eba8 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| ac68d6e | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 3839e6e | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| cecfa67 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| eefaf1e | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 7814c10 HEAD | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

- The CI policy steps were extracted from each export's own `ci.yml` and run under `bash --noprofile --norc -eo pipefail`. V13 and V14 ran inside a scratch git repository.
- At HEAD:
  - `cargo deny check`: advisories, bans, licenses and sources ok, rc 0.
  - `cargo test -p gafira-pathops --release --locked`: 142 passed, 0 failed (round 2 W2 release evidence).
  - The three property binaries passed 20 runs in a row, with 0 failures.
- Negative runs on HEAD exports:
  - P-N1 (row reduced to `gafira-pathops -`): rc 1, "must not depend on gafira-geom".
  - P-N2 (row deleted): V5 rc 1, "has clippy.toml but no V5 row". V12 now derives its list from the V5 table, so it passes in that case (rc 0) and V5 alone guards it.
  - P-N3 (`gafira-doc` dependency added): rc 1, "must not depend on gafira-doc" (it also reports `gafira-color`).
  - Drifted `gafira-pathops/clippy.toml`: V12 rc 1, "clippy.toml differs".
  - A scratch test with `Instant::now()` and `HashMap` fails clippy with `disallowed_methods` (note ARQ-02) and `disallowed_types`.

## Independent probes (written from the spec before reading the new tests; scratch export of HEAD)

21 probe tests (`tests/zz_probe.rs` on a scratch export only):

- **Coordinate limit** (p01-p05). Every check passes:
  - `±next_up(1e9)` in an end point, a control point and a move is rejected by all eight operations, the booleans in both operand positions. The error carries the same bits and the limit `1e9`.
  - The first offending value in path order is reported, x before y.
  - The inclusive rectangle `±1e9` passes the four booleans at `1e-11 × 1e9`, and `next_down` of that value is rejected with that minimum. It also passes a stroke (round and butt), the simplification, `contains` and `distance` at the default tolerance.
  - The overflow cases return `1e154` and `1e155`.
  - The error order holds: non-finite before out of range, out of range before an open subpath, and a moves-only `2e9` operand is rejected.
  - A moves-only `MoveTo(5e8,0)` counts in `m`: the union with the default tolerance returns `ToleranceTooFine { 5e-3 }`.
- **Floors** (p06-p08). Every check passes:
  - Simplification: the `1e7` polyline rejects `1e-9` and `next_down(min)` and accepts `min`, and every vertex stays within `min`. A control point at `5e8` sets `m`. A path with `m = 0` accepts `f64::MIN_POSITIVE`.
  - Boolean: the `2e8` case, the circle-rectangle case at `1e-13`, the exact minimum, `next_down` and two empty operands at `5e-324` all behave as the spec says.
  - Stroke: `1e-10`, `5e-7` and `4e-12` hold at their exact boundaries. Width `1e6` at its minimum gives 108 elements. `2e9` gives out of range. A width of `4e9` is too fine at the default tolerance, and `2e9` is accepted.
- **Query point** (p09, p10). Every check passes:
  - NaN, ±inf and `±next_up(1e9)` in x and y are rejected by both hit tests, and `(1e9,-1e9)` and `(-1e9,1e9)` are accepted.
  - The order holds: path before point, open subpath before point, `(1e10, NaN)` gives NaN, and an invalid tolerance comes first.
  - An empty path with a bad point is rejected.
  - Tolerances of `1e-30`, `1e-300` and `5e-324` on a circle return at once, with the right answers: no hang without a floor.
- **Zero-length subpath** (p11). Passes. The spec example comes back as one subpath `MoveTo(5,5)` with segments only at (5,5) and `EvenOdd`, and the square plus that subpath gives two subpaths. Variants also pass: closed, quad and cubic, and in the middle of two squares.
- **Round 2 C1, stroke arcs** (p12b). Resolved. Over open polylines with a cap, joins turning either way and an acute join (widths 0.1 to 1000, tolerances 1e-1 to 1e-7), no outline point lies outside the exact stroke by more than 0.92 × tolerance. Points inside the exact boundary are kurbo's inner-join detour through the vertex, which is interior and changes no covered area.
- **Round 2 C2, drawing after a close** (p13). Resolved for validation: the four booleans in both positions and `contains` return `OpenSubpath { subpath: 1 }`.
- **Drawing after a close, through the simplification** (p11b, p11c, p11e, p15). **Fails**, see C1:
  - kurbo's `segments()`, the distance and the stroke start the tail at (0,0), the start of the closed subpath (SVG convention, as the spec says). The simplification starts it at (10,10), the last drawn point.
  - Spec C2 path at tolerance 1e-3: the point (2.5,10) of the original tail lies 5.30 from the simplified path.
  - 200 random closed polylines followed by a tail: 200 of 200 violate the tolerance, by up to 11.3 at tolerance 1e-2. The same polylines with a `MoveTo` before the tail: 0 of 200.
  - A zero-length tail `ClosePath, LineTo(0,0)` comes back as `MoveTo(10,10), LineTo(0,0)`. That breaks the zero-length rule (start at the input start point; all points equal it).
- **Lone moves through the simplification** (p11d). `MoveTo(1,1) MoveTo(4,4)` gives 0 elements (`subpath_count` 2 to 0), and a lone move before a square gives 2 to 1. See W1.

### Comparison with the implementer's tests

- `limits.rs` and `tolerance_floor.rs` match the probes scenario by scenario, check the exact error values (bits for the coordinate), and pin both sides of each boundary with `next_down`. They are not weak.
- `validation::the_stroke_and_the_distance_accept_drawing_after_a_close` covers the stroke and the distance, but it does not check the simplification of that path. `prop_stroke` generates single open polylines only, with no close, no tail and no lone move. Both of C1's paths are therefore untested.
- `stroke_accuracy.rs` checks the outer contour of closed shapes and the cap of a line. My open-polyline join probe agrees with it.

## Hand mutants (5, each on its own HEAD export, rebuilt from touched sources)

| # | Mutation (rule order or rule input) | Result |
|---|---|---|
| HM1 | `contains`: the query-point check before the open-subpath check | killed (`limits::the_query_point_is_checked_after_the_path_and_non_finite_first`) |
| HM2 | `query_point`: x checked fully (finite, then range) before y | killed (same test, `(1e10, NaN)`) |
| HM3 | `boolean_op`: the empty-operand rule before the too-fine rule | killed (`boolean::the_minimum_is_checked_before_the_empty_operand_rule`) |
| HM4 | `stroke_outline`: minimum from `m` alone (no `w / 2`) | killed (2 tests in `tolerance_floor.rs`) |
| HM5 | `simplify`: range check before the finiteness check | killed (`limits::a_non_finite_coordinate_is_reported_before_one_beyond_the_limit`, `validation::a_non_finite_coordinate_is_rejected_by_every_operation`) |

## Mutation

No report was supplied. `.cargo/mutants.toml` gained only the `History::debug_check` exclusion from `origin/main` (edit-history), and it has a reason. The `distance_to_perimeter` equivalent mutant is still not excluded (W5).

## The implementer's flagged items

1. **Inclusive-limit booleans at the minimum.** Consistent with the spec at `61fb23a`: at `m = 1e9` the boolean minimum is `1e-2`, so the default `1e-3` is rejected (p03 confirms `next_down` is rejected and the minimum accepted). The spec states the consequence: the default works for booleans only up to `m = 1e8`. The text is sound, but its provenance is C2.
2. **The stroke error carries only the minimum.** Compliant: the spec asks for "the same too-fine-tolerance error carrying that minimum".
3. **Query-point checks after the open subpath.** Compliant with error-order steps 5, 7 and 8. Tested, and HM1 is killed.
6. **No mutation run.** Recorded as W3.

## Status of the round 2 findings

| Round 2 | Status |
|---|---|
| C1 round caps and joins | resolved (p12b: outward deviation at most 0.92 × tolerance) |
| C2 drawing after a close | resolved for validation, booleans and `contains`. The simplification of such a path is wrong (new C1) |
| W1 coordinate bound | resolved (owner decision W1; p01-p05) |
| W2 release evidence | resolved (release run 142 passed; apply-progress records the disabled-check abort) |
| W3 size | carried, now W4 |
| W4 equivalent-mutant exclusion | carried, now W5 |

## Findings

### CRITICAL

- **C1** class: implementation, ref `crates/gafira-pathops/src/simplify.rs:57,66`, requirement "Path simplification" and "Degenerate and invalid inputs" (drawing after a close).
  - The defect: `subpaths` splits only before a `MoveTo`, so a segment drawn after a `ClosePath` without a new move reaches `kurbo::simplify::simplify_bezpath` in the same slice. kurbo then starts that tail at the last drawn point instead of the start of the closed subpath.
  - The spec requires every point of the original to lie within the tolerance of the result, and requires the tail to begin at the start of the subpath just closed. Spec C2 path at 1e-3: a tail point lies 5.30 away. Random cases: 200 of 200 fail, by up to 11.3 at 1e-2.
  - A zero-length tail comes back as `MoveTo(10,10), LineTo(0,0)`, against the zero-length-subpath rule.
  - It is critical code: a spec rule with an invariant.
  - Fix test-first: start each tail as its own slice with a `MoveTo` of the closed subpath's start. Add the C2 path and the zero-length tail to the simplification tests, and add closes, tails and several subpaths to the `prop_stroke` generator.
- **C2** class: ambiguity, ref `61fb23a` (delta spec `path-operations`).
  - The commit came after the first implementation commits, and `apply-progress.md` names the implementer as its author. Fix round 3 lists it among its own commits. Deviation 1 says "the spec commit changes the boolean calls of that scenario". Deviation 4 says the stroke rule "was added as new text" because the task's wording did not match the spec.
  - The rule in `verify.md` makes an implementer change to the delta spec CRITICAL, whatever its quality. The content is faithful to owner decision W2 and is internally consistent (checked line by line against the probes). The only extrapolations are the query-point order (x first, non-finite first) and the new inclusive-limit and large-operand scenarios.
  - Remedy: the specifier ratifies `61fb23a` (or rewrites it). No code change is expected.
  - If the coordinator can show the specifier wrote `61fb23a`, this finding is void.

### WARNING

- **W1** class: ambiguity, ref "Path simplification" and `Path::subpath_count`.
  - The requirement says every subpath of the input appears in the output, but it does not say whether a lone `MoveTo` counts as a subpath.
  - The crate counts it (`subpath_count`), and the simplification drops it: moves-only `M(1,1) M(4,4)` goes from 2 to 0, and a lone move before a square from 2 to 1. So "Subpath structure preserved" fails for such paths by the crate's own count.
  - The specifier should state whether lone moves survive the simplification. If they must, it is a code fix.
- **W2** class: implementation, ref `crates/gafira-pathops/src/boolean.rs:29-56`, scenario "Public docs state the coordinate limit".
  - The docs of `union`, `intersection`, `difference` and `xor` do not state the limit `1e9`, the error above it or the minimum `1e-11 × m`. They only link to `boolean_op`, while the spec says "the public documentation of each of the eight operations MUST state the limit".
  - `boolean_op`, `stroke_outline`, `simplify`, `contains`, `distance_to_perimeter`, `DEFAULT_TOLERANCE` and `COORDINATE_LIMIT` do state it.
- **W3** class: implementation, ref fix round 3 sources (`validate.rs`, `path.rs`, `simplify.rs`, `stroke.rs`, `hittest.rs`, `boolean.rs`, `error.rs`). There is no T3 `mutants` report for the round's code, so the critical-mutant gate is not evidenced. If the pending report shows a missed or timeout mutant in this critical code, it adds a CRITICAL.
- **W4** class: implementation, ref `38ee579..7814c10`. Fix round 3 measures 15 files, +1025/-91, and the crate code alone +824/-63. That is over the ~700-line ceiling, so it needs the owner's explicit OK (the implementer reported it).
- **W5** class: implementation, ref `crates/gafira-pathops/src/hittest.rs:93`. The equivalent mutant `<` to `<=` in `distance_to_perimeter` (proof in round 1) is still not excluded in `.cargo/mutants.toml`. Its line moved from 69 to 93, so the archive-time regex must match by name and operator, not by line.

### SUGGESTION

- **S1**: The `Display` texts of `NonFiniteCoordinate` and `CoordinateOutOfRange` say "path has ..." even when the query point is at fault. A neutral wording ("a coordinate is ...") would not mislead.
- **S2**: `stroke_accuracy.rs` compares only the outer contour of closed shapes. An open polyline with a turning round join (as in p12b, outward deviation only) would pin the join of open paths too.
- **S3**: The implementer edited the architect's `design.md` (`eefaf1e`, Q4 "Minimum tolerance"). The text matches owner decisions W1 and W2. The architect may want to acknowledge it at archive.
- **S4**: `distance_to_perimeter` on an empty path still returns `+inf` (round 2 S5), and the spec still does not define it.

## Criteria matrix (98 scenarios: 90 path-operations, 8 workspace-tooling)

- 96 of 98 are covered: each has a passing implementer test or a command I re-ran, and an agreeing probe. That includes all 18 new scenarios (`limits.rs`, `tolerance_floor.rs`, `boolean.rs`, `stroke_and_simplify.rs`).
- Partial (1): "Public docs state the coordinate limit" (manual). The four boolean wrappers do not state it (W2).
- Failing at requirement level (1): "Deviation stays within tolerance" (property). The property holds for its generator, but the requirement fails for paths with a segment drawn after a close (C1), which no scenario generates.
- Hand mutants: 5 run, 5 killed.

next: specifier (ambiguity: C2, W1), then tdd-apply (implementation: C1, W2, W3, W4, W5). Round cap reached and second consecutive implementation FAIL: the coordinator asks the owner before routing.

# Verify report: engine-pathops (C4), round 4 (final, owner-authorized, after fix round 4)

- Commit verified: `0b4c7ed` (HEAD of `feat/engine-pathops-1`). Flow: full | tier T3 | depth V3. Acceptance source: the delta specs at HEAD, 11 requirements and 99 scenarios (path-operations 91, one new since round 3; workspace-tooling 8).
- Clean `git archive` exports of every code commit since round 3 (`5c55ebf`, `d1638a3`, `440be8b`, `07a927c`, `8f1978c`) and of HEAD, every file `touch`ed, one fresh `CARGO_TARGET_DIR` for gates and separate ones for probes and hand mutants, `CARGO_INCREMENTAL=0`. Every gate log shows a rebuild of `gafira-pathops`. The docs-only commits `9c0c61f`, `f89f883` and `0b4c7ed` leave `crates/`, `Cargo.*`, `.github/` and `.cargo/` identical to their parents. Disk: 94 GB free at the start.
- `mutants` report: `mutants-pathops/mutants.out` in the session scratchpad (T3, `--file` for all eight source files of the crate, at HEAD).
- Owner decisions applied and not counted: single PR beyond ~700 lines, waiver of C8 and of the flaky-property history, and decision A ratifying `61fb23a` (round 3 C2, treated as resolved as instructed).

**Verdict: FAIL.** 2 CRITICAL, 2 WARNING and 5 SUGGESTION items. By class (CRITICAL and WARNING): implementation 2, ambiguity 2.

## Gates (every code commit since round 3, clean exports)

| Export | fmt | clippy | test (passed/failed) | ARQ-07 | forbid-unsafe | V5 | V12 | V13 | V14 |
|---|---|---|---|---|---|---|---|---|---|
| 5c55ebf | 0 | 0 | 0 (758/0) | 0 | 0 | 0 | 0 | 0 | 0 |
| d1638a3 | 0 | 0 | 0 (759/0) | 0 | 0 | 0 | 0 | 0 | 0 |
| 440be8b | 0 | 0 | 0 (759/0) | 0 | 0 | 0 | 0 | 0 | 0 |
| 07a927c | 0 | 0 | 0 (759/0) | 0 | 0 | 0 | 0 | 0 | 0 |
| 8f1978c | 0 | 0 | 0 (759/0) | 0 | 0 | 0 | 0 | 0 | 0 |
| 0b4c7ed HEAD | 0 | 0 | 0 (759/0) | 0 | 0 | 0 | 0 | 0 | 0 |

- At HEAD: `cargo deny check` rc 0; `cargo test -p gafira-pathops --release --locked` rc 0 (149 passed); the property binaries `prop_boolean`, `prop_hittest`, `prop_stroke` and `after_close` passed 10 runs in a row (40 of 40).
- Negative runs on HEAD exports: row reduced to `gafira-pathops -`: V5 rc 1 "must not depend on gafira-geom"; row deleted: V5 rc 1 "has clippy.toml but no V5 row"; `gafira-doc` dependency added: V5 rc 1 "must not depend on gafira-doc" (and `gafira-color`); drifted `clippy.toml`: V12 rc 1 "clippy.toml differs"; scratch `Instant::now()` and `HashMap` test: clippy rc 101 with `disallowed_methods` (ARQ-02) and `disallowed_types` (ARQ-10).
- Commits since round 3: 8, all by the owner identity, conventional, no attribution lines.
- `origin/main` has moved (archives of edit-history and domain-time). The branch does not merge cleanly: `docs/specs/README.md` conflicts on the status row (W2). No code, `Cargo.lock` or CI file changed on `main`, so the gates above still hold after the merge.

## Independent probes (written from the spec before reading the new tests; scratch export of HEAD, debug and release)

- **Drawing after a close, every operation (P01).** 300 generated cases: a closed polygon, then one or two tails of lines, quadratics and cubics drawn after a close with no move (some with the move written), closed or open, either fill rule. Each compared with its written-out form. All pass in debug and release:
  - simplification at 1e-1 and 1e-3: equal in value to the written-out form, same subpath count, every point sampled along every original segment within the tolerance (worst 0.013 × tolerance);
  - distance to the perimeter: identical values; stroke outline: equal in value in 300 of 300 cases (random caps and joins);
  - closed tails (135 cases): containment identical, the four boolean operations equal in value in both operand positions, and, for straight cases, the point-by-point definition holds away from both boundaries;
  - open tails: the four booleans in both positions, `contains` and `validate_closed` return `OpenSubpath` with the tail's index; after `close_subpaths` the implicit and written-out forms give equal boolean results.
- **Spec tail paths (P02).** The spec path simplified at 1e-3 stays within tolerance; booleans and `contains` return `OpenSubpath { subpath: 1 }`; the zero-length tail comes back as `MoveTo(0,0), LineTo(0,0)`; a degenerate closed subpath followed by a tail no longer panics. **Fails** for a close straight after a move, see C1.
- **Zero-area operands (P03).** A collinear closed triangle, a closed zero-length subpath, a zero-width rectangle, a square followed by a zero-area tail after a close, and a triangle followed by a collinear closed tail, against a rectangle, four operations, both positions: no error, implicit equals written-out, and a 61 × 61 grid away from the boundaries matches the definition (0 wrong points). Simplification, containment and stroke accept them.
- **Limits and floors (P04).** `±next_up(1e9)` in an end point, a control point and a move: all eight operations, booleans in both positions, return `CoordinateOutOfRange` with the same bits and limit `1e9`. The `±1e9` rectangle passes the booleans at `1e-11 × 1e9` (and `next_down` gives `ToleranceTooFine` with that minimum) and the other four at the default. Boolean floor at `m = 15` and for the circle case; simplification floor at `1e7+2` (`1e-9`, `next_down(min)`, `min`) and with `m` set by a tail point (`5e8`); stroke floors `1e-10`, `5e-7`, `4e-12` at both sides; query-point checks and order; hit tests at `1e-30`; two empty operands at `5e-324`. All pass.
- **Lone moves (P05).** `M M` simplifies to no elements; a lone move before, between and after closed squares is dropped; moves-only booleans and containment follow the empty-operand rule. All pass.
- **A close straight after a move (P07, P08). Fails, see C1.** In debug, `simplify` panics (`uninitialized subpath (missing MoveTo)`, kurbo `bezpath.rs:275`) for `M Z`, `M Z M Z`, a square then `M Z`, `M Z` then a square, and `M Z` then a tail. In release it returns elements without a leading move: `M Z` gives `[ClosePath]`, `M Z` then a square gives `[ClosePath, MoveTo(0,0), ...]`. On that output, `distance_to_perimeter`, `contains` and `union` panic in release (`Can't start a segment on a ClosePath`, kurbo `bezpath.rs:902`). The same input reproduces at `7814c10`, so the defect dates from D3. Booleans, stroke, containment and distance accept `M Z` inputs themselves.
- **Path with no initial move (P06; spec silent, release only, because kurbo debug-asserts at construction).** `validate_closed` reports an open one as `OpenSubpath { subpath: 0 }`, but `close_subpaths` returns it unchanged, so the error's advice cannot be followed; the union reads it from the origin. See S1.

### Comparison with the implementer's tests

- `tests/after_close.rs` matches P01 on the main points: its generator mixes tails with and without a move, closed or not, and checks simplify, stroke, distance, the four booleans, containment, `close_subpaths` and the error index against the written-out form. Not weak.
- No test, unit or property, holds a close straight after a move, so C1 is untested. `a_subpath_made_only_of_a_move_is_dropped` covers `M M` only.
- Hand mutant HM2 below is killed by a single property test (`the_interior_operations_read_the_implicit_move`) and by no fixed unit test (S5).

## Hand mutants (5, each on its own HEAD export, rebuilt from touched sources)

| # | Mutation (rule order or rule input) | Result |
|---|---|---|
| HM1 | `simplify` reads `as_bez_path()` instead of the written-out moves | killed (3 tests in `after_close.rs`) |
| HM2 | `boolean_op` feeds the sweep `as_bez_path()` again | killed (`the_interior_operations_read_the_implicit_move`) |
| HM3 | `with_explicit_moves`: a close does not require a new move | killed (4 tests) |
| HM4 | `with_explicit_moves`: the tail starts at the last drawn point | killed (4 tests) |
| HM5 | `with_explicit_moves`: the written move goes to the origin | killed (2 tests) |

## Mutation (T3, from the `mutants` report)

126 mutants over `simplify.rs`, `path.rs`, `boolean.rs`, `hittest.rs`, `validate.rs`, `stroke.rs`, `error.rs` and `lib.rs`: 105 caught, 0 missed, 0 timeouts, 21 unviable, 61 s wall. No critical mutant survives. The new `.cargo/mutants.toml` exclusion (`replace < with <= in Path::distance_to_perimeter`) has a reason and is sound (keeping the first or the last of equally near segments gives the same minimum); the report confirms it is gone while `==` and `>` at `hittest.rs:93` are still generated and caught.

## Status of the round 3 findings

| Round 3 | Status |
|---|---|
| C1 simplification after a close | resolved (P01, P02; `with_explicit_moves`) |
| C2 `61fb23a` | resolved by owner decision A (as instructed) |
| W1 lone moves in the simplification | resolved in spec (`f89f883`) and code for `M M`; the `M Z` form fails (C1) and the spec commit has the same provenance issue (C2) |
| W2 boolean docs | resolved (`boolean.rs:28-32`, `41-45`, `54-58`, `67-71` state `1e9`, `CoordinateOutOfRange` and `1e-11 * m`) |
| W3 mutation evidence | void: T3 report above, 0 missed |
| W4 size | owner approved a single PR (whole change against `main`: 35 files, +7829/-1; crates and CI +4546) |
| W5 equivalent-mutant exclusion | resolved (`07a927c`) |

## Findings

### CRITICAL

- **C1** class: implementation, ref `crates/gafira-pathops/src/simplify.rs:54-62,74` (`subpaths` and the call to `simplify_bezpath`), requirement "Path simplification" and the Terms ("Empty path": only moves and closes).
  - The defect: a subpath made of a move followed by a close, with no segment, reaches `kurbo::simplify::simplify_bezpath` as `[MoveTo, ClosePath]`. kurbo drops the move and pushes the close onto an empty path.
  - Debug builds: `simplify` panics for `M Z`, `M Z M Z`, a square then `M Z`, `M Z` then a square, and `M Z` then a tail.
  - Release builds: `simplify` returns a path without a leading move (`[ClosePath]`, `[ClosePath, MoveTo(0,0), ...]`). The subpath appears in the output, against "MUST NOT appear". `distance_to_perimeter`, `contains` and `union` of that output then panic in release. One operation of the crate thus breaks another on valid input.
  - The input is valid and empty by the spec's own definition. Present since D3 (reproduced at `7814c10`); no test holds the form.
  - Fix test-first: drop a slice that holds no segment before calling the simplifier. Test `M Z`, `M Z M Z`, `M Z` before and after a square and `M Z` before a tail, in debug and release, and reuse each result in `contains`, `distance_to_perimeter` and a boolean. Let the `after_close.rs` generator emit a close straight after a move.
- **C2** class: ambiguity, ref `f89f883` (delta spec `path-operations`, "Path simplification" and the new scenario).
  - Fix round 4 lists this spec commit among its own work ("W1 spec (docs commit first)"), after the first implementation commits. `verify.md` makes an implementer change to the delta spec CRITICAL, whatever its quality.
  - The text says it is "ratified by the owner's instruction to close the change", but the repo records no such owner decision (the "Owner decisions" table ends at W2). The content is faithful to round 3 W1 and matches the code for `M M`.
  - Remedy: the specifier ratifies `f89f883` and records the ratification. A good moment is when S2 is applied. If the coordinator shows the owner ratified it, as for `61fb23a`, this finding is void.

### WARNING

- **W1** class: ambiguity, ref `openspec/changes/engine-pathops/specs/workspace-tooling/spec.md` (MODIFIED "Per-crate allowed-dependency table (V5)" and "Formatting and per-crate Clippy policy").
  - The MODIFIED blocks were written against an older main spec. At archive they would replace the current requirements, dropping:
    - the `gafira-time` row and its notes (added by `domain-time`, `17fc5ff`);
    - scenarios N1-N4 of V5;
    - the scenarios "Disallowed I/O flagged", "HashMap and HashSet flagged in every domain crate (H-N1)" and "BTreeMap allowed";
    - the `gafira-time` copy in the clippy list.
  - The specifier should rebase both blocks onto `origin/main`'s `openspec/specs/workspace-tooling/spec.md`, adding only the `gafira-pathops` row, notes and scenario.
- **W2** class: implementation, ref `docs/specs/README.md`. `feat/engine-pathops-1` conflicts with `origin/main` on the status row of `02-arquitectura-software.md` (main records edit-history and domain-time as archived). Merge `origin/main` and resolve before the PR. The gates are unaffected (no code or CI change on main).

### SUGGESTION

- **S1**: A path whose first element is a segment (no `MoveTo`) is not a valid `BezPath`. kurbo debug-asserts at construction; in release `validate_closed` reports it open while `close_subpaths` cannot close it, and the booleans read it from the origin. The spec is silent. Either state that a path starts with a move and reject it with a named error, or document the origin convention.
- **S2**: Reword the simplification rule from "a subpath made only of a move" to "a subpath that holds no segment (moves and closes only)". That matches the Terms and covers C1's form explicitly.
- **S3** (carried from round 3 S1): `NonFiniteCoordinate` and `CoordinateOutOfRange` display "path has ..." also when the query point is at fault.
- **S4** (carried): `distance_to_perimeter` on an empty path returns `+inf`; the spec does not define it.
- **S5**: Add a fixed unit test for the zero-area sliver that motivated `d1638a3` (union of a closed zero-area tail against its written-out form). HM2 is killed today only by a generated property.

## Criteria matrix (99 scenarios: 91 path-operations, 8 workspace-tooling)

- 99 of 99 scenarios pass as written, with an implementer test or a command I re-ran and an agreeing probe. That includes the new "A subpath made only of a move is dropped" and every drawing-after-a-close scenario.
- Failing at requirement level (1): "Path simplification" (a subpath with no segment must not appear; the operation must not panic) for a close straight after a move (C1). I count it as not covered: 98 of 99.
- Hand mutants: 5 run, 5 killed. Mutation T3: 0 missed, 0 timeouts.

next: specifier (ambiguity: C2, W1), then tdd-apply (implementation: C1, W2). This is round 4, beyond the 3-round cap, and the second consecutive FAIL with both an `implementation` and an `ambiguity` CRITICAL: the coordinator stops and asks the owner before routing.

# Verify report: engine-pathops (C4), round 5 (closing pass, narrow)

- Commit verified: `9d549f1` (HEAD of `feat/engine-pathops-1`). Flow: full | tier T3 | depth V2, narrowed by the coordinator to the round 4 findings. Acceptance source: the delta specs at HEAD (path-operations 9 requirements, 91 scenarios; workspace-tooling 2 MODIFIED requirements, now restating 23 scenarios).
- Code trees since round 4: `a7c1dcb` (fix) and `c8aff2e` (test). The merge `4ac92bf` changes no file under `crates/`, `Cargo.*`, `.github/`, `.cargo/` or `deny.toml` against `a7c1dcb`; `951e777`, `5f9ead0`, `de7b8fe` and `9d549f1` are docs-only; HEAD equals `c8aff2e` on code. Clean `git archive` exports of `a7c1dcb` and HEAD, every file `touch`ed, fresh `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`; both logs show a rebuild of `gafira-pathops`. Disk: 90 GB free at the start.
- Mutation: no fresh `mutants` report; the coordinator states the T3 evidence for the closing pass, which matches the `cargo mutants` outcomes in the session scratchpad (`mut3`: `simplify.rs` 8 mutants, 6 caught, 2 unviable; `mut4`: `path.rs` 43 mutants, 40 caught, 3 unviable; 0 missed, 0 timeouts). The earlier `--jobs 4` run had 19 load timeouts; the reruns at `--jobs 2 --minimum-test-timeout 120` over the same files supersede it. No `.cargo/mutants.toml` change since round 4.
- Treated as resolved as instructed: round 4 C2 (`f89f883`, ratified by the owner's instruction to close the change).

**Verdict: FAIL.** 1 CRITICAL, 0 WARNING, 4 SUGGESTION items. By class (CRITICAL and WARNING): ambiguity 1. The code is clean; the only blocker is process (spec commits by the implementer), and it needs a ratification record, not a code or spec change.

## Gates (clean exports)

| Export | fmt | clippy | test (passed/failed) | ARQ-07 | forbid-unsafe | V5 | V12 | V13 | V14 |
|---|---|---|---|---|---|---|---|---|---|
| a7c1dcb | 0 | 0 | 0 (766/0) | 0 | 0 | 0 | 0 | 0 | 0 |
| 9d549f1 HEAD | 0 | 0 | 0 (767/0) | 0 | 0 | 0 | 0 | 0 | 0 |

- At HEAD: `cargo deny check` rc 0 (advisories, bans, licenses, sources ok); `cargo test -p gafira-pathops --release --locked` rc 0 (157 passed).
- The CI policy steps ran from `ci.yml` under `bash --noprofile --norc -eo pipefail` on the export (V13 and V14 need a git index; the export was `git init`ed and committed first).
- `origin/main` (`c7187ca`) is an ancestor of HEAD: the PR merge is a fast-forward, `git merge-tree` reports 0 conflicts, and 0 commits of main are missing. Against main the branch adds 36 files, +8507/-1; outside `openspec/changes/` and the crate it touches only `.cargo/mutants.toml`, `ci.yml` (+1, the V5 row), `Cargo.toml` (+1), `Cargo.lock` (+32) and the README row.
- Commits since round 4: 6 own plus the merge, all by the owner identity, conventional, no attribution lines.

## Independent probes (written from the spec before reading `tests/empty_subpaths.rs`; scratch export of HEAD)

- **P1, simplification over every mix (9840 cases, debug and release).** Fragments: a closed square, a closed triangle, `M Z`, `M Z M Z`, `M Z Z`, a lone `M`, a bare `Z`, a closed tail (line and quadratic) and an open tail (line and cubic) drawn after a close with no move; every sequence of 1 to 4 fragments that starts with a move, both fill rules, tolerances `1e-1` and `1e-3`, each case under `catch_unwind`. Reference written independently (implicit moves written out, subpaths with no segment dropped, a close after a close collapsed). Result: 0 panics and 0 failures in debug and release. Every output starts with a move or is empty, holds no `M Z`, `M M`, trailing `M` or `Z Z`, equals the simplification of the reference element by element, keeps the fill rule and the reference's subpath count, and simplifies again without a panic; `contains`, `distance_to_perimeter`, `validate_closed` and `close_subpaths` accept it.
- **P2, every operation, original and reused output (1092 cases, debug and release).** Sequences of 1 to 3 fragments, both rules. On the input and on its simplification at both tolerances, compared with the reference: `is_empty`, `validate_closed` (ok or error alike), `close_subpaths` then `contains`, `contains` and `distance_to_perimeter` at 7 points (two of them on an `M Z` point), the four boolean operations against a rectangle in both positions (results compared on a 37 x 9 containment grid, errors alike), `stroke_outline` with butt, round and square caps (grid). 0 panics, 0 failures.
- **P3, a zero-length subpath with a doubled close.** `M(1,1) L(1,1) Z Z`, the same followed by a tail, and `M Z Z` + tail + `Z Z Z`: simplify three times in a row and run every operation on each output, debug and release. No panic, no error, idempotent. The zero-length subpath keeps its `Z Z` (it is copied as it is), the other forms collapse; see S1.
- **Positive control.** P1 fails on the round 4 behaviour (hand mutant HM1 below), so the probe detects the defect.

- P2 in debug: the same 1092 cases, 0 panics and 0 failures (364 s).

### Comparison with the implementer's tests

- `tests/empty_subpaths.rs` holds the 8 fixed forms (`M Z`, `M Z M Z`, square + `M Z`, `M Z` + square, two squares with `M Z` between and after, square `Z Z`, `M Z` + tail, square `Z Z` + tail), each against the path without them, over simplify (with output reuse in `validate_closed`, `contains`, `distance_to_perimeter`, `union`, `stroke_outline`, `simplify`), the queries, the four booleans, the stroke, `close_subpaths`, plus a generated property over squares, `M Z` and bare `Z` in any order. `after_close.rs` now emits `M Z` before and after the polygon. Not weak; P1 and P2 add `M Z Z`, lone moves, open tails and curved tails after empty subpaths, and agree.
- `c8aff2e` (round 4 S5) pins the union of a closed tail after a close against its written-out form. I did not re-run the implementer's claim that it fails with the `boolean_op` fix reverted (hand-mutant budget used below).

## Hand mutants (5, each on its own HEAD export, rebuilt from touched sources; kills counted on the implementer's tests only)

| # | Mutation of `Path::simplify` | Result |
|---|---|---|
| HM1 | no-segment subpaths are not skipped (`if false`) | killed (`a_simplification_never_starts_with_a_close...`, the generated property, `the_simplification_reads_the_implicit_move`) |
| HM2 | the `Z Z` collapse removed | killed (`a_simplification_never_starts_with_a_close...`, the generated property) |
| HM3 | rule order: the zero-length check before the no-segment check | survives, **equivalent**: `is_zero_length` returns false without a segment, so the two branches are disjoint |
| HM4 | the round 3 rule: skip only a slice shorter than 2 | killed (3 tests, as HM1) |
| HM5 | the no-segment check sees only `LineTo` (forgets curves) | killed (`a_curved_path_stays_within_the_tolerance`) |

4 killed, 1 equivalent, 0 surviving.

## Status of the round 4 findings

| Round 4 | Status |
|---|---|
| C1 `M Z` breaks simplify | resolved (`a7c1dcb`; P1, P2, P3; HM1, HM2, HM4) |
| C2 `f89f883` | resolved by the owner's instruction to close the change (as instructed) |
| W1 workspace-tooling delta | resolved in content (`951e777`): each MODIFIED block, diffed against `origin/main`'s requirement, only adds lines: the `gafira-pathops` row, the landing sentence, the layer-2 note, the copy in the clippy list, "seven rows" in N0, "six" copies, `gafira-pathops` in H-N1, and the P-N1, P-N2, P-N3, geom-edge and byte-identical scenarios. Nothing of main is dropped: the `gafira-time` row and notes, N1-N5, the known-limitation, D-N1, D-N2, E-N1, E-N2 and dev-dependency scenarios, "Disallowed I/O", H-N1, "BTreeMap allowed" and the `gafira-time` copy are all kept, so the merged spec keeps them. Provenance: see C1 below |
| W2 merge with main | resolved (`4ac92bf`; fast-forward against `c7187ca`, README row keeps main's text and adds `engine-pathops` as active) |
| S1 path with no initial move | accepted, recorded under "Not settled" |
| S2 simplify wording | applied (`5f9ead0`, "holds no segment (moves and closes only)"); provenance: see C1 |
| S3, S4 | accepted as they are |
| S5 fixed sliver test | applied (`c8aff2e`) |

## Findings

### CRITICAL

- **C1** class: ambiguity, ref `951e777` (delta spec `workspace-tooling`) and `5f9ead0` (delta spec `path-operations`, "Path simplification").
  - Both commits change a delta spec after the first implementation commit, and the closing pass in `apply-progress.md` lists them as its own work ("W2/W1", "S2"). `verify.md` makes an implementer change to a delta spec CRITICAL, whatever its quality; the instruction treated as resolved covers `f89f883` only.
  - The content is right: W1 is a pure rebase (only additions against main) and S2 is the wording round 4 proposed. No spec or code change is needed.
  - Remedy: the coordinator records in `apply-progress.md` that the owner's instruction to close the change also ratifies `951e777` and `5f9ead0` (as for `f89f883`), or the specifier ratifies them. With that record this finding is void and the change can go to archive.

### SUGGESTION

- **S1**: A zero-length subpath keeps a doubled close (`M(1,1) L(1,1) Z Z` comes back as it is), while every other subpath has `Z Z` collapsed. Harmless (P3: every operation accepts it, the result is idempotent), but one canonical form would be simpler: collapse in that branch too.
- **S2**: The V5 paragraph of the `workspace-tooling` delta states the `gafira-pathops` landing twice ("and for the `gafira-pathops` row (C4 D1)" and "The `gafira-pathops` row lands with that crate (C4 D1)"). Drop one when merging at archive.
- **S3**: No scenario names `M Z` in "Path simplification"; the requirement text covers it and `tests/empty_subpaths.rs` proves it. The archive may add one ("A subpath of a move and a close is dropped").
- **S4**: `apply-progress.md` records the whole change as +8458; measured at HEAD it is +8507 (the two later docs commits). Informational.
- Carried and accepted, not counted: no initial move (round 4 S1), message wording when the query point is at fault, `distance_to_perimeter` of an empty path is `+inf`.

## Criteria matrix (114 scenarios: 91 path-operations, 23 workspace-tooling)

- path-operations: 91 of 91, including "Path simplification" at requirement level for subpaths with no segment (round 4 C1), now proved by `tests/empty_subpaths.rs` and P1 to P3.
- workspace-tooling: 23 of 23. Re-run on the HEAD export: N0 (V5 under xtrace runs `cargo tree` for all seven rows, rc 0), P-N1 (row `gafira-pathops -`: rc 1, "gafira-pathops must not depend on gafira-geom"), "Doc may depend on geom and color" (exactly `gafira-color`, `gafira-doc`, `gafira-geom`), "Pathops may depend on geom" (exactly `gafira-geom`, `gafira-pathops`), dev-dependencies absent from the `gafira-edit` normal tree, "Entries present in all copies" (six files, both entries), "Clean tree" (fmt and clippy rc 0), V12 rc 0. The other restated main scenarios (N1-N5, D-N1, D-N2, E-N1, E-N2, the known limitation, "Disallowed I/O", H-N1, "BTreeMap allowed") are unchanged by this change and covered by the archived edit-history and domain-time verifications; P-N2, P-N3, H-N1 for `gafira-pathops` and the byte-identical copy were run in rounds 1 to 4 and nothing they read has changed.
- Hand mutants: 5 run, 4 killed, 1 equivalent. Mutation T3 (closing pass files): 0 missed, 0 timeouts.

next: specifier (ambiguity: C1, a ratification record only; the coordinator may void C1 by recording the owner's ratification of `951e777` and `5f9ead0`, then route to archive).
