# Verify report: layout-core D4

Change `layout-core`, delivery D4, round 1, depth V2, mode full, tier T4 (`mutation: archive`, so no `mutants` report and no mutation finding).

Commit verified: `47617dc`, the tip of `feat/layout-core-d4`. Verdict: **FAIL**. Counts: 3 CRITICAL, 5 WARNING, 3 SUGGESTION. Per class: implementation 6 (3 CRITICAL, 3 WARNING); ambiguity 2 (WARNING); architecture 0; claim 0. `next`: `specifier`, then `tdd-apply`.

The D3 report is in git history at `d608bbd`; this file now covers D4.

## CRITICAL

**C1. `grid_lines` drops a line that lies exactly on the view edge.** class: `implementation`. Ref: `crates/gafira-layout/src/grid.rs:51-52`, FD-07 (a), row "Vista de `grid_lines`".
`axis_lines` bounds `k` with `(lo / spacing).ceil()` and `(hi / spacing).floor()`. That quotient rounds, so it can land just past the integer, and the line at `k` is never tried. The edge is inclusive: the row's own example (641 lines from 0 to 3840) counts it, and the function's doc says `lo..=hi`.

| Input | Expected | Got | Why |
|---|---|---|---|
| step 7 px, 3 subdivisions, zoom 4, view 0 to 35 | major line at 35 (`5 × step`) | missing | `35 / (7/3)` is `14.999999999999998`, floor 14 |
| step 1 mm, 150 dpi, zoom 1, view ending on line `k = 13` (153.5433 px, spacing 11.81 px) | that line | missing | quotient `12.999999999999998`, with no subdivisions at all |

Measured: integer px steps 1 to 500, whole-multiple views: 9 329 major lines lost over 387 235 views. Steps in mm: 3 980 of 76 360 views lost a line. A view edge exactly on a line: 21 798 of 152 721 tries lost it. Interior lines are right: 300 000 random views matched an independent scan with 0 differences.
The impact grows in D5: FD-07 snaps to "la línea mostrada más cercana", so a lost line is also a lost snap target.
Why the tests missed it: `fd_07_a` (`tests/grid.rs:118`) has `prop_assume!((hi / step_px).fract() > 1e-6 ...)`, which throws away exact multiples, and its oracle at line 120 is `floor(hi / step) + 1`, the same formula as the code.
Fix hint: widen the loop bounds (`floor` for `first`, `ceil` for `last`) and let the existing `contains` filter decide. On the export that gives 0 lost lines in all three measurements and the whole crate suite stays green. It needs a test that keeps exact multiples.

**C2. The exact rules are not proved; four hand mutants survive.** class: `implementation`. Ref: `crates/gafira-layout/tests/{grid,guides_generate}.rs`, FD-04, rows "Lado de página de `generate`", "Vista de `grid_lines`".
The code is right here: probes P3 and P7 use `next_up` and `next_down` and pass. The tests stand far enough from each limit that a tolerance goes unseen.

| Mutant | Spec | Closest test | Result |
|---|---|---|---|
| V1 `cell < 1.0 - 1e-9` | `c ≥ 1 px` | 9.999 px, so `c = 0.9995` | survived |
| V2 dedup within `1e-6` | "misma posición exacta", gaps can be tiny | property uses `gap >= 0.5` | survived |
| V4 view limit `+ 1e-6` on both axes | "`f64` exacto y sin tolerancia" | `..._with_exact_comparison` stops at +0.01 | survived |
| V5 page side `+ 1e-6` | `≤ MAX_PX`, "por encima, `Err`" | 16 384.01 | survived |

All of this is critical code (size limits, spec rules with an invariant). Tests need `next_up` and `next_down` at each limit, and a pair of guides less than `1e-6` apart that stays two guides.

**C3. No test places a page away from the origin; mutant V3 survives.** class: `implementation`. Ref: `crates/gafira-layout/src/guides.rs:114-115`, FD-03 ("se mide desde la esquina superior izquierda de la página").
`generate` takes a `Rect` and every test uses `x0 = y0 = 0`, apart from overflow cases. V3 adds `page.x0` and `page.y0` to every position and no test fails. The code is right (P12), so one offset-page case closes it.

## WARNING

**W1. Two equivalence claims in the notes are false.** class: `implementation`. Ref: `apply-progress.md` D4 "Equivalent", `crates/gafira-layout/src/guides.rs:98`.
- `ceil`/`floor` swapped: not equivalent. It is the fix for C1, see the table below.
- Raw `finite(gap)` and `finite(margin)` removed: not equivalent. `Length::convert` checks the DPI first, so `generate` with a NaN gap and `dpi = 0` gives `OutOfRange { dpi }` where every other function of the crate gives `NonFinite` for the number. `bleed_rect` and `margin_rect` keep those raw checks, and D3 added ordering tests for exactly this. ARQ-110's table has one bad input at a time, so this is an inconsistency and not a broken criterion. The fix is two lines and one test.

