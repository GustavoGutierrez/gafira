# Verify report: gafira-raster, RS3a, round 2

- Commit: `6ccebeb` (branch `feat/gafira-raster`). Fix commits: `219ffb7`, `9d54263`, `2f0480f`; P15 amended by `architect` in `1d5cc3f`.
- Closure round, depth V2, mode full, tier T4 at close, `mutation: archive`, so no `mutants` report (not a finding). T4 is still owed at close.
- This report replaces RS3a round 1, which stays in git at `107f9d9`. RS2 round 2 stays at `1909734`, RS2 round 1 at `0b563fe`, RS1 round 2 at `e5e1d0e`. All rounds are also in `verification.json`.

**Verdict: PASS for RS3a; RS3b and later deliveries remain.** 0 CRITICAL, 1 WARNING (follow-up), 5 SUGGESTION.
Per class: implementation 1 (1 WARNING, follow-up), ambiguity 0, architecture 0, claim 0.

All 8 findings of round 1 are closed. Every reapplied mutant dies. One new test gap, as a follow-up.

## Gates

Each exit code was read alone, on a clean clone of `6ccebeb`, with no `proptest-regressions` folder.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` (1 422 passed, 0 failed, 0 ignored; 104 in the `gafira-raster` library) | 0 |
| `cargo deny check` | 0 |
| policy: ARQ-07 CLI | 0 |
| policy: forbid unsafe | 0 |
| policy: V5 layers | 0 |
| policy: V12 clippy copies | 0 |
| policy: ARQ-107 language | 0 |
| policy: V13 EditToken | 0 |
| policy: V14 property tests | 0 |

The 15 seeds in the untracked `proptest-regressions/` also pass on the unmutated code, on a scratch copy. The CI test job's Windows and macOS targets were not run here. Disk: 36 GB free at the start, 32 GB at the lowest reading, 36 GB after `cargo clean` of all five build folders.

## Artifact integrity (step 3c)

- `dc6fced..6ccebeb`: `apply-progress.md` changes at lines 72, 159, 169 and 190 (`02:1308` becomes `02 §7.3`, S5) and gains the fix-round sections. The Slice card, lines 1 to 8, is untouched.
- `specs/02-arquitectura-software.md`, `.cargo/`, `Cargo.lock` and `deny.toml`: no change. `specs/changes/layer-2/design.md`: only `1d5cc3f`, the allowed `architect` amendment, which comes before the fix commits.
- No exclusion was added to `.cargo/mutants.toml`.

## Closure

| Finding | Status | Evidence |
|---|---|---|
| C1 | closed | `fade.rs:127`, `:145`: the conversion for the format in use is checked before any write. Probe, public API: 11 colours (huge finite, mixed sign, alpha 0, all three spaces) at 7 `progress` values, with extreme pixels. 308 `f32` cases: 168 `Err` naming `color` with `dst` untouched, 140 `Ok` and finite. 77 byte cases: never a bad write. At 0, 1 and clipped values the copy stays `Ok` |
| C2 | closed | I1, I2, I3 killed, see below. `scale` before `progress` is in the `cx.scale` test |
| C3 | closed | I4, I4b killed through the public API. The false equivalent claim is withdrawn in the record |
| W1 | closed | `benches/transitions.rs` builds and runs: 159.85 to 159.91 ms per frame here (5 prior runs of round 1: 154 to 156). XP13 and O5 carry the cost for the owner |
| W2 | closed | I5, I5b killed |
| W3 | closed | `mix.rs:114`: `Encoded` works in `f64`. In-crate probe, 584 647 mixes of 17 extreme values (`±MAX`, subnormals, tiny and negative alphas) at 7 weights: all finite. The property covers all three spaces |
| W4 | closed | Amended P15 steps 1 to 8. See the ruling below |
| W5 | closed | Replay on exports of the stated stubs: `54d8e7b` gives 3 failing, `effect_error_displays_for_the_log` included; `09847aa` with `Dissolve::handles` `None` fails `dissolve_has_the_identity_and_schema_of_the_table`. The height-alone and ARQ-111 entries are corrected. Lines 177 and 200 keep the old wording, and line 244 supersedes them |

Also closed: S1 (the comment says why), S5 (no `02:1308` left under `crates/`). S4 is settled and was not reopened. For the record, a test pins the code at `mod.rs:234`: `from` `Alpha8` is `UnsupportedFormat` whatever the others are; a mix under a supported `from` is `FormatMismatch`.

## Mutants reapplied

No independent budget spent. Each on a scratch export, `gafira-raster` library tests, 1 of 104 failing.

| # | Mutant | Killed by |
|---|---|---|
| I1 | `progress` after `resolve` | `p15_order_of_the_checks_with_two_faults_at_once` |
| I2 | `resolve` after dimensions | same |
| I3 | Pixels before dimensions | same |
| I4 | Fade threshold 0.4999 | `arq_118_fade_boundary_is_exactly_one_half` |
| I4b | Fade threshold 0.5001 | same |
| I5 | `colour_f32` alpha clamp to 2 | `arq_118_fade_colour_alpha_is_clipped_in_f32` |
| I5b | `colour_f32` no lower clip | same |

## Rulings

- **ARQ-31: clean.** No `static`, `OnceLock`, `lazy_static!` or `inventory::` in the crate, tests included; the only `static` matches are `'static` lifetimes. One note: ARQ-31 bans mutable global state for registries, so the old read-only table was probably within the rule. The commit body reads it more strictly than the row does. The result is still fine.
- **Second sRGB curve in `Transfer`: accepted while dormant, with W6.** It is not a defect. `gafira-color`'s `math::encode` and `decode` are `pub(crate)`, `AnalyticTransform` is `f32` only, and that is what overflowed in W3, so sharing needs a new layer-0 API, out of this delivery. No spec row forbids the copy. The same constants and the same sign rule; a sweep of about 65 000 values in `[-8, 8]` matches `AnalyticTransform` exactly after the cast to `f32`. Only the `Encoded` arm calls it, and `MIX_SPACE` does not select that arm. If O5 goes to (b), this becomes production code and the curve should come from `gafira-color`.
- **`Transfer::new()` per `run_cpu`: accepted.** Measured in `--release`, 5 runs of 20 000: 10.4 µs, 0.13 % of the 8 ms share and under 0.01 % of today's 160 ms frame. It is also spent on `RgbaF32`, which does not use it under (a); the waste is too small to matter. This holds for whole-frame calls, which is the contract. A caller that splits a frame into many calls would have to look again.
- **W4's `Err` field: correct.** `Buffer(NonFinite { field: "cx.scale", index: 0 })` matches P15 to the letter for all three transitions, NaN and both infinities, before every other check, with `dst` untouched. It reaches `error.raster.non_finite` with `field` `cx.scale` and `index` 0. A finite `scale` of any sign or size is no error, as P15 says.

