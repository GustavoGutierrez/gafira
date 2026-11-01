# Verify report: layout-core D5-D7

Current state: round 1 at `a77292d`, depth V2, mode full, tier T4 (`mutation: archive`, no `mutants` report, so no mutation finding): **FAIL**. 4 CRITICAL, 4 WARNING, 4 SUGGESTION. Per class: implementation 6 (3 CRITICAL, 3 WARNING); ambiguity 2 (1 CRITICAL, 1 WARNING); architecture 0; claim 0. Next: `specifier`, then `tdd-apply`.

The D4 report is in git history at `9383597`; this file now covers D5-D7, one delivery by owner decision (Slice card amend `a584943`). Open D4 follow-ups (W7, S2, S5) and earlier ones stay with `archive`.

## Round 1

Gates ran on a clean clone. Probes and hand mutants ran on clean exports, outside the repo. Probes were written from the specs, before the implementer's tests were opened.

## CRITICAL

**C1. A reference of `-0.0` gives a grid target of `-0.0`.** class: `implementation`. Ref: `crates/gafira-layout/src/snap.rs:164-167`, `src/grid.rs:62-71`, row "Destino en la Cuadrícula".
With reference `-0.0`, `(at / spacing).floor()` is `-0.0`, so line `k` sits at `-0.0`. `grid_lines` casts `k` through `i64` and gives `+0.0`. The row's property requires the same bits.
- Probe: step 10, reference `-0.0`: coordinate bits `8000000000000000`, `grid_lines` bits `0`. The offset is right.
- Sweep against `grid_lines`: 2 310 300 cases (1 133 042 negative, 189 415 ties), 8 differ, and all 8 have a reference of exactly `-0.0`. Every other case matches bit for bit.
- It is reachable: `(-0.3_f64).round()` is `-0.0`.
- The impact is one sign bit, and the value compares equal. It is CRITICAL because it fails a stated acceptance property. The test property draws `around` from `-5000..5000`, which never yields `-0.0`.

**C2. `n < 2` for `n < 3` is not equivalent, and it survives.** class: `implementation`. Ref: `src/distribute.rs:63`, FD-11 / D-17 "con menos de 3 libres no hace nada", `apply-progress.md` D5 item 16.
The code is right; the claim and the test are not. With two free boxes the mutant computes the gap, which can overflow when wide boxes overlap, because `first_end` can be `+MAX` while `last_start` is negative.
- `[(-MAX, MAX), (-MAX, MAX)]`: HEAD gives two zero offsets; the mutant gives `Err(NonFinite { field: "gap" })`. The same holds with a locked box between them.
- Adversarial comparison with HEAD: 39 698 of 3 000 000 results differ.
- All 156 tests of the crate pass on the mutant.

**C3. Box checks of margins and elements after the DPI rule survive.** class: `implementation`. Ref: `src/snap.rs:140-148`, row "DPI", `tests/snap.rs:346`.
The code is right: on HEAD, `dpi = 0` with `x1 < x0` gives `InvalidBox` for `moving`, `page`, `margins` and `element`. The row orders all box checks before the DPI rule, but the test proves it for `moving` only. Moving the `margins` and `element` checks after the DPI rule passes all 156 tests.

**C4. A hidden or locked child of an attracting group: the row contradicts itself.** class: `ambiguity`. Ref: 02 row "Elementos que atraen", `src/attractors.rs:33-34`, `tests/attractors.rs:357`.
Answer to the question asked: the row's literal text does not allow what the code does, and it does not allow the alternative either.
- "Quedan fuera ... los nodos ocultos o bloqueados" and "un NaN en un nodo que queda fuera no cambia el resultado" say the NaN must not matter.
- "Cada caja es la de `node_bounds`" and "el grupo ... por la unión de sus hijos" make the group read that child, and `node_bounds` gives `Err` on it.
- Probe: A holds B and C, B holds D and E, E is hidden with a NaN `x`. Empty selection gives `Err(NonFinite { field: "x" })`. Selecting C gives the same error. A locked E gives the same error. Selecting D gives `Ok`, because A and B are out.
- One hidden corrupt child stops the magnet for the whole page. The acceptance examples only cover NaN in the selection and its ancestors, and the implementer pinned one reading in a test.

