# Verify report: domain-color (C2a)

This report has three parts: the pre-archive pass on `main`, the PR2 delivery (`ColorTransform`), and the change-complete check.

**Verdicts**
- Pre-archive (on `aa4280c`): FAIL, because PR2 was missing. This finding is now resolved by PR2.
- PR2 (`feat/domain-color-transform`, commits `ec53526`, `d370b65`, `d11b1ee`): **FAIL**. CRITICAL 1, WARNING 4, SUGGESTION 4. The PR2 code and tests are complete and all 11 scenarios are covered. The failure comes from commit `d11b1ee`: it excludes a mutant in critical code as "equivalent", but that mutant is live.
- Change complete: **FAIL**. Every scenario of every delta spec is covered, but PR2 is not PASS. Archive is also waiting on the owner to confirm the `Result` deviation (task 0.3).

`next`: tdd-apply. Make a small fix on the PR2 branch (CRITICAL 1), then re-verify. After a PASS, the owner confirms task 0.3 before archive.

---

## Part A. Pre-archive findings (on `origin/main` `aa4280c`, before PR2)

This is the summary of the earlier report (scratchpad `verify-c2a.md`). PR1 (T1-T6, merged as PRs #3-#5) was green and covered its own requirements:
- color-model: 23/23 scenarios covered.
- color-conversions: 19/19 non-PR2 scenarios covered.
- workspace-tooling delta: all scenarios covered. N1-N7 rely on earlier evidence. The V5 known limitation is not met and is recorded.

Gates on `aa4280c` were all rc 0: fmt, clippy, test (gafira-color 63 lib + 10 api + 21 properties), `cargo deny check`, V5, V11, V12, ARQ-07, V13. GitHub CI run 40150102530 was green.

| # | Pre-archive item | Status now |
|---|---|---|
| C1 | PR2 not built: PixelLayout, ColorTransform, AnalyticTransform and the `apply_f32` shared-pipeline sentence (11 scenarios) | **Resolved** by PR2 (Part B) |
| W1 | Task 0.3 open (chain strategy and `Result` deviation) | Partly resolved. The chain strategy is moot and annotated at `tasks.md:40`. The `Result` deviation is **still pending owner confirmation** |
| W2 | "Monotonic" spec claimed `f(x) <= f(y)` for encode, which is false at 0.0031308 | **Resolved**. `specs/color-conversions/spec.md:46` now states strict decode and encode with `+ 5e-8`, which matches `tests/properties.rs:145-154` |
| W3 | V5 known limitation (a crate without `clippy.toml` is not checked) | Open by design, recorded. Deferred to `xtask check-layers` (ARQ-01) |
| W4 | Spec and design both label two different cases "N5" | Open (cosmetic). Both cases were run |
| W5 | PR1 split into #3-#5 was not recorded in apply-progress | Resolved by "PR1 delivery note" (`apply-progress.md:135-137`). Task 6.4 is still unticked (see B-W3) |
| W6 | Scratch negatives not re-run; no whole-crate mutation run | Unchanged. Archive must request a whole-crate `cargo mutants -p gafira-color` run |
| S1 | `proposal.md:9-15` says `gafira-cli validate` is in C2b, but it moved to `gafira-format` | Open |

Archive merge notes from the earlier report still apply. Order is C1, then domain-color, then domain-doc. The destructive MODIFIED "Layer purity interim proof" replaces "Geom is a leaf" with "Domain crates are leaves". domain-doc's MODIFIED V5 and V11/V12 retitle or drop scenarios of this change.

---

## Part B. PR2 (`ColorTransform`, tasks 7.1-7.4, 8.1)

Branch `feat/domain-color-transform`, based on `origin/main` `0bb420b` (base is an ancestor of HEAD). Toolchain is rustc 1.93.1 and cargo-deny 0.20.2. Each commit was extracted with `git archive` into the scratchpad and checked there. The policy steps ran in the clean worktree at `d11b1ee`.

### B1. Gates

| Gate | ec53526 | d370b65 | d11b1ee |
|---|---|---|---|
| `cargo fmt --check` | rc 0 | rc 0 | rc 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | rc 0 | rc 0 | rc 0 |
| `cargo test --workspace --locked` | rc 0, 312 passed, 0 failed | rc 0, 312 / 0 | rc 0, 312 / 0 |

gafira-color tests went from 63 to 71 lib (+8), from 21 to 22 properties (+1, P6), and api stayed at 10.

At `d11b1ee`, the CI `policy` steps were extracted from `.github/workflows/ci.yml` with a YAML parser and run under `bash --noprofile --norc -eo pipefail`:
- Format: rc 0
- ARQ-07: rc 0
- V11 forbid unsafe: rc 0
- V5 layer table: rc 0
- V12 clippy copies: rc 0
- V13 EditToken: rc 0
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok

No `Cargo.toml` or `Cargo.lock` changes, so there are no new dependencies.

**Size.** `git diff --shortstat origin/main..HEAD -- crates .github` gives 3 files, +272 -1, which is 273 code lines. That matches the figure in apply-progress. Including `.cargo/mutants.toml` it is 281 lines, against a forecast of ~175 and a budget of 800.

**Code rules.**
- `transform.rs` has no `#[allow]`, `unwrap`, `expect`, `panic!`, `HashMap` or `HashSet`.
- `#![forbid(unsafe_code)]` is unchanged.
- No `bake_lut3d` or `Lut3d` exists.
- Commits use conventional messages with no AI attribution lines.

### B2. Scenarios (color-conversions)

| Scenario | Evidence | Status |
|---|---|---|
| Typed conversions: the typed API and `apply_f32` share one pipeline (`spec.md:120`) | `apply_f32` calls `math::convert_rgb` (`src/transform.rs:105`), the same function as `Color::convert` (`src/color.rs:62`) and the typed methods (`src/rgb.rs:65-93`). Bit agreement is proven by P6 (`tests/properties.rs:156-169`) | covered |
| Channel counts (`:146`) | `src/transform.rs:155`. Derives on `PixelLayout` at `:9` (exhaustive, no `non_exhaustive`) | covered |
| Object safety and thread bounds (`:155`) | `src/transform.rs:161-172`: a `Box<dyn ColorTransform>` passes a `T: Send + Sync` bound and is called through the trait object. Declared deviation: no thread is spawned (see B-W1) | covered (deviation accepted) |
| Rgba slice (`:160`) | `src/transform.rs:175-189`. The 3-pixel RGBA buffer includes the sRGB -> linear pair, is bit-identical to the typed path, and keeps every alpha bit-identical (`:184-186`) | covered |
| Rgb slice (`:165`) | `src/transform.rs:192-205`: sRGB -> P3, 2 pixels, bit-identical to the typed path. `assert_ne` shows the conversion is not a no-op | covered |
| Wrong length (`:170`) | `src/transform.rs:208-219`: (10, Rgba, 4) and (7, Rgb, 3) give the exact `Err`, and the buffer bits are unchanged | covered |
| Empty buffer (`:175`) | `src/transform.rs:222-228`, both layouts | covered |
| Error rendering (`:180`) | `src/transform.rs:242-250`: used through `&dyn std::error::Error`, and the message contains `10` and `4`. Derives and `#[non_exhaustive]` at `:28-29`, `Display`/`Error` at `:40-51` | covered |
| Agreement with the typed API, P6 (`:185`) | `tests/properties.rs:156-169`: all nine pairs through `space()`, both layouts, any bit pattern (NaN, inf, `-0.0`), 0-7 pixels | covered |
| All nine pairs, AnalyticTransform (`:194`) | `src/transform.rs:175-189`: `src()`/`dst()` are checked and results agree with `Color::convert`. The buffer has 3 pixels, not 4 (B-S2) | covered |
| Identity transform (`:199`) | Unit test `src/transform.rs:231-239` (NaN, `-0.0`, inf, subnormal, all three spaces). For the property: P6 with `from == to` over `any_bits`, plus "Same-space identity" (`tests/properties.rs:227`) | covered |
| Slice round trip (`:204`) | No dedicated test. Derived from three properties: P6 (bit identity with `Color::convert`, per step), `display_p3_round_trip_in_unit_range` (`tests/properties.rs:199`, `from = Srgb`, 1e-5 absolute in [0,1]) and P4 alpha (`:173`). Composing two bit-identical steps gives exactly the typed round trip | covered (derived; B-S1) |

Other rules of "ColorTransform trait":
- Alpha untouched: only `pixel[..3]` is written (`src/transform.rs:106`). Hand mutations H1 and H2 were killed.
- No panic and no skipped remainder: the length check happens before `chunks_exact_mut` (`:98-104`). H3 was killed.
- No I/O: ARQ-02 `clippy.toml` and the code itself.
- `AnalyticTransform::new` is `const fn` and infallible (`:82`).

### B3. Mutation

The `mutants` report (scratchpad `mutants-C2a-PR2/report.md`) used gafira-color in-diff mode against `origin/main`. Results: 7 mutants, 5 caught, 0 missed, 0 timeout, 2 unviable.
- Caught: `channels -> 0` and `-> 1`, `apply_f32 -> Ok(())`, `delete !` (length check), `Display::fmt -> Ok(Default)`.
- Unviable: `src`/`dst -> Default::default()` (`ColorSpaceId` has no `Default`).

The delivery's code has no missed mutants.

**Hand mutations** (5). These are rules cargo-mutants cannot express. Each ran on a scratch copy of `d11b1ee`, and the source was restored afterwards:

| # | Mutation | Killed by |
|---|---|---|
| H1 | Overwrite alpha with 1.0 for Rgba | `all_nine_pairs_match_color_convert_on_rgba`, `identity_transform_keeps_every_bit` |
| H2 | Canonicalize alpha (`pixel[3] += 0.0`: `-0.0` becomes `+0.0`, sNaN is quieted) | P6 `apply_f32_agrees_with_the_typed_api` (only the property catches it) |
| H3 | Length check moved after the conversion loop (buffer modified on error) | `wrong_length_is_an_error_and_leaves_the_buffer_untouched` |
| H4 | Error reports `len % channels` as `channels` | `wrong_length_is_an_error_and_leaves_the_buffer_untouched` |
| H5 | `Display` omits the length | `error_names_length_and_channels_and_is_a_std_error` |

**`.cargo/mutants.toml` `exclude_re` entries** (added in `d11b1ee`):

1. `'replace \|\| with && in Transform2D::inverse'` (`.cargo/mutants.toml:7`). The match is narrow: there is one `||` in `inverse`. However, the mutant is **NOT equivalent**.
   - The comment at `:6` claims the later all-finite check returns the same error. That is false when finite coefficients overflow the determinant.
   - Example: for `Transform2D::scale(1e200)`, `det = +inf`, so kurbo's `inverse` computes `inv_det = 0` (`kurbo-0.13.1/src/affine.rs:426-435`), and every coefficient is a finite 0.
   - The original code returns `Err(SingularTransform)` (`crates/gafira-geom/src/transform.rs:63`, documented at `:58-60` as "fails when the determinant is ... not finite").
   - The mutant returns `Ok(Transform2D(Affine([0.0, -0.0, -0.0, 0.0, 0.0, 0.0])))`.
   - Reproduced in a scratch copy: the scratch test `assert!(Transform2D::scale(1e200).inverse().is_err())` passes on the original and fails on the mutant. The whole existing gafira-geom suite (31 + 7 + 11) passes with the mutant in place.
   - So this is a live, uncaught mutant in critical code (transforms, AGENTS.md), and the exclusion hides it. See CRITICAL 1.
2. `'replace Document::schema_version -> u32 with 1$'` (`.cargo/mutants.toml:9`). This one is **equivalent and narrow**.
   - The field is `pub(crate)` (`crates/gafira-doc/src/document.rs:20`).
   - Every public construction path sets `SCHEMA_VERSION`: `from_parts` (`:62`) and `Default` (`:83`).
   - Every load path rejects any other version: `Deserialize` goes through `try_from = "DocumentWire"` and then `validate` (`json.rs:48-59`, `validate.rs:29`).
   - The only writes of other values are in-crate tests, which never read the accessor.
   - The `$` anchor keeps `with 0` active. The removal condition is documented. Accepted.

### CRITICAL

1. `.cargo/mutants.toml:6-7` (commit `d11b1ee`) excludes `replace || with && in Transform2D::inverse` as equivalent, but it is not (see B3.1). This is a missed mutant in critical code (transform inverse) hidden by a false equivalence claim.
   Fix on the PR2 branch:
   - Drop that `exclude_re` entry and its comment.
   - Add a gafira-geom test, for example `Transform2D::scale(1e200).inverse() == Err(SingularTransform)` (determinant overflows to `+inf` from finite coefficients), in its own `test(geom): ...` commit.
   - Optionally also cover `scale_xy(1e200, -1e200)` (`det = -inf`).
   - Re-run `mutants` on `crates/gafira-geom` to confirm it is caught.
   The `schema_version` entry can stay.

### WARNING

1. "Object safety and thread bounds" (`spec.md:155-158`) says the transform is "moved to another thread and called there". The test (`src/transform.rs:161-172`) proves the same property at compile time with a `Send + Sync` bound, because ARQ-02 bans `std::thread::spawn` in domain crates and `#[allow]` is forbidden.
   - The proof is sound: `Box<dyn ColorTransform>` is `Send + Sync` only through the trait's supertraits, so removing them fails to compile. Accepted.
   - `std::thread::scope` is not on the ban list, but using it would exploit a gap in ARQ-02 and should not be done.
   - Amend the scenario before archive, as was done for Monotonic, so the main spec does not describe a test that cannot exist. Suggested THEN: "a `T: Send + Sync` bound accepts it and a call through the trait object succeeds".
2. Task 0.3 (`tasks.md:40`): the `Result` deviation from 02 §4.10 is still pending owner confirmation. The delta spec already requires `Result` (`spec.md:153`). The owner must confirm before archive, or the spec and code must change.
3. `tasks.md:95` ticks 8.2 ("push, open PR2, wait for CI green") before verify, push or CI. Untick it until the coordinator has pushed and CI is green. `tasks.md:81` (6.4) is still unticked even though PR1 merged as #3-#5. Tick it with a note.
4. The P6 property is the only test that catches alpha canonicalization (H2). It depends on `any_bits` drawing `-0.0` or a signaling NaN in an alpha slot, which happened here but is probabilistic. A deterministic unit case with `-0.0` and an sNaN alpha would make this robust (see S3).

### SUGGESTION

1. "Slice round trip" has no dedicated test. The derivation (P6 + `display_p3_round_trip_in_unit_range` + P4) is sufficient, and no test is required. However, `apply-progress.md:159` cites "P2" (linear -> sRGB -> linear), which is the wrong property; the right one is the sRGB <-> P3 round trip (`tests/properties.rs:199`). Fix the citation. A short direct property would make the scenario self-evident.
2. "All nine pairs" (`spec.md:194-197`) says "a fixed 4-pixel RGBA buffer", but `RGBA` (`src/transform.rs:117-121`) has 3 pixels. Add a pixel or amend the wording.
3. Add `-0.0` and a signaling-NaN alpha to `RGBA` (`src/transform.rs:117`) so alpha bit preservation is checked deterministically.
4. `apply-progress.md:3` and `:7` (branch `feat/domain-color`, "DECISION REQUIRED BEFORE PUSH") are stale PR1 text. Mark them historical.

`usize::is_multiple_of` (`src/transform.rs:98`) has been stable since Rust 1.87, within MSRV 1.93. `channels` is always 3 or 4, so the zero-divisor case cannot occur. Required by clippy `manual_is_multiple_of`. Fine.

---

## Part C. Change complete (C2a)

| Check | Result |
|---|---|
| color-model scenarios (23) | all covered (Part A) |
| color-conversions scenarios (31) | all covered: 20 PR1 (Part A) + 11 PR2 (Part B) |
| workspace-tooling delta scenarios | all covered. N1-N7 rely on earlier evidence. V5 limitation not met, recorded |
| Deliveries | PR1 (#3-#5) merged, CI green. **PR2 FAIL (CRITICAL 1)** |
| Tasks | 0.3 open (owner). 6.4 stale. 8.2 ticked early |
| Open items recorded | ΔE2000 deferred (manual scenario, recorded in Part A and `apply-progress.md`); `Result` deviation pending owner (`tasks.md:40`, `apply-progress.md:191`); N5 limitation not met (`apply-progress.md:191`); ARQ-80 403 (`tasks.md:39`, `apply-progress.md:191`) |
| Whole-crate mutation for archive | not run (archive must request `cargo mutants -p gafira-color`) |

**Change-complete verdict: FAIL.** Fix CRITICAL 1 on the PR2 branch and re-verify PR2. Then the owner confirms the `Result` deviation (task 0.3). Then run the whole-crate mutation. Then archive in the order C1, domain-color, domain-doc.

## Result

PR2: FAIL (CRITICAL 1, WARNING 4, SUGGESTION 4). Change complete: FAIL. `next`: tdd-apply (drop the `inverse` exclude entry and add the geom overflow test), then verify, then the owner decision on task 0.3, then archive.