C1 note: `Srgb(1e30, ..)` is `Err` on `RgbaF32` and `Ok` with clamped bytes on `Rgba8`, because only the conversion in use is built, and sRGB to sRGB computes nothing. No non-finite value exists on the byte path, so 02 §4.17 "Rangos" leaves the finite result unspecified. No finding.

## Criteria

16 of 16 covered. The four open in round 1 (Fade boundary, ARQ-110 computed non-finite, ARQ-110 `cx.scale`, P15 order) now have tests, proved above. P15 now has 8 steps. Left out on purpose: Slide, Zoom, Wipe (`RS3b`); GPU variants; `TransitionRegistry`; `extension-demo`.

## Findings

### CRITICAL

None.

### WARNING

- **W6. No test ties the second sRGB curve to `gafira-color`.** class: implementation, `follow-up`. `crates/gafira-raster/src/transfer.rs:46`, `:57`. The curves match today, but a change on either side would pass. New test gap: a row comparing `encode_f64` and `decode_f64` against `AnalyticTransform` over a sweep is enough.

### SUGGESTION

- S2. Still open: the Slice card's Deliveries line and IDs say `RS3`. It is the coordinator's card.
- S3. Still open: `proptest-regressions/` is untracked, now with 15 seeds.
- S6. `transfer.rs:1` says the curve goes "through the layer-0 transform"; `encode_f64` and `decode_f64` do not.
- S7. `benches/transitions.rs:3` says the nightly job reads the figure. ARQ-40 specifies that job, but it does not exist yet, and neither does `perf-budgets.toml`. Say "will be". Also, the XP13 row in `design.md` still says `RS3a` owes the bench; it does not now.
- S8. `6ccebeb` is a `docs` commit about the change folder and also edits a test comment in `error.rs`. Harmless, but it belongs with the S5 fix.

## Size

2 498 lines added in `crates/gafira-raster` against `baf5e59`, 594 of them in this fix round, against a forecast of 420 (590). Noted, no finding.

next: tdd-apply (RS3b; W6 is a follow-up, no routing)