The specifier must choose one:
- Keep `node_bounds` and state the exception.
- Have the group's box skip nodes that are left out, which also changes finite cases: a hidden child at 500 today widens the box to `x1 = 501`.

## WARNING

**W1. Which computed checks run with the magnet off is invented.** class: `ambiguity`. Ref: `src/snap.rs:240-266`, row "Entradas del Imán".
The row says that an inactive magnet does "las comprobaciones de entrada y de rango" and gives (0, 0). HEAD also runs some computed checks. All inputs here are finite and in range:

| Magnet off | HEAD |
|---|---|
| Moving box at `x = 1e308`, centre overflows | `Err(moving)` |
| Element box at `x = 1e308`, centre overflows | `Err(element)` |
| Threshold `1e308` at zoom 0.05 | `Err(threshold)` |
| Distance that would overflow | `Ok`, not computed |

Row "Suma que desborda" can be read either way, so the row should state it. `align` and `distribute` have no such problem: their row applies the rules "si la caja no interviene".

**W2. Grid neighbours `[k, k+1]`: the equivalence claim is false.** class: `implementation`, `follow-up`. Ref: `src/snap.rs:166`, `apply-progress.md` D5 item 14.
The mutant passes all tests but differs from HEAD in 73 of 1 500 000 adversarial cases, the smallest at reference `5.47e14` px, so the neighbours do change the result. It is a WARNING because that is about 3e10 times the largest page, and no difference appeared below it. HEAD itself stops being the nearest line from about `2^51` (2 of 200 000 cases there, 2 064 at `2^55`).

**W3. A `-0.0` offset is reachable in `distribute`.** class: `implementation`. Ref: `src/distribute.rs:83-85`, `apply-progress.md` "Zero offsets are `+0.0` in all four functions".
Boxes `(-1, -0.0)`, `(0, 5e-324)`, `(0, 1)` in equal-gap mode give `dx = -0.0` on the middle box. The gap underflows to `-0.0`. The spec only says 0, so behaviour is acceptable, but the comment and the note are false, and the `+ 0.0` was removed as unreachable.

**W4. Two criteria have no test that names them.** class: `implementation`. Ref: Slice card, `AGENTS.md` testing policy.
FD-03(c) and FD-11(d) are covered in substance by `fd_06_a_five_px_from_a_guide_the_edge_lands_on_it` and `d_17_locked_elements_do_not_count_and_do_not_move`, and the IDs FD-03 and FD-11 are named elsewhere. No test or comment names these two criteria, so a search on them finds nothing. It is not the CRITICAL of step 3c, which applies per ID.

## SUGGESTION

**S1.** `snap_rotation` is not the nearest multiple from `9.0e15` degrees (`2^53`): 37 704 of about 4 000 000 oracle cases, for example 7e16 gives `7.000000000000001e16`, 16 from the angle. No test reaches this, and below that magnitude the exact oracle has no difference. The row has no range, so a sentence in the spec would settle it. `f64::MAX` gives `Err(angle)`, which fits the intermediate overflow rule.

**S2.** `grid_lines` differs from D4 only at `|x0| >= 1e15`, past `2^53` lines: 7 040 of 1 500 000 inputs, in the `major` flag among positions that already repeat. See "D4 behaviour".

**S3.** `benches/layout.rs:4` says a nightly job checks the budgets. No such workflow or `perf-budgets.toml` exists, and criterion gives no p99 for the `bench_snap_1000` row. ARQ-40 is outside this slice and ARQ-114 only asks for the names; the comment should not imply the job exists.

**S4.** The row example "caja en `x = −MAX`" gives its reason as the distance. A zero-size box fails earlier at the centre, naming `moving`. `Rect(-MAX, 0, 0, 0)` gives `distance`. Both are non-finite errors, so only the wording differs.

## Independent checks asked for

**Per-commit gates:** all pass on clean exports (table under "Gates and integrity").

**Priority and ties (D-16):** all four adjacent pairs and the exact comparison hold on probe. The spot-check is killed.

**Grid target against `grid_lines`:** every row example passes; non-integer steps, negative `k`, mm, in, pt, cm and the shared level all hold bit for bit, except C1. There is no view limit.