**W2. The FD-07 (c) tests do not reach the worst case.** class: `implementation`. Ref: `tests/grid.rs:147`, `tests/grid.rs:163`.
Judged per the request, because this criterion passed vacuously at the stub. On their own the `fd_07_c_*` tests failed 0 of 5 runs with the `2^k` fallback removed (V7), and 2 of 5 with the 6 px minor check removed (V6). Random steps are mostly large and the fixed cases use zoom 32 and zoom 1, so small steps at low zoom are never tried. The behaviour is protected, because `fd_07_d`, `fd_07_minor_*` and `fd_07_fallback_*` kill both mutants. The bound itself holds: P7 swept 3 201 zooms with dense steps, maximum 1 002 lines. Add step 1 px at zoom 0.05.

**W3. The exact view limit rejects the natural 4K view.** class: `ambiguity`. Ref: `02` row "Vista de `grid_lines`", `tests/grid.rs:41-50`, FD-07 (c).
This is the answer on the `next_down` trim. A UI computes `view = screen / zoom`, and that rounds. With 3840×2160, over 31 951 zooms from 0.05 to 32 in steps of 0.001: `Err(view)` at 3 682 zooms (11.5 %) with the view at the origin, and at 13 245 (41.5 %) with a pan offset. The round zoom 0.9 is one: `3840 / 0.9 * 0.9` is `3840.0000000000005`. The code follows the row as written, so this is not an implementation defect, and it is not the owner-accepted case: the screen here is exactly 3840×2160, not larger. On that screen the grid would vanish at those zooms. The trim in `screen()` is needed for the tests to pass and hides this. The cap `3800 / zoom` in `fd_07_a` hides nothing further about the bound, since `fd_07_c` runs up to the limit. Owner or specifier decision: keep it exact and tell the caller to trim, or define the limit another way.

**W4. API points the spec does not state.** class: `ambiguity`. Ref: FD-03, FD-04, `02` §4.17.2, `crates/gafira-layout/src/lib.rs:10-13`.
- FD-04 names `gafira-layout::guides::generate`, and FD-07 `gafira-layout::grid`; both modules are private, and the path is `gafira_layout::generate`. Either the modules or the spec should change.
- `dpi ≤ 0` is `Err` in `place_guide`, `generate` and `grid_lines`, but the DPI row covers only `bleed_rect` and `margin_rect`, and "Rangos" says an engine checks only ranges with a row. The check comes from `Length::convert` and is sensible; the row is missing.
- Lines at negative positions, pinned by `fd_07_lines_cover_a_view_that_starts_off_the_page`, while the row calls `view` "una región de la página". The `major` flag on them is unproved: V9 (`k >= 0 && ...`) survives.
- `generate` always returns px whatever unit `gap` and `margin` use. FD-03 keeps "la unidad en que se escribió" for typed guides and says nothing on generated ones.
- `place_guide` and its tuple return are invented shape, consistent with FD-03(a), with no conflict. `GuideSpec`, the added `dpi` (FD-04 states it as an input), `grid_lines` (named by the row), `GridLine`/`GridLines` and `pub(crate) convert` raise no objection.

**W5. Size.** class: `implementation`. Ref: AGENTS.md Delivery rules. 1 057 insertions against `develop`, ~700 ceiling, forecast 340 (475 at x1.4): source 269, tests 747, notes 39, spec 2. The ceiling is passed by the tests alone. Redundant test lines are few, about 25: `fd_03_a_typed_value_survives_the_extreme_dpis` (11, the property covers it) and the zoom and view NaN cases in `fd_07_view_is_checked_after_...` (about 8). Trimming will not reach the ceiling, and the fixes above add lines. Owner decides; not a blocker on its own.

## SUGGESTION

**S1.** Two row examples are tested in another form: 641 + 361 uses step 12 with 2 subdivisions, not step 6 with 1, and the zoom 33 case uses 100 000 px, not 1000×1000. Both pass on P7. Using the row's literal values costs no lines.

**S2.** Removing the `major_on_screen <= 0.0` guard (V10) survives. It is reachable only with a subnormal `dpi`, there is `Err` either way and only the field kind changes, and no spec rule covers it. Not treated as a critical survivor.

**S3.** The D4 notes are stale: "Status: partial", "one open question" and 879 inserted lines predate round 2, and round 2 has no gates line.

## Independent checks asked for

**Strict TDD.** Credible, not provable from history. Each feature commit holds tests and code together, so git cannot show the order. Reproduced on an export: `f5a11e0` tests on `33ab1c5` source fail exactly the two named tests, and the property passes as noted; `5333284` tests on `f5a11e0` source fail `fd_07_view_screen_size_...` alone, as noted. With the notes' stub on all three functions, every D4 test the notes list fails, and the only D4 tests that pass are `fd_07_c_at_most_2000_...` and `arq_110_finite_input_gives_finite_output`, the two the notes declare vacuous. The notes are honest here. The weakness is elsewhere: see C1 on the `prop_assume`.