**D4 behaviour:**
- `grid_lines` against the D4 code in process: 653 137 Ok pairs and 846 863 Err pairs, with 7 040 differences, all at `|x0| >= 1e15` (S2). Error kinds and order match.
- `node_bounds` against D4 on 400 000 random graphs with cycles, dangling children, shared nodes and bad numbers: 0 differences.

**Cache:** no stale or wrong entry. An independent tree oracle over 300 000 `attractors` calls, 21 844 of them with errors, gives 0 differences. The cache only lives inside one call.

**`attractors`, 100 000 levels:**

| Case | Debug | Release |
|---|---|---|
| Chain, empty selection | 616 ms | 79 ms |
| Chain, selection deep, root or middle | 281 to 291 ms | 32 to 34 ms |
| Chain closed into a loop, gives `Cycle` | 224 ms | 30 ms |
| Comb of 50 000 levels (99 999 boxes) | 581 ms | 74 ms |

Ancestors of the selection hold, including the row's A, B, C, D, E case and NaN in D or A.

**`distribute`:** the stable sort ties `-0.0` and `0.0` in input order. An oracle over 300 000 random calls gives a gap spread of `8.5e-13`. Offset overflow gives `Err`. See C2 and W3.

**Equivalence claims:**

| Claim | Result |
|---|---|
| `n < 2` for `n < 3` | **False**, C2 |
| Grid neighbours `[k, k+1]` | **False** at `>= 5.47e14` px, W2 |
| Reference order before coordinate | Holds: 0 of 1 500 000 adversarial cases differ. The proof holds generally: the later reference is itself closer to the larger target, whether or not a centre lies between them |

**Implementer's choices:**

| Choice | Finding |
|---|---|
| Check order, threshold before zoom | Note: unspecified detail; the DPI row is respected |
| Centres always checked | `align`, `distribute`: allowed. `snap` with the magnet off: W1 |
| Inactive magnet computes no distance | Allowed by the row; part of W1 |
| Zero offsets `+0.0` | Note: spec says 0; the claim is false, W3 |
| `attractors` in document order | Note: unspecified, deterministic |
| `margins: None` left to the caller | Note: the row makes it optional, and `snap` reads no document |
| Cycle or dangling child in a node left out gives `Err` | Note: the row speaks only of NaN; such a tree fails validation |
| Selected id in the arena but unreachable | Note: not an error; only a missing id is |

**Benches:** the four names match §6.7, each registered once. Quick run: `bench_guides_generate` 360 µs, `bench_layout_rects` 25 ns, `bench_snap_1000` 30 µs, `bench_grid_lines` 7.8 µs; each is under its figure on this machine, which is not the reference one. `criterion` 0.8 has `default-features = false`.

**`Cargo.lock`:** +33 entries (32 new names, 1 new version), none removed. `criterion`, `half`, `ciborium`, `rayon` and `plotters` do not appear in the normal or build graph of the workspace. §9.2 lists `criterion` as dev. The V5 row needs no change.

## Hand mutants

Run 4, killed 3. No recorded list was repeated.

| Mutant | Type | Result |
|---|---|---|
| Page and Element swapped in priority (item 2) | Spot-check | Killed by the recorded `fd_06_f_at_equal_distance_each_kind_beats_every_lower_one` |
| `snap` centre as `lo + (hi - lo) / 2` | Independent | Killed by `arq_110_a_result_that_is_not_finite_is_an_error_even_if_the_value_would_be_finite` |
| Centres mode sorted by start edge | Independent | Killed by 3 tests |
| Margin and element box checks after DPI | Independent | **Survives**, C3 |

The three equivalence spot-checks are counted separately above.

## Acceptance map