**Vacuous at the stub.** `arq_110_finite_input_gives_finite_output` is proved: V8 is killed by it. `fd_07_c_at_most_2000_...` is only partly proved: see W2.

**General rules beyond the examples.** All correct in the code unless listed; probes P2 (200 000 cases against an independently written reference), P3, P5, P7, P9.

| Rule | Code | Tests |
|---|---|---|
| `2n` per axis | ok | ok |
| Dedup within an axis, not across | ok | across ok; exactness: C2 |
| Order, vertical first | ok | ok |
| No rounding | ok | ok |
| `c ≥ 1` | ok | C2 |
| 25 % and 6 px | ok | ok |
| `2^k` fallback | ok | ok |
| Bound of lines | ok, 1 002 at most | W2 |
| View and page limits | ok | C2 |
| Lines at the view edge | **C1** | **C1** |

**Equivalent mutants in the notes.**

| Claim | Result |
|---|---|
| `ceil`/`floor` swapped | **False**, W1 |
| Raw `finite(gap)` / `finite(margin)` | **False**, W1 |
| `(count - 1) * gap` finite check | True: `usable - inf` fails on `cell` with the same field |
| `major < 6` to `<= 6` | True: the fallback at multiple 1 gives the same lines |
| Fallback multiple from 2 | True: that branch never fits at 1 |
| (15) accumulated positions | Accepted: bytes differ, within the ±0.01 tolerance |

## Hand mutants

Run on a scratch export, all reverted. 12 run, 4 killed.

| ID | Mutant | Result |
|---|---|---|
| Spot-check | item (5) `c >= 1` to `c > 1` | killed: `fd_04_d_...`, `fd_04_page_side_limit_...` |
| V1, V2, V4, V5 | tolerances | survived, C2 |
| V3 | positions offset by the page origin | survived, C3 |
| V6 | minor shown with no 6 px check | killed: `fd_07_d_...`, `fd_07_minor_...` |
| V7 | fallback removed | killed: `fd_07_d_...`, `fd_07_fallback_...`, `fd_07_minor_...` |
| V8 | non-finite guide | killed: `arq_110_finite_input_...`, 2 more |
| V9 | no major flag on negative `k` | survived, W4 (spec silent) |
| V10 | zero-step guard removed | survived, S2 |
| V11 | `ceil`/`floor` swapped | survived, and it fixes C1 |

## Acceptance map

14 criteria: 8 covered, 4 partial, 1 weak, 1 failing.

| Criterion | State | Proof |
|---|---|---|
| FD-03 (a) | covered | `fd_03_a_*`; P1, every DPI 1 to 10 000, worst round trip 7.3e-12 |
| FD-03 (d) | covered | `fd_03_d_*`; `Page.guides` itself: D2 |
| FD-04 (a) | covered | `fd_04_a_twelve_columns_in_1080` |
| FD-04 (d) | partial | `fd_04_d_...`; C2 V1 |
| FD-04 (e) | covered | `fd_04_e_three_by_three_in_1080` |
| FD-04 (f) | partial | `fd_04_f_*`; C2 V2 |
| FD-07 (a) | **failing** | C1 |
| FD-07 (c) | weak | `fd_07_c_*`; W2 |
| FD-07 (d) | covered | `fd_07_d_...` |
| FD-07 (e) | covered | `fd_07_e_...` |
| Row "Lado de página" | partial | `fd_04_page_side_*`, `fd_04_every_ok_...`; C2 V5 |
| Row "Vista" | partial | `fd_07_view_*`; C2 V4 |
| ARQ-110 | covered | `arq_110_*`, `fd_03_d_*`, P8, P10 (400 000 raw-bit inputs: no panic, no non-finite output) |
| ARQ-111 | covered | `arq_111_*`, one per function, bytes compared; P9 |

Slice card IDs left out on purpose: FD-03 (c) and FD-06 are D5; FD-05, FD-09 D3; FD-11 D6; ARQ-114 D7.

## Gates and integrity

| Check | Result |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0; 986 passed, 0 failed |
| `cargo test -p gafira-layout --locked --release` | 0 |
| `cargo deny check` | 0 |
| CI policy: format, ARQ-07, forbid unsafe, V5, V12, V13, V14 | all 0, run from `ci.yml` on a clean clone |

Slice card: commit `f08d005` touches only `apply-progress.md` and precedes the first implementation commit; nothing above the D4 section changed. `33ab1c5` touches only `docs/specs/02-arquitectura-software.md`. No tracked file deleted, no `*.proptest-regressions` added or changed (9 on both sides), `ci.yml` unchanged, no absolute path added, no attribution lines. The commits of this delivery changed no behaviour outside `gafira-layout`.

## Notes for the record

Owner-known and accepted, not findings: the per-axis view limit with no orientation swap; a view above 3840×2160 gives `Err` (rotated or 5K screens); extreme view positions above 2^52 px leave the 2 000 bound unproved.

For D5, from the specifier, not a D4 finding: FD-06 snap cannot call `grid_lines` over a whole page at high zoom. C1 also matters there.

next: specifier, tdd-apply