| Item | State | Proof |
|---|---|---|
| FD-03(c) | covered, W4 | `fd_06_a_five_px_from_a_guide_the_edge_lands_on_it` |
| FD-06(a) to (i) | covered | `fd_06_a_*` to `fd_06_i_*`, probes |
| FD-06(c) | covered, see C4 for the row | `fd_06_c_*`, oracle |
| FD-11(a), (b), (e), (f) | covered | `fd_11_*` |
| FD-11(d) | covered, W4 | `d_17_locked_elements_do_not_count_and_do_not_move` |
| Row Umbral | covered | `fd_06_the_threshold_is_compared_exactly_and_divided_by_the_zoom`, `threshold_must_be_finite_and_above_zero` |
| Row Zoom | covered | `zoom_range_is_closed_and_checked_without_targets_and_with_the_magnet_off` |
| Row Empate | covered | `fd_06_after_the_kind_the_smaller_coordinate_wins`, `d_16_*` |
| Row DPI | **weak**, C3 | `dpi_must_be_above_zero_even_when_nothing_uses_it`, `the_order_of_checks_*` |
| Row Entradas del Imán | covered, W1 | `arq_110_*`, `d_16_zero_margins_*`, `guides_convert_*` |
| Row Elementos que atraen | **contradictory**, C4 | `arq_110_a_nan_outside_the_attracting_nodes_changes_nothing` |
| Row Destino en la Cuadrícula | **failing**, C1 | `fd_06_the_grid_*`, `fd_06_a_major_line_is_exactly_k_times_the_step` |
| Row Giro del Imán | covered, S1 | `fd_06_i_*` |
| Row Entradas de alinear y distribuir | covered | `fd_11_the_result_has_one_offset_per_box_in_the_input_order`, `d_29_*`, `arq_110_*` |
| Row Tolerancia | covered | `fd_11_a_*`, `fd_11_b_*`, oracle |
| ARQ-110 | covered | `arq_110_*` in five test files |
| ARQ-111 | covered | `arq_111_*` in five test files |
| ARQ-114, §6.7 rows | covered | `arq_114_each_layout_figure_has_a_benchmark_named_after_its_row` |
| D-16 | covered | `d_16_*`, `fd_06_f_*` |
| D-17 | **weak**, C2 | `d_17_*` |
| D-28 | covered | `d_28_*`, `fd_06_c_a_rotated_node_*` |
| D-29 | covered | `d_29_locked_elements_count_in_the_box_but_never_move` |

Criteria: 32 counted with FD letters and rows one by one; 28 covered, 4 not (the two weak rules, one failing, one contradictory). FD-11(c) is out of this crate (row "Primer corte").

## Gates and integrity

Gates at HEAD: `cargo fmt --check` 0; clippy 0; `cargo test --workspace --locked` 0 (1 087 passed, 0 failed); `cargo deny --offline check` 0 (advisories, bans, licenses, sources ok, no warnings). Disk: 60 GB free at the start, 56 GB after the builds.

CI policy, all 0: ARQ-07, `forbid(unsafe_code)`, V5, V12, V13, V14.

Per commit on clean exports:

| Commit | Clippy | fmt |
|---|---|---|
| `4a99a1a` | 0 | 0 |
| `117c1dc` | 0 | 0 |
| `866b8b6` | 0 | 0 |
| `761a2ef` | 0 | 0 |
| `7a1b454` | 0 | 0 |
| `65217bc` | 0 | 0 |
| `b947b02` | 0 | 0 |
| `594f304` | 0 | 0 |
| `0c686e2` | 0 | 0 |
| `a77292d` (notes only) | 0 | 0 |

`cargo test` was not repeated per commit; that belongs to V3.

Integrity:
- No tracked file deleted, and no `*.proptest-regressions` added or changed; the test run left the clone clean.
- `a584943` touches only `apply-progress.md` and states why. The Slice card is unchanged since then: the later diff of that file has 0 removed lines.
- No implementer commit touched `docs/specs`; only `dc6e4d6` and `7aa7aa5` did, and both precede the first implementation commit.
- No absolute paths in the tracked files checked, no `#[allow]`, no `.cargo/mutants.toml` change, no AI attribution in commit bodies.

Size, figures only (no ceiling, owner decision), matching the implementer's: `git diff --shortstat develop -- crates Cargo.toml Cargo.lock` gives 17 files, 4 026 insertions, 43 deletions. Source +804 -42, tests and bench +2 918, `Cargo.lock` +299. Forecast 935 (475 + 335 + 125).

next: specifier (C4, W1), then tdd-apply (C1, C2, C3, W2, W3, W4)
