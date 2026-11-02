# Verify report: gafira-audio, AU1, round 1

| Field | Value |
|---|---|
| Commit | `e1afac9`, branch `feat/gafira-audio`, base `9124700` |
| Depth, mode, tier | V2, full, T4 at close; `mutation: archive`, no `mutants` report (not a finding) |
| Verdict | **FAIL** |
| Counts | 4 CRITICAL, 6 WARNING, 2 SUGGESTION |
| Per class (CRITICAL + WARNING) | implementation 5, ambiguity 5, architecture 0, claim 0 |

## Gates

Each command ran alone, on a clean tree at `e1afac9`, in the worktree's own `target`.

| Gate | Exit code |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1 345 passed, 0 failed, 0 ignored) |
| `cargo deny --locked check` | 0 |
| ARQ-07 | 0 |
| Forbid unsafe | 0 |
| V5 | 0 |
| V12 | 0 |
| ARQ-107 | 0 |
| V13 | 0 |
| V14 | 0 |

The policy steps were extracted from `.github/workflows/ci.yml` and run unchanged. Disk before the first build: 20.7 GB (19.3 GiB), which `df -h` shows as 20G. It rose to 23.7 GB before the builds, so the threshold was not crossed.

## Slice card integrity (step 3c)

- `9bfba1c` touches only `apply-progress.md`, and it precedes the first implementation commit.
- `git diff 9bfba1c HEAD` on that file has no removed line, so the card is unchanged.
- Only `crates/gafira-audio/` and that file changed.

## CRITICAL

### C1. An empty `ProcessorChain` breaks ARQ-123

- class: implementation
- ref: `crates/gafira-audio/src/chain.rs:55`, `crates/gafira-audio/tests/chain.rs:83`
- `ProcessorChain` is an `AudioProcessor`. ARQ-123 says "Ninguna salida es no finita ni subnormal", and its criterion is "por procesador".
- Probe `p01`, prepared at 48 kHz with no processors: NaN, `±∞` and subnormals come out unchanged.
- `arq_129_an_empty_chain_changes_nothing` asserts `is_nan()`, so it pins the defect as behaviour. `makers()` in `tests/total.rs` only builds a non-empty chain. The deviations list does not mention this.
- The fix needs the chain to know its `Setup`, because a 0 in `prepare` or an oversized block must still stay intact.

### C2. Hand mutant H1 survives: a stereo fade runs at the wrong speed

- class: implementation
- ref: `crates/gafira-audio/src/fade.rs:84`, `crates/gafira-audio/tests/fade.rs`
- Mutation: `position` advances once per channel sample instead of once per frame. A stereo fade then finishes in half its duration.
- All 11 test binaries pass with it. Every test in `fade.rs` is mono, and the stereo tests elsewhere never check the fade's slope.
- ARQ-124 fixes the slope at 1/96 000 + 1e-6 per sample for 2 s at 48 kHz. That is a spec rule with an invariant, so this is critical code under `AGENTS.md`.
- Independent probe `k1`: passes on the original; on the mutant the slope is 2.08e-5, about 2 × the limit.
- The code at HEAD is correct; the gap is in the tests.

### C3. Hand mutant H2 survives: a gain change during a ramp jumps

- class: implementation
- ref: `crates/gafira-audio/src/gain.rs:110`, `crates/gafira-audio/tests/gain.rs`
- Mutation: the step is computed from the old target instead of from `current`.
- All 11 test binaries pass with it. No test changes the gain while a ramp is running; each lets the ramp finish first.
- ARQ-124 requires that a gain change applies with a linear 10 ms ramp, at most 1/480 + 1e-6 per sample for 0 to 1.
- Independent probe `k2`: passes on the original; on the mutant one sample jumps 0.498, about 239 × the limit.
- The code at HEAD is correct; the gap is in the tests.

### C4. `mix` contradicts ARQ-110, and the spec does not settle it

- class: ambiguity
- ref: `crates/gafira-audio/src/mix.rs:20`, ARQ-110, ARQ-123, 02 §4.17.5
- ARQ-110 applies to every public function of a layer-2 engine and lists its exceptions: `AudioProcessor::process` and four named functions. `mix` is not among them.
- Its acceptance test requires `Err` on NaN and `±∞`, with the destination buffer identical byte for byte. `mix` returns `()`, counts those samples as 0 and overwrites `out`.
- Against that, the 02 §4.17.5 row "Desbordamiento de `f32` en la mezcla" says the output stays finite and cites ARQ-123, which suggests `mix` follows the `process` rules. The rows for NaN name only functions with `Result` and `process`; `mix` is neither.
- The two readings give different public signatures, and AU2 (`bench_mix_16`) builds on this one. `apply-progress.md` does not record the choice.
- Decision needed: add `mix` to the ARQ-110 exceptions, or give it a `Result`. The current behaviour is safe (probe `p14`), so the first option changes only the spec.

## WARNING

### W1. The first gain change ramps from 1.0

- class: ambiguity
- ref: `crates/gafira-audio/src/gain.rs:70`, ARQ-124
- ARQ-124 does not say whether the first value is a change. The implementer disclosed the choice.
- Probe `p03`: a track at volume 0 % starts at 0.998 and takes 480 samples to reach 0, so a silent track emits a 10 ms burst at the start of playback.

### W2. Parameter ids are not in any spec

- class: ambiguity
- ref: `crates/gafira-audio/src/gain.rs:11`, `fade.rs:10`, `pan.rs:13`
- `volume`, `duration` and `pan` are the implementer's names. Design P5 calls them ids but no `ParamSchema` defines them, and `Descriptor` has no parameter list.
- `gafira-media` and the UI will depend on these names.

### W3. The `Fade` clock and a single `duration` are unspecified

- class: ambiguity
- ref: `crates/gafira-audio/src/fade.rs:71`, ARQ-124, FV-10
- No spec says where a fade starts. `Fade` counts from `prepare` or `reset`.
- Probe `p05`: the clock runs at duration 0, so a fade-out set to 2 s after 1 s starts at 0.5. Shortening the duration mid-fade jumps from 0.49999 to 1.
- Probe `p25`: FV-10 has separate in and out fades, but both read one `duration` from the shared snapshot and share the reset. In one chain they multiply: peak 0.25, and silence at the end of the fade.
- The implementer's note leaves scheduling to `gafira-media`, which has no spec row for it.

### W4. Unequal channels stay intact, and the channel count is not checked

- class: ambiguity
- ref: `crates/gafira-audio/src/dsp.rs:18`, design P5, ARQ-123
- P5 states "equal lengths" but not what happens otherwise. Leaving the block intact matches the ARQ-123 pattern, but that rule's list of intact cases does not include this one, and probe `p04` shows NaN leaving `process`.
- `accepts` also ignores a block whose channel count differs from `prepare`.

### W5. `ProcessorChain::tail()` always returns 0

- class: implementation
- ref: `crates/gafira-audio/src/chain.rs:48`, 02 §4.9
- The default `tail` is allowed by the trait in 02, and ARQ-129 aggregates only `latency`. Aggregating `tail` is derivable from the crate's own doc comment on the method.
- Probe `p07`: children with tails 7 and 11 give a chain tail of 0. `push` accepts any processor, and `Eq3Band` arrives in AU2. Fix it with C1 or before AU2.

### W6. ARQ-111 does not cover the pure functions

- class: implementation
- ref: `crates/gafira-audio/tests/`, ARQ-111
- ARQ-111 requires running each operation twice and comparing bytes. `db_to_gain`, `percent_to_gain`, `crossfade_gains` and `flicks_to_samples` have no such test. They are pure, so the risk is low.

## SUGGESTION

- S1. FV-10 (a) says a fade "reduce la ganancia de 0 a 1", which reads backwards. `specifier` can fix the wording when it handles C4.
- S2. A mono track in a stereo `mix` adds only to the left channel. No spec covers it; state it when `gafira-media` needs it.

## Implementer assumptions

| Assumption | Classification | Result |
|---|---|---|
| Parameter units: percent, seconds, `[-1, 1]` | Stated (FV-10, ARQ-124) | No finding |
| Parameter ids | Ambiguity | W2 |
| Saturation at `±f32::MAX` | Finite output is stated (ARQ-123, 02 §4.17.5); the value is derivable, since `±1` is banned and overflow must stay finite | No finding. Sign proved by H3 |
| Unequal channel lengths | Ambiguity | W4 |
| First gain change ramps from 1.0 | Ambiguity | W1 |
| `Fade` counts from `prepare` or `reset` | Ambiguity | W3 |
| Ramp as `sample_rate / 100` in integers | Derivable | No finding. Exact at 8, 44.1, 48, 88.2, 96 and 192 kHz; 22.05 kHz is 0.5 samples short and 11.025 kHz 0.25. Below 200 Hz it clamps or floors to 1 sample |
| `tail()` not overridden | Signature stated; aggregation derivable | W5 |

Not in the implementer's list, found here: the empty chain (C1) and `mix` against ARQ-110 (C4).

## Allocation in `process`

No allocation path in AU1 code.

- Reading: `Gain`, `Fade`, `Pan`, `ProcessorChain::process`, `mix` and `ParamSnapshot::get` hold no `Vec`, `Box`, `String`, format or lock. `to_string` is only in `AudioError::to_message`, and `Vec` growth only in `ProcessorChain::push`.
- Run: probe `p08` ran a three-processor chain, `reset`, hostile samples and a 16-track `mix` under `assert_no_alloc` in a debug build, and it passed. Its negative control aborted with "memory allocation of 1024 bytes failed", so the guard was live.
- This does not replace the AU2 guard test (ARQ-23).

## Acceptance map

| Criterion | State | Proof |
|---|---|---|
| ARQ-123 | **Failing** | C1. Otherwise `tests/total.rs`, probes `p02`, `p15`, `p23` |
| ARQ-124 | Code correct, proof weak | `tests/gain.rs`, `fade.rs`, `pan.rs`; gaps C2, C3 |
| ARQ-129 | Covered | `tests/chain.rs`, `mix.rs`, `offset.rs`; probes `p09`, `p14` |
| FV-10 (a) | Covered, mono only | `fv_10_a_two_second_fade_in_rises_from_zero_to_one_continuously`, probe `p11`; stereo gap C2 |
| FV-10 (d) | Covered | `fv_10_d_one_millisecond_is_44_samples_at_44_1_khz_and_48_at_48_khz`, probe `p09` |
| ARQ-110 | **Open** | Three functions covered; `mix` is C4 |
| ARQ-111 | Partial | Processors and `mix` covered, probe `p18`; gap W6 |

Covered 3 of 7. ADR 0005 and design P5: `AudioBlock`, `ParamSnapshot` match. ADR 0012: `error.audio.non_finite`, exhaustive match, `field` in `Text`, no `Cancelled` variant (D-43). D-22: `Pan` done; `Eq3Band` is AU2. D-21: AU3, no AU1 code. The AU2 to AU4 IDs of the card are out of this delivery.

## Hand mutants

| # | Mutation | Result |
|---|---|---|
| Spot-check, item 7 | No end hold (`.min(1.0)` removed) | Killed by the two recorded tests |
| H1 | Fade clock per channel sample | **Survived**, C2 |
| H2 | Step from the old target | **Survived**, C3 |
| H3 | Saturation loses its sign | Killed by `arq_123_there_is_no_clipping_at_zero_dbfs`, `arq_123_the_mix_stays_finite_when_the_sum_overflows_f32` |

First versions of H1 and H2 were discarded: they failed tests only because they were badly built (H1 also doubled the mono clock; H2 reduced to "no ramp"). The table shows the corrected versions.

## Probes

19 probes in `vp.rs`, 1 in `vp_alloc.rs` and 3 more (`p25`, `k1`, `k2`), written before reading the tests, except `k1` and `k2`, which were written after to prove the survivors. One failed on the code: `p01` (C1). `p18` failed first through a wrong frame count in the probe itself and passes when fixed.

## Size

2 094 insertions and 1 deletion in 21 files, 714 in `src/`; the prompt said 2 085. Over the ~700 line signal and the 670 forecast. The owner said size does not concern him: noted, no finding.

next: specifier (ambiguity: C4, W1 to W4), then tdd-apply (implementation: C1 to C3, W5, W6)


---

# Verify report: gafira-audio, AU1, round 2 (closure)

Round 1 stays above, unchanged.

| Field | Value |
|---|---|
| Commit | `d9581ba`, branch `feat/gafira-audio`, amended specs merged at `a96494a` |
| Depth, mode, tier | V2, full, T4 at close; `mutation: archive`, no `mutants` report (not a finding) |
| Verdict | **FAIL** |
| Counts | 1 CRITICAL, 3 WARNING, 2 SUGGESTION, all new; C1 to C4 and W1 to W6 are closed |
| Per class (CRITICAL + WARNING) | implementation 2, ambiguity 2, architecture 0, claim 0 |

## Gates

Each command ran alone and its exit code was read directly, on a clean tree at `d9581ba`. Free disk was 30 GB at the start and never below 23 GB.

| Gate | Exit code |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1 367 passed, 0 failed, 0 ignored) |
| `cargo deny --locked check` | 0 |
| ARQ-07 | 0 |
| Forbid unsafe | 0 |
| V5 | 0 |
| V12 | 0 |
| ARQ-107 | 0 |
| V13 | 0 |
| V14 | 0 |

The policy steps were extracted from `.github/workflows/ci.yml` and run unchanged with `bash --noprofile --norc -eo pipefail`.

## Artifact integrity (step 3c)

- `9bfba1c` touches only `apply-progress.md` and precedes the first implementation commit, `17cdde0`.
- `git diff 9bfba1c HEAD` on that file removes no line, so the card is unchanged.
- The fix commits touch only `crates/gafira-audio/`, plus `apply-progress.md` in `d9581ba`.
- The spec amendment commits touch no file under `crates/`.
- `.cargo/` is unchanged, so no exclusion needs checking.
- `params()` is not in the card's ID list. It reaches AU1 through the design table and P13, so this is not a finding.

## Closure of round 1

| Finding | State | Evidence |
|---|---|---|
| C1 | Closed | An empty chain cleans the block. Removing the clean step is killed by 8 tests, among them `arq_123_an_empty_chain_cleans_the_block_and_keeps_normal_samples` |
| C2 | Closed | H1 is killed by `fv_10_a_stereo_fade_runs_at_the_same_speed_on_both_channels`. Probe: stereo fade equals `n / L` exactly, In and Out, blocks of 512 and 7 |
| C3 | Closed | H2 is killed by `arq_124_a_change_during_a_ramp_continues_from_the_current_gain`. Probe: largest step on a retarget is 0.00208336, within 1/480 + 1e-6 |
| C4 | Closed | ARQ-110 names `mix` as an exception and ARQ-129 makes it total. Probe: NaN and `±∞` give the sum of the others, two tracks at `f32::MAX` stay finite, 2 channels onto 3 feed the first two |
| W1 | Closed | ARQ-124 settles the first value. Probe: volume 0 and 50 from sample 0, also after `reset`; no value then 0 ramps. An empty first block is W8 |
| W2 | Closed | The 02 §4.17.5 table states ids, ranges, defaults, labels and `animatable`; `params()` matches it through `dyn AudioProcessor`. Flipping `animatable` on `Fade` is killed |
| W3 | Closed | ARQ-124 states the clock and separate instances. Re-reading the length each block is killed by 2 tests. Probe: 0 to 2 s without `reset` changes no sample |
| W4 | Closed | ARQ-123 states both block shapes. Removing the channel count check is killed by 2 tests. The state rule has a new gap, C5 |
| W5 | Closed | `tail()` gives 18 for 7 and 11. `max` and wrapping are killed by `arq_129_the_tail_of_a_chain_is_the_sum_and_saturates`. See the ruling below |
| W6 | Closed | `tests/determinism.rs` covers the four pure functions |

S1 is closed by the new FV-10 wording. S2 is closed by ARQ-129: a track with fewer channels adds what it has.

## CRITICAL

### C5. An empty block after a `prepare` with a 0 changes the state

- class: implementation
- ref: `crates/gafira-audio/src/dsp.rs:23`, ARQ-123, `crates/gafira-audio/tests/total.rs:223`
- ARQ-123 says that if `prepare` got a 0, `process` does not process and its state does not change.
- `accepts` does not refuse an empty block after `prepare` with `max_block` 0 (`0 <= 0`), or a block with no channels after `prepare` with `channels` 0 (`0 == 0`).
- `process` then reads the parameter. `prepare` keeps the last valid value, so the state survives into the next valid stream.
- Probe `q3`: `prepare(48 000, 0, 2)`, an empty block with the parameter, `prepare(48 000, 64, 2)`, then a block with no parameter. The rows with `channels` 0 and with both at 0 give the same results.

| Processor | Parameter in the empty block | Output | Control |
|---|---|---|---|
| `Gain` | `volume` 50 | 0.5 | 1.0 |
| `Fade` In | `duration` 0.001 | ramp from 0 | 1.0 |
| `Pan` | `pan` −1 | right channel 0 | 1.0 |

- `sample_rate` 0 is refused correctly in the same probe.
- The two clauses were removed as equivalent or unreachable in `apply-progress.md` (round 0 and the fix round). This probe shows they were not equivalent. The doc comment on `accepts` covers only non-empty blocks.
- `arq_123_an_unprocessed_block_leaves_no_trace_in_the_state` skips the three `prepare` rows (`.skip(3)`), and its blocks are not empty.
- The reach is narrow: it needs a second `prepare`. It is still a stated rule on critical code, so it counts as a real defect.
- Tried on the scratch export only: restoring `max_block != 0 && channels != 0` makes all 9 `q3` rows match and the crate's tests pass. Refusing `frames() == 0` does the same and also resolves W8, if `specifier` chooses that reading.

## WARNING

### W7. "Se lee en el primer bloque" does not say which value a later `reset` uses

- class: ambiguity
- ref: `crates/gafira-audio/src/fade.rs:83`, ARQ-124
- ARQ-124 has two sentences that differ in one case: a valid value arrives mid-ramp, and the first block after `reset` has none, or a non-finite one.
- See the ruling below. The code stays; the wording should change.

### W8. Whether an empty block is the "first block" is unstated

- class: ambiguity
- ref: `crates/gafira-audio/src/dsp.rs:23`, ARQ-124, 02 §4.17.5 row "Tamaño 0"
- On a valid `prepare`, an empty block is accepted, consumes `first` and latches the length. The row "Bloque vacío" speaks only of samples.
- Probe `q4`: an empty block with no value, then `volume` 0, gives 0.9979 and ramps, so the burst of W1 returns.
- Probe `q4`: an empty block with no value, then `duration` 2, gives no fade at all until `reset`.
- ARQ-124 says the first value rules "desde la muestra 0", and an empty block has no sample 0. Both readings are defensible, so the spec must choose.

### W9. No test pins the mid-ramp read of `duration`

- class: implementation
- follow-up
- ref: `crates/gafira-audio/tests/fade.rs`
- Mutant R1, which reads `duration` only at the latch, survives all tests. Add the test once W7 is settled.

## Rulings

### `ProcessorChain::tail()`: saturating sum, keep it

- Stages run in series. The second stage's input ends only when the first stage's tail ends, so the tails add. They overlap only for parallel stages.
- Probe `q1`, with real FIR stages that ring exactly their declared tail, on an impulse:

| Tails | `tail()` | Last non-zero sample | `max` would give |
|---|---|---|---|
| 7, 11 | 18 | 18 (value 0.25) | 11 |
| 11, 7 | 18 | 18 | 11 |
| 7, 11, 3 | 21 | 21 | 11 |

- `max` would cut real output, 7 samples in the first row. The sum is exact here and an upper bound for other stages.
- A plain sum fails on overflow: it panics in debug and wraps in release to a tail that is too short. Saturating keeps the upper bound and matches `latency()`.
- No spec states this rule; see S3.

### `Fade`: reading `duration` every block and latching at the start, keep it

- The latch is required: ARQ-124 forbids any change in the samples until the next `reset`. Mutant "re-read the length" is killed.
- The read on every accepted block meets every acceptance criterion. Probe `q2`: the mid-ramp sample does not change.
- It is also the only reading under which "un cambio durante la rampa rige en el siguiente `reset`" holds when the first block after `reset` has no value. Probe `q2`: 0.02 s, then 0.005 s mid-ramp, then `reset` and a block with no value or NaN gives slope 1/240. Reading only at the latch would drop the change and give 1/960.
- It also fits ARQ-123, "queda el último válido". A refused block reads nothing, so ARQ-123 holds there too, apart from C5.
- The literal "se lee en el primer bloque" points the other way, so W7 asks `specifier` to reword it, for example: the length is fixed in the first block from the last valid value.

## SUGGESTION

- S3. ARQ-129 states only `latency()`. Add that `tail()` is the saturating sum of the stages, in the same row.
- S4. Stale doc comments: `ProcessorChain::new` says "it changes nothing" (`chain.rs:20`) and `Pan` says "any other block is not changed" (`pan.rs:33`). Both blocks are now cleaned.

## Acceptance map

| Criterion | State | Proof |
|---|---|---|
| ARQ-123 | **Failing** | C5. Otherwise `tests/total.rs`, `chain.rs`, `fade.rs` |
| ARQ-124 | Covered | `tests/gain.rs`, `fade.rs`, `pan.rs`, probes `q2`, `q5`; open wording W7, W8 |
| ARQ-129 | Covered | `tests/chain.rs`, `mix.rs`, `offset.rs`, probe `q5` |
| FV-10 (a) | Covered | `fv_10_a_two_second_fade_in_rises_from_zero_to_one_continuously`, `fv_10_a_stereo_fade_runs_at_the_same_speed_on_both_channels`, `fv_10_the_fade_in_and_the_fade_out_have_independent_durations` |
| FV-10 (d) | Covered | `fv_10_d_one_millisecond_is_44_samples_at_44_1_khz_and_48_at_48_khz` |
| ARQ-110 | Covered | `arq_110_*` tests; `mix` is an exception, C4 closed |
| ARQ-111 | Covered | `arq_111_*` tests, `tests/determinism.rs` |
| P13, 02 §4.17.5 table | Covered | `tests/params.rs`; `duration` at `f64::MAX` stays finite, probe `q5` |

Covered 7 of 8. The AU2 to AU4 IDs of the card are out of this delivery.

## Mutants reapplied

This is a closure round, so there is no independent budget. These repeat the report's list and check closure. All ran on a scratch export of `d9581ba`.

| Mutation | Result |
|---|---|
| Spot-check, item 7: no end hold | Killed by 2 tests |
| H1 fade clock per channel sample | Killed by 1 test |
| H2 step from the old target | Killed by 1 test |
| H3 saturation loses its sign | Killed by 2 tests |
| Empty chain not cleaned | Killed by 8 tests |
| First value ramps | Killed by 8 tests |
| Duration re-read each block | Killed by 2 tests |
| Channel count unchecked | Killed by 2 tests |
| Tail `max` | Killed by 1 test |
| Tail wrapping | Killed by 1 test |
| `Fade` `animatable` flipped | Killed by 1 test |
| R1 duration read only at the latch | **Survived**, W9 |

## Route

- This is the second consecutive FAIL with `implementation` and with `ambiguity` findings, so the coordinator's stop for the owner applies. One round is left under the cap of three.
- W7 and W8 go first because the answer to W8 decides the shape of the C5 fix.
- Size: 2 643 insertions under `crates/` since the card, against a 670 forecast. The owner said size does not concern him: noted, no finding.

next: specifier (ambiguity: W7, W8), then tdd-apply (implementation: C5, W9 as follow-up)


---

# Verify report: gafira-audio, AU1, round 3 (closure)

Rounds 1 and 2 stay above, unchanged.

| Field | Value |
|---|---|
| Commit | `a605fe7`, branch `feat/gafira-audio`, base `develop`; amended specs `ac411a3`, fix `1d54942` |
| Depth, mode, tier | V2, full, T4 at close; `mutation: archive`, no `mutants` report (not a finding) |
| Verdict | **PASS** for delivery AU1; AU2 to AU4 remain, so `next` is `tdd-apply` |
| Counts | 0 CRITICAL, 4 WARNING (all new, all follow-up), 2 SUGGESTION (S3, S4 carried over) |
| Per class (CRITICAL + WARNING) | implementation 4 (all follow-up), ambiguity 0, architecture 0, claim 0 |

## Gates

Each command ran alone and its exit code was read directly, on a clean tree at `a605fe7`, with no `CARGO_TARGET_DIR`. Free disk was 28 GB at the start and never below 24 GB.

| Gate | Exit code |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1 374 passed, 0 failed, 0 ignored) |
| `cargo deny --locked check` | 0 |
| ARQ-07 | 0 |
| Forbid unsafe | 0 |
| V5 | 0 |
| V12 | 0 |
| ARQ-107 | 0 |
| V13 | 0 |
| V14 | 0 |

The policy steps were extracted from `.github/workflows/ci.yml` and run unchanged with `bash --noprofile --norc -eo pipefail`.

## Artifact integrity (step 3c)

- `9bfba1c` touches only `apply-progress.md` and precedes the first implementation commit.
- `git diff 9bfba1c HEAD` on that file removes no line, so the card is unchanged.
- `ac411a3` touches only `specs/02-arquitectura-software.md`, the ARQ-123 and ARQ-124 rows.
- `1d54942` touches only `crates/gafira-audio/src/dsp.rs` and `tests/empty.rs`.
- `a605fe7` adds 31 lines to `apply-progress.md` and touches nothing else.
- `.cargo/` is unchanged, so no exclusion needs checking.

## Closure of round 2

| Finding | State | Evidence |
|---|---|---|
| C5 | Closed | `accepts` requires `frames() > 0`. Independent probe `q3`: Gain, both fades, Pan, a full chain and an empty chain, with `prepare` at `max_block` 0, `channels` 0 and both, and 0 to 3 empty channels, all equal a control bit for bit. Control mutant E0 is killed by 5 tests |
| W7 | Closed | ARQ-124 now says the first processed block fixes `L` with the last valid value, counting its own, and a later value becomes the last valid. The code matches. Probe `w7`: 1/240, 1/960 and 1/1920, In and Out, blocks of 1, 7 and 500 |
| W8 | Closed | ARQ-123 and ARQ-124 say an empty block is not processed and is not the first block. Probe `q4` passes, and `anywhere` shows an empty block at any point of a stream changes no later sample |
| W9 | Closed | Mutant R1 of round 2 (D2 below) is killed by `arq_124_a_duration_in_a_processed_block_mid_ramp_becomes_the_last_valid_one` |

C1 to C4 and W1 to W6 stayed closed in round 2; the gates and the mutants below touch nothing that reopens them.

The doc item on row `Tamaño 0` of 02 §4.17.5 is the recorded follow-up the coordinator named, so it is not routed here. The row is incomplete, not contradictory.

## Criteria of `ac411a3`

The coordinator asked for each new criterion to be proved by mutation, so this closure round ran more mutants than the usual closure list. All ran on a scratch export of `a605fe7`, with the probe files removed, so a kill can only come from the repo's tests. The export used its own `target` and `CARGO_INCREMENTAL=0`. Every mutated file was restored and compared to the commit.

Every explicit acceptance sentence added by `ac411a3` has a test that fails when that sentence alone is broken.

| New criterion | Narrowest mutant | Killed by |
|---|---|---|
| ARQ-123 empty block, valid `prepare`, 2 shapes | E1, E6, N5 | `arq_123_an_empty_block_after_a_valid_prepare_leaves_the_next_block_untouched` |
| ARQ-123 empty block, `max_block` 0, `channels` 0, both | E2, E3, E4, E5 | `arq_123_an_empty_block_after_a_prepare_with_a_zero_leaves_a_later_prepare_untouched` |
| ARQ-124 empty without `volume`, then 0 | F1 | `arq_124_an_empty_block_is_not_the_first_block_of_a_gain` |
| ARQ-124 empty with 0, then no `volume` | N1 | 3 tests, among them the one above |
| ARQ-124 `volume` 100, empty with 0, no `volume` | N2 | `arq_124_an_empty_block_is_not_the_first_block_of_a_gain` |
| ARQ-124 empty without `duration`, then 2 s | L1 | `arq_124_an_empty_block_does_not_latch_the_fade_length` |
| ARQ-124 1/240, no `duration` and NaN | D2, D6, N6 | `arq_124_a_duration_in_a_processed_block_mid_ramp_becomes_the_last_valid_one` |
| ARQ-124 1/960 | N3 | `arq_124_a_duration_in_an_empty_block_mid_ramp_is_not_read` |
| ARQ-124 1/1920 | N4 | `arq_124_the_block_after_reset_sets_the_length_with_its_own_duration` |
| Prose: no parameter read, Gain, Fade, Pan, every block | R1, R2, R3 | 1 to 5 tests each |

The gaps below are in prose rules that have no acceptance sentence of their own, or in an older criterion. The code is correct in all of them: each survivor was killed only by a probe that passes on HEAD, which proves the mutant was built and is not equivalent.

## WARNING

All four are test gaps found after round 1, so they are follow-ups and not routed. None is a code defect.

### W10. A refused non-empty block consuming the `Gain` first block is not pinned

- class: implementation
- follow-up
- ref: `crates/gafira-audio/tests/total.rs:214`, ARQ-124
- `ac411a3` says a block `process` does not process is not the first block. The empty case is pinned; the non-empty case is not.
- Mutant F2 sets `first = false` on an oversized block and survives. Control probe: oversized block, then `volume` 0, gives 0 from sample 0.
- The comment at `total.rs:216` claims a consumed first block would show in the valid block. It does not, because that block carries no value.
- Fix: follow the refused block with a block carrying a value.

### W11. The `prepare` rows of the no-trace test stay skipped

- class: implementation
- follow-up
- ref: `crates/gafira-audio/tests/total.rs:223`, ARQ-123
- `.skip(3)` leaves a non-empty block after a `prepare` with a 0 outside the test, and `ac411a3` adds "no lee ningún parámetro".
- Mutant R4, reading only in that case, survives on `Fade` and `Pan`. On `Gain` it is killed, but only by `arq_129_prepare_and_reset_reach_every_processor`, so that is incidental.
- Fix: the same shape as `tests/empty.rs`, with a valid `prepare` before the next block.

### W12. "Su estado no cambia" is pinned only before the first processed block

- class: implementation
- follow-up
- ref: `crates/gafira-audio/tests/empty.rs`, `tests/total.rs:214`, ARQ-123
- No test sends a refused block during a running ramp and then checks the next samples.
- Survivors: F3 empty re-arms `first`; F4 empty ends the gain ramp; F5 the same for a refused non-empty block; L3 empty clears `L`; L4 empty restarts the fade clock; L5 a refused non-empty block clears `L`.
- `arq_124_a_duration_in_an_empty_block_mid_ramp_is_not_read` sends a `reset` right after the empty block, so it cannot see these.
- Fix: compare a mid-ramp stream with and without the refused block, for `Gain` and `Fade`.

### W13. The last valid value across `prepare` is not pinned

- class: implementation
- follow-up
- ref: `crates/gafira-audio/tests/fade.rs:210`, `tests/gain.rs:305`, ARQ-124, ARQ-123
- ARQ-124 says the first block after `prepare` or `reset` uses the last valid value. The tests prove it after `reset` only.
- Survivors: D4 `prepare` drops `duration`; V1 `prepare` drops `volume`; V3 `prepare` drops `pan`.
- Fix: one test per processor with a second `prepare` and a block with no value.

## SUGGESTION

- S3 (from round 2, still open). ARQ-129 states only `latency()`. Add that `tail()` is the saturating sum of the stages.
- S4 (from round 2, still open). Stale doc comments: `ProcessorChain::new` says "it changes nothing" (`chain.rs:20`) and `Pan` says "any other block is not changed" (`pan.rs:33`).

## Acceptance map

| Criterion | State | Proof |
|---|---|---|
| ARQ-123 | Covered | `tests/total.rs`, `chain.rs`, `fade.rs`, `empty.rs`; probes `q3`, `lopsided`, `anywhere`; follow-ups W10 to W12 |
| ARQ-124 | Covered | `tests/gain.rs`, `fade.rs`, `pan.rs`, `empty.rs`; probes `q4`, `w7`, `one_frame`; follow-ups W10, W13 |
| ARQ-129 | Covered | `tests/chain.rs`, `mix.rs`, `offset.rs` |
| FV-10 (a) | Covered | `fv_10_a_two_second_fade_in_rises_from_zero_to_one_continuously`, `fv_10_a_stereo_fade_runs_at_the_same_speed_on_both_channels`, `fv_10_the_fade_in_and_the_fade_out_have_independent_durations` |
| FV-10 (d) | Covered | `fv_10_d_one_millisecond_is_44_samples_at_44_1_khz_and_48_at_48_khz` |
| ARQ-110 | Covered | `arq_110_*` tests |
| ARQ-111 | Covered | `arq_111_*` tests, `tests/determinism.rs` |
| P13, 02 §4.17.5 table | Covered | `tests/params.rs` |

Covered 8 of 8. The AU2 to AU4 IDs of the card are out of this delivery.

## Hand mutants

45 run, 33 killed by the repo's tests, 12 survived. Spot-check item: R1 of round 2 (D2), killed.

| Group | Mutants | Result |
|---|---|---|
| Empty clause in `accepts` | E0 to E7 | 8 killed |
| Parameter read before the guard, per processor | R1, R2, R3 on `Gain`, `Fade`, `Pan`; R4 on `Gain` | 10 killed |
| The same, non-empty block after `prepare` with a 0 | R4 on `Fade`, `Pan` | **2 survived**, W11 |
| `Gain` first value | F1 | Killed |
| `Gain` first value, refused non-empty | F2 | **Survived**, W10 |
| `Gain` mid-stream | F3, F4, F5 | **3 survived**, W12 |
| `Fade` latch on a refused block | L1, L2 | 2 killed |
| `Fade` mid-stream | L3, L4, L5 | **3 survived**, W12 |
| `duration` rules | D1, D2, D3, D5, D6 | 5 killed |
| Last valid across `reset` | V2 | Killed |
| Last valid across `prepare` | D4, V1, V3 | **3 survived**, W13 |
| One per acceptance sentence | N1 to N6 | 6 killed |

E7 (a one-frame block refused) is killed by a single test, `arq_123_a_non_finite_duration_keeps_the_last_valid_one`, and probably by accident. Probe `one_frame` confirms the code.

## Probes

14 probes, 6 in `vr3.rs` and 8 in `vr3b.rs`, on the scratch export. All pass on `a605fe7`, so no code defect showed. Each of the 12 survivors fails at least one probe in `vr3b.rs`.

## Route

- This round has no CRITICAL. The verdict is PASS for the delivery. "All deliveries done" does not apply yet: `next` is the rule for PASS with deliveries remaining.
- The follow-ups W10 to W13 reach `archive`. T4 at change close still runs per the Flow line, and these same mutants are ones a hand list for AU2 could reuse.
- Size: 2 836 insertions under `crates/` since the card, against a 670 forecast; this round added 230 lines in all. The owner said size does not concern him: noted, no finding.

next: tdd-apply (next delivery, AU2)

# Verify report: gafira-audio, AU2, round 1

| Field | Value |
|---|---|
| Commit | `c4946a2`, branch `feat/gafira-audio`, base `develop` `d437b8a` |
| Depth, mode, tier | V2, full, T4 at close; `mutation: archive`, no `mutants` report (not a finding) |
| Verdict | **FAIL** |
| Counts | 3 CRITICAL, 2 WARNING, 4 SUGGESTION |
| Per class (CRITICAL + WARNING) | implementation 4, ambiguity 1, architecture 0, claim 0 |

The production code passed every probe: no code defect was found. All three CRITICAL items are surviving hand mutants, so the missing piece is three tests, given below. They are CRITICAL because step 4b says a survivor in critical code is.

## Gates

Each command ran alone in the worktree, with no `CARGO_TARGET_DIR`. `df -h /` showed 31 GB free at the start and 21 GB at the end.

| Gate | Exit code |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1 577 passed, 0 failed, 0 ignored) |
| `cargo deny --all-features --locked check` | 0 |
| CI policy (ARQ-07, unsafe, V5, V12, ARQ-107, V13, V14) | 0 on each |

## Slice card integrity (step 3c)

- `9bfba1c` (`docs(gafira-audio): slice card`) touches only `apply-progress.md`.
- Lines 1 to 7 of `apply-progress.md` are identical at `d437b8a` and `c4946a2`, so no implementer commit changed the card.
- Files outside `crates/gafira-audio/` are one testkit test (`crates/gafira-testkit/tests/alloc_guard.rs`), which the design assigns to AU2.
- No exclusion was added to `.cargo/mutants.toml`.

## CRITICAL

### C6. Hand mutant M1 survives: NaN enters the biquad history

- class: implementation
- ref: `crates/gafira-audio/src/eq.rs:250`, `crates/gafira-audio/tests/total.rs:74`
- Rule: ARQ-123, a NaN or `±∞` sample counts as 0.
- Mutation: `f64::from(*sample)` in place of `f64::from(clean_input(*sample))`. `cargo test -p gafira-audio` exits 0.
- Root cause: `tests/total.rs` asserts only that the output is finite. Under the mutant the history holds NaN for ever and every later sample comes out as 0, which is finite. The track goes silent for good and no test sees it. The code at HEAD is correct.
- Fix: compare with a run where the bad sample is 0, including the following block. The scratch test `k1` passes at HEAD and kills M1:

```rust
// Same input, once with `bad` at index 100 and once with 0.0, gains [5, -7, 9]:
// both output blocks and the next block must be equal, and the next one non-zero.
```

### C7. Hand mutant M2 survives: `prepare` at another rate keeps the old coefficients

- class: implementation
- ref: `crates/gafira-audio/src/eq.rs:232`, `crates/gafira-audio/tests/refused.rs` (`arq_124_the_last_valid_value_survives_prepare`)
- Rule: ARQ-125, the response is the design at the prepared `fs`, and a band at or above `fs / 2` is omitted.
- Mutation: the `band.design(sample_rate)` loop removed from `prepare`. The suite exits 0.
- Root cause: every test re-prepares at the same rate, or sends the gains again after `prepare`. A processor kept with its gains and re-prepared at another rate is never tested, so a stale filter would go unnoticed. The code at HEAD is correct.
- Fix: after a block with gains at 48 kHz, `prepare(96_000, ..)` and a block with no parameters must equal a fresh 96 kHz run with those gains. Also, `high = 15` then `prepare(16_000, ..)` must pass the signal unchanged. The scratch test `k2` does this and kills M2.

### C8. Hand mutant M3 survives: `reset` forgets the last valid gains

- class: implementation
- ref: `crates/gafira-audio/src/eq.rs:260`, `crates/gafira-audio/src/processor.rs:33`
- Rule: ARQ-123 (the last valid value stays), and the trait contract "parameters and stream shape stay".
- Mutation: `reset` puts each band's `Param` back to its default. The suite exits 0.
- Root cause: W13 pinned the last valid value across `prepare` for every maker, and the Eq `reset` tests send the gains again. Nothing tests a block with no parameters after `reset` on `Eq3Band`. The code at HEAD is correct.
- Fix: run a block with gains, `reset`, run a block with no parameters, and compare with a fresh run with those gains. The scratch test `k3` does this and kills M3. Adding a `reset` twin of `arq_124_the_last_valid_value_survives_prepare` over `makers()` covers every processor at once.

## WARNING

### W14. `Eq3Band::tail()` is 0, but the trait says it is the ring-down

- class: ambiguity
- ref: `crates/gafira-audio/src/processor.rs:29`, 02 ARQ-125, ARQ-129
- The trait comment says `tail` is "Samples the output keeps ringing after the input ends". An IIR filter rings, and 02 never says what `tail` measures. ARQ-125 declares none.
- Measured after a unit impulse, worst of 11 settings at each rate:

| fs | Falls below −60 dB after | Falls below −120 dB after | Last non-zero sample |
|---|---|---|---|
| 44.1 kHz | 393 samples | 1 653 samples (37 ms) | 14 423 |
| 48 kHz | 424 samples | 1 786 samples (37 ms) | 16 045 |
| 96 kHz | 785 samples | 2 935 samples (31 ms) | 28 084 |

- Effect: no consumer exists yet. When `gafira-media` cuts at the clip end by `tail()`, it will drop up to about 40 ms of ring-down.
- Fix, for the specifier to choose: either state in ARQ-125 that `tail()` is `ceil(0.04 × fs)` with a band active and 0 otherwise (−120 dB, the default I recommend, and it is a few lines in `eq.rs`), or define `tail` as declared only for processors whose ring-down must be kept, and reword the trait comment.

### W15. RED is recorded, not provable from git, and ARQ-23 came after the code

- class: implementation
- ref: `0f7d798`, `c4946a2`, `apply-progress.md` AU2 RED table
- No stub commit exists: `0f7d798` holds `eq.rs` and its tests together, so git cannot show that the tests came first.
- I rebuilt the described stub (a `process` that only cleans the block). Five tests fail, and the three messages I could compare match the record exactly: `fs 44100 gains [15.0, 0.0, 0.0] f 20.0: got 0.000 dB, want 14.962 dB`, `f 20: got -0.000 dB, want 8.976 dB`, and the `!run_pair(16_001, ...)` assertion. The record is credible.
- ARQ-23 did not go through RED: the guard test landed one commit after the code it checks, and `apply-progress.md` says so. Its bit-identical, pole, `reset` and `prepare` tests also passed on the stub by construction, again as recorded. Those are proved by mutants, and C6 to C8 show that this proof had gaps.
- W10 to W13 are test-only, pinned existing behaviour, and are recorded as such.
- Fix: for later deliveries, commit the stub and tests first, or keep the RED log in the scratchpad and cite it. Nothing to change in this code.

## SUGGESTION

- S5. Say in ARQ-125 that a band gain change has no ramp and applies from the next block, so ARQ-124's 10 ms ramp clearly stays with `Gain`. See the ruling below.
- S6. `arq_125_every_pole_is_inside_the_unit_circle` proves stability from outside, by ring-down, and asks only for half the amplitude per 16 384 samples. A `#[cfg(test)]` unit test on `Coefficients` could check `|p| < 1` directly.
- S7. CAL-15 was read on the developer machine, not a mid-range reference one, and `cargo test` checks no threshold. The margin is about 12 times, so the risk is low. Read it once on the reference machine at change close.
- S8. The `AudioProcessor` doc comment (`processor.rs`, lines 11 to 16) lists the blocks that are not processed but omits the empty block, which ARQ-123 now includes.

## Rulings on the recorded deviations

| Deviation | Ruling | Evidence |
|---|---|---|
| Gain change with no ramp, from the next block | Accepted | ARQ-124's ramp belongs to `Gain` (`volume`); ARQ-125 is silent and the bands are not animatable. Direct form I keeps the history through the change. Measured on a tone of amplitude 0.5: a step of at most 0.49, the same order as the tone's own step at 8 kHz, and at most 0.08 at 1 kHz for 0 to +15 dB. Switching every band between +15 and −15 dB every 64 samples on full-scale noise stays bounded (peak 10.4 at 48 kHz, 17.6 at 96 kHz). Worth reviewing when the Mixer moves the EQ live (F2). See S5 |
| History flushed below `f32::MIN_POSITIVE` | Accepted | It serves ARQ-123 and acts about 750 dB below full scale. On noise at seven rates and split into blocks of 1, 7, 500 and 20 000, the output is bit-identical to an unflushed reference. No subnormal after 1 s of signal, then zeros, then a 1e-40 input |
| `tail() == 0` | Accepted against the text of 02, but it contradicts the trait comment | See W14 |

## Closure of the AU1 follow-ups

| Finding | Status | Evidence |
|---|---|---|
| W10 | Closed | `arq_124_a_refused_block_does_not_consume_the_first_block_of_a_gain` kills F2 (spot-checked) |
| W11 | Closed | `arq_123_a_refused_block_after_a_prepare_with_a_zero_reads_no_parameter`, read and run |
| W12 | Closed | `arq_123_a_refused_block_during_a_running_state_leaves_the_next_samples_as_they_were`, read and run, with `eq3band` in its makers |
| W13 | Closed | `arq_124_the_last_valid_value_survives_prepare`, read and run. The `reset` side for `Eq3Band` is C8 |

## Acceptance map

| Criterion | Status | Proof |
|---|---|---|
| ARQ-125 response ±0.5 dB, three rates | Covered | `arq_125_the_impulse_response_matches_h_*`, with a bilinear analogue oracle independent of `eq.rs`. Probe: worst error 5e-6 dB |
| FV-20 (c) | Covered | `fv_20_c_a_sine_sweep_follows_the_theoretical_curve`. Probe with 13 tones, 11 settings, 3 rates: worst error 5e-5 dB |
| Bit-identical at 0 dB | Covered | `arq_125_with_every_band_at_0_db_*`. Probe: random finite bit patterns, 0, −0, no value, and back to 0 after a non-zero run |
| Poles < 1 | Covered | `arq_125_every_pole_*` (indirect, S6). Probe: output sample-exact to an independent RBJ cascade, whose poles were computed directly on a ±15 dB grid and 14 rates, largest 0.9999996 |
| Omission at `fs / 2` | Covered | `arq_125_a_band_at_or_above_*`. Probe: each band at `fs − 1`, `fs`, `fs + 1` |
| Row of 4.17.5 | Covered | `p13_the_schema_has_the_three_bands_of_the_table`, `tests/eq_contract.rs` |
| ARQ-23 | Covered | `arq_23_sixteen_tracks_*`, `arq_23_an_allocation_inside_the_guard_*`. Own probe with 64 rounds of changing gains. No lock in `src/`. Spot-check: `vec!` in `process` gives SIGABRT |
| CAL-15 | Covered | `cargo bench`: 209 µs per block (208 to 210) against 2 667 µs, 2.0 % of a core, recorded 255 µs. All bands were active and stereo, and my own timing gave 243 µs. See S7 |
| W10 to W13 (4 rows) | Covered | Above |
| ARQ-123 on `Eq3Band` | Weak | C6, C8 |
| ARQ-125 after `prepare` at another rate | Weak | C7 |

12 of 14 covered. Product IDs: ARQ-125, ARQ-23 and FV-20 (c) are named in tests, and CAL-15 in `benches/mix.rs`. None was left out.

## Hand mutants

| Mutant | Source | Result |
|---|---|---|
| History flushed at the `f64` threshold | Spot-check of the recorded list | Killed by `arq_123_the_history_is_flushed_below_the_smallest_normal_sample` |
| F2, a refused block consumes the Gain first block | Spot-check of W10 | Killed by `arq_124_a_refused_block_does_not_consume_the_first_block_of_a_gain` |
| `vec!` in `Eq3Band::process` | Spot-check of ARQ-23 | Killed, process aborts with SIGABRT |
| M1, no `clean_input` | Independent, ARQ-123 | **Survives** (C6) |
| M2, no redesign in `prepare` | Independent, ARQ-125 | **Survives** (C7) |
| M3, `reset` forgets the gains | Independent, ARQ-123 | **Survives** (C8) |

Each survivor is not equivalent: the scratch tests `k1`, `k2` and `k3` pass at HEAD and each fails on its mutant.

## Probes

Scratch export of `c4946a2`, with a 15-test probe file written before the implementer's tests were read. All pass. It also covers refused and empty blocks with hostile values, parameter clipping, unknown ids, stereo histories, and block-split invariance.

Extreme rates (201 Hz to `u32::MAX`) were stable at +15 dB on every band, so there is no finding.

## Size

`git diff --shortstat d437b8a c4946a2`: 15 files, 1 446 insertions, 10 deletions, against a 400 forecast (560 at x1.4). The owner judges serial deliveries by completion, so this is not a finding.

## Result

Verdict **FAIL**: 3 CRITICAL, 2 WARNING, 4 SUGGESTION. Per class: implementation 4 (C6, C7, C8, W15), ambiguity 1 (W14).

next: specifier (ambiguity: W14), then tdd-apply (implementation: C6, C7, C8, W15)

# Verify report: gafira-audio, AU2, round 2 (closure)

| Item | Value |
|---|---|
| Commit | `050cd25`, branch `feat/gafira-audio`, base `develop` |
| Depth, mode, tier | V2, full, T4 |
| Scope | C6, C7, C8, W14, W15 of AU2 round 1, the ARQ-125 and ARQ-129 text of `b9442ae`, commits `9ecc6c3`, `595487b`, `050cd25` |
| Mutation | `mutation: archive`, no run yet, not a finding |
| Verdict | **PASS** for delivery AU2; AU3 and AU4 remain, so `next` is `tdd-apply` |

## Gates

Each command ran alone in the worktree, with no `CARGO_TARGET_DIR`. `df -h /` showed 42 GB free at the start and 41 GB at the end.

| Gate | Exit code |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1 587 passed, 0 failed, 0 ignored; 10 more than round 1, the 10 tests of this fix round) |
| `cargo deny --all-features --locked check` | 0 |
| CI policy (ARQ-07, unsafe, V5, V12, ARQ-107, V13, V14) | 0 on each |
| Host matches matrix target | 0, with `x86_64-unknown-linux-gnu` put in place of `${{ matrix.target }}`, which does not resolve outside CI |

## Artifact integrity (step 3c)

- The slice card commit `9bfba1c` touches only `apply-progress.md`. From `d437b8a` to HEAD that file has additions only, at the end, so the card is unchanged and no amend exists.
- `9ecc6c3` and `595487b` touch only `crates/gafira-audio/`. Only `b9442ae` and `050cd25` touch `specs/`, and they are records, not implementation.
- `b9442ae` changes three rows of 02: ARQ-125, ARQ-129, and the "Tamaño 0" row, which now says an empty block is cleaned and not processed (ARQ-123, already so in the code).

## Closure of round 1

Mutants ran on a clean export of HEAD, one at a time, with `cargo test -p gafira-audio --locked --no-fail-fast`. Baseline: 130 passed. The source was restored and compared afterwards. These are the round 1 mutants, so no independent budget was spent.

| Finding | Check | Result |
|---|---|---|
| C6 | M1, no `clean_input` in `Eq3Band` | **Closed.** Killed by `arq_123_a_non_finite_sample_counts_as_zero_now_and_in_the_history` |
| C7 | M2, no redesign in `prepare` | **Closed.** Killed by `arq_125_prepare_at_another_rate_designs_for_the_new_rate_with_the_last_valid_value` and `arq_125_a_band_omitted_at_the_new_rate_does_nothing_after_prepare` |
| C8 | M3, `reset` forgets the gains | **Closed.** Killed by `arq_124_the_last_valid_value_survives_reset` and `arq_125_tail_is_kept_by_reset_prepare_at_the_same_rate_and_a_refused_block` |
| W14 | ARQ-125 and ARQ-129 of `b9442ae`, tests of `595487b`, probes below | **Closed** |
| W15 | RED replay of the W14 tests | **Open, follow-up.** Process only, see below |
| S5 | ARQ-125 of `b9442ae` | **Closed.** The row now says the change has no ramp and the ramp of ARQ-124 is only for `Gain` |

## Ruling: `tail()` is 0 before the first `process`

**It conforms to the spec. Keep it. It is not a deviation, so the "Deviation" line in `apply-progress.md` needs no action.**

- ARQ-125: a band is active if it is not omitted and "su ganancia vigente, el último valor válido o, sin él, el valor por defecto" is not 0. The gain default is 0 (`Band::new`), so with no valid value yet no band is active, and "0 si no" applies.
- ARQ-129: `tail()` is the frames the output keeps ringing after the input ends. Probe `p2`: a prepared `Eq3Band` with no value passes an impulse bit for bit, so 0 is also the true ring-down.
- `tail()` takes no snapshot, so it cannot see a value that has not arrived. The first processed block that carries a gain turns the tail on in that same block: probe `p2` gives 0, then 1 920 at 48 kHz.
- An empty or refused block carrying a gain leaves it at 0, as ARQ-123 and the criterion "un bloque rechazado no lo cambia" require (`arq_125_tail_is_kept_by_reset_*`, probe `p2`).
- Consequence for later work: `tail()` is a value of the current state. A consumer that needs it reads it after the last block, not ahead of the first one. This is what ARQ-129 defines, so nothing needs to change.

## W14: criteria of ARQ-125 and ARQ-129

| Criterion | Proof | Status |
|---|---|---|
| 1 764, 1 920, 3 840, 1 765, 171 798 692 at `u32::MAX`, no `panic` | `arq_125_tail_is_forty_milliseconds_in_frames_with_a_band_active` | Covered |
| 0 at 0 dB, −0 dB, `sample_rate` 0, only `high` at 16 000 Hz; 641 at 16 001 Hz | `arq_125_tail_is_zero_without_an_active_band` | Covered |
| Kept after `reset`, `prepare` at the same `fs` and a refused block | `arq_125_tail_is_kept_by_reset_prepare_at_the_same_rate_and_a_refused_block` | Covered |
| Below 1e-6 from frame `tail()`, every combination at three rates | `arq_125_after_an_impulse_every_sample_from_tail_is_below_1e_6`; probe `p1` | Covered |
| 0 to +15 dB in `mid`: ±1e-6 against the new biquads from the prior history | `arq_125_a_gain_change_applies_from_sample_0_of_the_block_that_carries_it`, against the `Reference` in the test file | Covered |

Probes, in the export and not committed (`zz_r2.rs`), all passed:

| Probe | Result |
|---|---|
| `p1`, the test's impulse carried on for 12 more blocks of 8 192 frames, 78 cases | Worst sample from `tail()` is 3.3e-7, below 1e-6 |
| `p2`, before the first `process` | See the ruling |
| `p3`, `prepare` at another rate with a gain kept | 3 840 at 96 kHz, 0 at `fs` 0, 0 with only `high` at 16 kHz |
| `p4`, `ProcessorChain` of two `Eq3Band` | 3 840 at 48 kHz, the ARQ-129 sum on real tails |
| `p5`, active band set to 0 dB after an impulse | `tail()` 0 and the next block is exact zeros, so nothing is cut |

## WARNING

### W15. RED is recorded, not provable from git (carried, follow-up)

- class: implementation
- follow-up: yes
- ref: `0f7d798`, `c4946a2`, `595487b`, `apply-progress.md` AU2 fix round 1
- Round 1 asked later work to commit the stub and tests first, or to cite a RED log. `595487b` again holds the tail code and its tests in one commit, and no log is cited.
- The record is credible. With `Eq3Band::tail` removed on the export, the same five tests fail, and the messages match the record: `left: 0, right: 1764` at fs 44100, `left: 0, right: 641`, `left: 0, right: 1920`, and the `tail > 0` panic in the impulse test.
- `9ecc6c3` pins code that was already right, so RED could not apply. Its proof is the mutants above.
- The commits cannot change and no code needs to. Keep it for AU3 and AU4.

## SUGGESTION

- S9. The `tail` comment in `processor.rs` (line 29) says "Samples"; ARQ-129 and `Eq3Band::tail` say frames. Change the word when the file is next touched, with S8.

S6, S7 and S8 of round 1 are out of this round's scope and were not re-examined.

## Result

Verdict **PASS** for AU2: 0 CRITICAL, 1 WARNING (follow-up), 1 SUGGESTION. Per class: implementation 1 (W15, follow-up); ambiguity 0, architecture 0, claim 0. Criteria 5 of 5 covered. Mutants M1 to M3: 3 run, 3 killed. Size of the fix round: 5 files, 261 insertions.

next: tdd-apply (next delivery, AU3)

# Verify report: gafira-audio, AU3 and AU4, round 1

| Field | Value |
|---|---|
| Commit | `0102cd6`, branch `feat/gafira-audio`, base `develop` (merge base `040ba8e`) |
| Depth, mode, tier | V2, full, T4 at close; `mutation: archive`, no `mutants` report (not a finding) |
| Verdict | **FAIL** |
| Counts | 6 CRITICAL, 3 WARNING, 5 SUGGESTION |
| Per class (CRITICAL + WARNING) | implementation 8, ambiguity 1, architecture 0, claim 0 |
| Criteria | 4 of 9 covered: LoudnessNormalizer row, ARQ-111, ARQ-128, FV-04 (b). Partial: ARQ-126, ARQ-127, ARQ-110, FV-10 (b), FV-20 (d) |

## Gates

Worktree at `0102cd6`, no `CARGO_TARGET_DIR`, each exit code read alone. The worktree stayed clean.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 |
| `cargo deny check` | 0 |
| CI policy (ARQ-07, unsafe, V5, V12, ARQ-107, V13, V14) | 0, each step |

## Artifact integrity (step 3c)

Commit `9bfba1c` (`docs(gafira-audio): slice card`) touches only `apply-progress.md`. `git diff 9bfba1c HEAD` on that file only adds text after line 9, so no later commit changed the card. No `slice card amend` was needed. Every product ID of AU3 and AU4 is in the tests (`git grep`): ARQ-126, ARQ-127, ARQ-128 in test names; FV-10 (b) in test names; FV-20 (d) and FV-04 (b) in header or inline comments only, with no test named after them.

## EBU Tech 3341: three sources compared

Source: the published document, fetched from `tech.ebu.ch` (Tech 3341, 2023 edition, Table 1), so neither reading from memory had to be trusted. The independent reference is a NumPy BS.1770-4 meter using the published 48 kHz coefficients, cross-checked with `ffmpeg ebur128` (one decimal).

| Case | Published | Coordinator's reading | Implementer's test |
|---|---|---|---|
| 1 | -23 dBFS, 20 s; -23.0 | same | same |
| 2 | -33 dBFS, 20 s; -33.0 | same | same |
| 3 | -36, -23, -36 for 10, 60, 10 s; -23.0 | same | same |
| 4 | -72, -36, -23, -36, -72 for 10, 10, 60, 10, 10 s; -23.0 | same | **differs**: -72, **-40**, -23, **-40**, -72 for 10, 10, **40**, 10, 10 s |
| 5 | -26, -20, -26 for 20, 20.1, 20 s; -23.0 | same | **differs**: -26, -20 for 20, **10** s, third piece missing, expected -22.99 |

The coordinator's reading matches the published table on all five cases, with no differences. The dBFS figures are per-channel peak levels, applied in phase.

Published signals through the real code (`integrated_loudness`), tolerance ±0.1 LU:

| Case | Rust 48 kHz | NumPy | ffmpeg | Rust 44.1 kHz | Rust 96 kHz |
|---|---|---|---|---|---|
| 1 | -22.9933 | -22.9933 | -23.0 | -22.9905 | -23.0106 |
| 2 | -32.9933 | -32.9933 | -33.0 | -32.9905 | -33.0106 |
| 3 | -23.0139 | -23.0139 | -23.0 | -23.0111 | -23.0312 |
| 4 | -23.0139 | -23.0139 | -23.0 | -23.0111 | -23.0312 |
| 5 | -22.9787 | -22.9787 | -23.0 | -22.9759 | -22.9960 |

The code passes the real cases, and Rust agrees with NumPy to four decimals. The defect is in the proof (C10), not in the meter at these rates. Case 6 (5.0 in L, R, C, LFE silent, Ls, Rs) gives -23.016.

## Rulings on the recorded deviations

| Deviation | Ruling | Why |
|---|---|---|
| Strict gates (`>`) | Compatible | BS.1770-4 gates with `>`; ARQ-126 does not say otherwise |
| New errors `OutOfRange`, `ChannelCount` | Compatible | ARQ-126 and ARQ-128 require an `Err`; both have keys (ARQ-102) and tests |
| Unequal channels give `Err` | Compatible | Measuring the shortest would hide data. A shape, not a numeric range. Worth one line in 02 (W18) |
| Rates ≤ 3 364 Hz give `Err` | **Not compatible as written** | ARQ-126 lists no rate floor and says "En otras frecuencias" with no limit. `SampleRate` admits every divisor of 705 600 000, from 1 Hz, so 3 360 Hz and below are valid. §4.17 "Rangos" says an engine checks only the ranges its rows indicate. Better than the garbage it replaced, but it is a workaround for the design in C9, and the fix there removes it. Keeping it needs a spec row (W18) |
| True peak with a 24-tap Kaiser sinc | Compatible | Annex 2 fixes the oversampling, and its table is an example. Cases 15 to 19 pass (-6.019 to -6.021 and +2.986 at 48 kHz, limits +0.2/-0.4). Near Nyquist it beats the table: -0.018 dB against -0.723 dB at 23 kHz. The table used for that comparison was quoted from memory, so it is a comparator, not an oracle |
| x2 up to 96 kHz, none above | Compatible, not pinned | ARQ-127 pins only x4 at 48 kHz or less, and x2 at 96 kHz and x1 at 192 kHz keep the same 192 kHz grid. Under-read measured at 96 kHz: -0.47 dB at 24 kHz. No test pins it (W16) |
| Resampler latency 0 | Compatible | Output frame `j` is the input at `j × from × speed / to`. The impulse lands within 1 frame on 7 rate and speed rows. `process` still holds back about 60 lobes of look-ahead, which `latency()` does not report (S12) |
| Speed range inclusive | Compatible | "de 0,1 a 10" with `Err` at 0.09 and 10.1 |

## CRITICAL

### C9. K weighting misses ±0.1 dB at rates of 16 kHz and below
- class: `implementation`
- ref: `crates/gafira-audio/src/loudness.rs:43`, ARQ-126
- Evidence, real meter against its own 48 kHz reading, worst tone (2.5 kHz), confirmed by the coefficient response:

| Rate | Deviation | Rate | Deviation |
|---|---|---|---|
| 8 kHz | **+0.496 dB** | 22.05 kHz | +0.081 dB |
| 11.025 kHz | **+0.292 dB** | 24 kHz | +0.068 dB |
| 12 kHz | **+0.252 dB** | 32 kHz | +0.032 dB |
| 16 kHz | **+0.152 dB** | 44.1 to 192 kHz | ≤ 0.040 dB |

- Root cause: the bilinear transform compresses the shelf towards Nyquist. The pre-warp is exact only at the shelf centre.
- Test gap: `arq_126_the_k_weighting_at_other_rates_*` covers only 32, 44.1, 88.2, 96 and 192 kHz, and stops at 15 kHz, not 16 kHz.
- Recorded claim is wrong: `apply-progress.md` says the 3.6 to 400 kHz rates stay "within the spec's 0.1 dB". That sweep used only a 1 kHz tone, where the deviation is small. Its note that a low rate is "not a fault" reads a limit into ARQ-126 that the text does not have.
- Fix: run the K filter on the signal oversampled by an integer `L`, with `L = ceil(32 000 / fs)`, and gate on the same 400 ms. Measured here with a long Kaiser filter: worst deviation 0.032 dB at 8 kHz (x4), 0.028 dB at 11.025 kHz (x3), 0.021 dB at 12 kHz (x3), 0.032 dB at 16 kHz (x2). The filter is then always designed well above the shelf, so the `rate` guard goes away too. Extend the test to 8, 11.025, 12, 16, 22.05 and 24 kHz, tones up to `min(16 kHz, 0.98 × Nyquist)`, including 2.5 kHz. If the owner prefers to limit the promise to rates of 32 kHz and up, that is an ARQ-126 amendment (W18), not a code note.

### C10. Tests named EBU cases 4 and 5 do not use the EBU signals
- class: `implementation`
- ref: `crates/gafira-audio/tests/loudness.rs:77`, `:94`, ARQ-126
- ARQ-126 asks for cases 1 to 5. Two of the five are the implementer's own sequences, as `apply-progress.md` records, yet their names claim the EBU numbers. See the table above.
- Fix: replace the pieces with the published ones. Case 4 is `(10, -72), (10, -36), (60, -23), (10, -36), (10, -72)`, want -23.0. Case 5 is `(20, -26), (20.1, -20), (20, -26)`, want -23.0. Both pass today (-23.0139 and -22.9787), and the published case 4 puts the -36 pieces about 3 LU under the relative gate, closer than the invented -40. Update the note that says these may differ.

### C11. The Nivelar ceiling is exceeded on ordinary signals, and a test tolerance hides it
- class: `implementation`
- ref: `crates/gafira-audio/src/loudness.rs:336`, `crates/gafira-audio/tests/leveling.rs:252`, ARQ-127, FV-10 (b), 02 §4.17.5 "Tope de pico"
- Evidence: ordinary filtered-noise stereo, gain from `level`, applied through `LoudnessNormalizer`, peak re-measured with `true_peak`: **49 of 120** cases finish over -1 dBTP, worst +3.41e-7 dB (first probe: 13 of 40, +3.34e-7 dB).
- This is inaudible. It still breaks a stated bound: the row says "≤ −1 dBTP; igualdad admitida", the "Tolerancias" row gives the ceiling no tolerance, and `design.md` lists the peak ceiling as an invariant. The test's `+ 1e-3` is recorded under "decisions the specs leave open", but the spec does not leave it open.
- Root cause: the gain is exact in `f64`, then each output rounds to `f32`.
- Fix: in `level`, lower a binding ceiling by 1e-5 dB. Measured: 0 of 120 over the ceiling at 1e-6 dB and at 1e-5 dB, and "+2 dB ±0.01" still holds. Keep `plan_leveling` exact so its equality rule stays testable. Change the test bound to `<= -1.0` with no tolerance, through `LoudnessNormalizer` (W17). The alternative is a stated tolerance in the "Tolerancias" row, which is an owner decision.

### C12. A non-finite intermediate gives "no measure" instead of `Err`
- class: `implementation`
- ref: `crates/gafira-audio/src/loudness.rs:184`, ARQ-110
- Evidence, public `integrated_from_blocks`: `[1.7e308, 1.7e308]` and `[1e308, 1e308, 1e308]` give `Ok(None)`, while `[1e308]` gives `Ok(Some(3079.3))`.
- Root cause: the sum in `mean` overflows to `+∞`, the relative gate becomes `+∞`, no block passes, and the loudest input reports the same result as silence.
- These inputs are extreme, but ARQ-110 is explicit: a non-finite intermediate result gives a typed `Err`, so it stays CRITICAL. `integrated_loudness` itself cannot reach it, since `f32` samples cannot overflow the energy sum.
- Fix: make `mean` return `Result` and pass the sum through `finite("energies", ...)`, or sum `e / n`. Add a test with `[1.7e308, 1.7e308]`.

### C13. Hand mutant survives: the relative gate can keep blocks the absolute gate dropped
- class: `implementation`
- ref: `crates/gafira-audio/src/loudness.rs:169`, ARQ-126
- Mutant `am1`: `louder_than(energies, ...)` for `louder_than(&passed, ...)`. All tests pass.
- Why it matters: when the relative gate falls under -70 LUFS, the mutant lets absolute-gated blocks back in. Ten blocks at -68 and ten at -75 give -68.0 correctly and -70.22 with the mutant, 2.2 LU off. It is a rule-order mistake that `cargo-mutants` cannot express.
- Fix: add that signal to `integrated_from_blocks` tests, want -68.0 within 1e-9.

### C14. Hand mutant survives: the stopband test cannot see 1.0 of the lower Nyquist
- class: `implementation`
- ref: `crates/gafira-audio/tests/resample.rs:250`, ARQ-128
- Mutant `rm5`: `CUTOFF` 0.97. All tests pass, but the mutant gives only **-28.98 dB** at 1.0 of the lower Nyquist, against the ≥ 80 dB rule. The real filter is safe: -94.70 dB at worst.
- Root cause: `sine()` has phase 0, so a tone at exactly the output Nyquist samples as `sin(πj) = 0` and the output RMS is near zero whatever the filter does. Rows at 1.0 can pass blind on downsampling, and the next row is 1.02, past the mutant's transition (-106.76 dB there).
- Fix: measure each fraction at several phases (for example 0, 0.5, 1.1 and π/2 rad) and take the largest, and add rows 1.005 and 1.01. Keep at least one downsampling row with a large input-side tone, such as 48 → 8 kHz.

## WARNING

### W16. Hand mutant survives: x2 between 48 and 96 kHz is not pinned
- class: `implementation`
- ref: `crates/gafira-audio/src/loudness.rs:235`
- Mutant `am2`: x1 for 48 001 to 96 000 Hz. All tests pass. WARNING because ARQ-127 pins only 48 kHz or less, so this is a recorded decision, not a spec rule.
- Fix: a 96 kHz fs/4 sine at phase 45 degrees, where x1 reads 0.707 and x2 reads the crest.

### W17. Nivelar is never proved end to end through `LoudnessNormalizer`
- class: `implementation`
- ref: `crates/gafira-audio/tests/leveling.rs:41`, ARQ-127, FV-10 (b), FV-20 (d)
- The FV-10 (b) tests apply the gain with a local `scaled` helper, so the path `level` to `LoudnessNormalizer` to `true_peak` has no test. My probe exercised that path. It behaves, apart from C11.
- Fix: run both FV-10 (b) examples through the processor, in blocks.

### W18. ARQ-126 does not settle low rates or the rate floor
- class: `ambiguity`
- ref: specs/02 ARQ-126, §4.17 "Rangos"
- "hasta 16 kHz" does not say what applies when Nyquist is under 16 kHz, and no row gives a floor. The plain reading, used for C9, is every frequency up to `min(16 kHz, Nyquist)` at every rate. The implementer read it as applying only where 16 kHz fits under Nyquist.
- Fix: one line in ARQ-126 stating the band at low rates. If the C9 fix is taken, also state that the meter accepts every `SampleRate`. Otherwise add a floor row with its `Err`. Unequal channel lengths can go in the same edit.

## SUGGESTION

- S10. `crates/gafira-audio/src/loudness.rs:232`: use x4 up to 192 kHz in place of x2 and x1. The current choice is compatible, but under-reads 0.47 dB at 24 kHz on 96 kHz, and the recorded sweep saw -1.5 dB at 192 kHz. It would cost one line.
- S11. Doc comments: `loudness.rs:110` says "3 364 Hz or less", but the guard is `≤ 3 363.95`, so 3 364 passes it (it is not a valid `SampleRate`; valid rates at the edge are 3 360 refused, 3 528 accepted). `true_peak` says "As `integrated_loudness`" for its errors, but it has no `rate` check.
- S12. `crates/gafira-audio/src/resample.rs:158`: expose the look-ahead of `process` (for example `lookahead_frames()`). `latency()` 0 is right for alignment, but a streaming caller has no way to learn how many frames are held back.
- S13. `crates/gafira-audio/src/resample.rs:178`: near `2^42` input frames, `f64` rounding of `pos ± half` could step past the 2-entry guard of the kernel table and panic. This comes from reading the code and was not run: it needs about 3 years of 48 kHz audio. `table.get(k)` with a 0 default closes it.
- S14. `bessel_i0` exists twice (`loudness.rs:195`, `resample.rs:70`) with different iteration limits. One copy in `dsp.rs` is enough.

## Requirement matrix

| Criterion | State | Proof |
|---|---|---|
| ARQ-126 cases 1 to 5 | Partial | Code passes the published cases (probe). Tests prove 1 to 3 only (C10) |
| ARQ-126 997 Hz at 44.1, 48, 96 kHz | Covered | -22.997, -23.000, -23.017; `arq_126_a_997_hz_stereo_sine_*` |
| ARQ-126 six channels, silence, 399 ms, 3 channels | Covered | Probe: -23.01, -21.52, `None`; `arq_126_*` |
| ARQ-126 K weighting at other rates | **Failing** | C9 |
| ARQ-126 gates | Partial | Exact boundaries tested; rule order not (C13) |
| ARQ-127 gain rule, `reached`, no measure | Covered | `arq_127_*` |
| ARQ-127 ceiling | **Failing** | C11 |
| LoudnessNormalizer row (SP19) | Covered | `p13_*`, `arq_127_a_gain_of_16_db_*`, `alloc.rs` |
| FV-10 (b), FV-20 (d) | Partial | C11, W17 |
| ARQ-128 lengths, empty, speed `Err` | Covered | 48 000, 48 000, 0 (probe); `arq_128_*` |
| ARQ-128 SNR | Covered | 125.3 dB probe, against ≥ 80 |
| ARQ-128 passband | Covered | 540 checks over 6 × 6 rates, 5 speeds: worst 0.00012 dB |
| ARQ-128 stopband | Covered, weak test | Real filter -94.70 dB at 1.0 on 12 rows; the test is blind there (C14) |
| ARQ-128 impulse, declared latency | Covered | 7 rows within 1 frame |
| FV-04 (b) | Covered | 96 000 samples at 2.0× give 48 000 |
| ARQ-110 | Partial | C12; the other non-finite paths are tested |
| ARQ-111 | Covered | `arq_111_*`; chunk sizes 1, 7, 333 and 5 000 are byte-identical in the probe |

## Hand mutants

Applied on the scratch export, all reverted. The repo was not touched.

| # | Rule and mutation | Result |
|---|---|---|
| spot | Recorded: length order `n × (to / from) / speed` | Killed by `arq_128_the_length_is_the_f64_expression_*` |
| am1 | ARQ-126: relative gate over all blocks | **Survived** (C13) |
| am2 | Recorded x2: x1 for 48 001 to 96 000 Hz | **Survived** (W16) |
| am3 | ARQ-127: last x4 phase dropped | Killed by `arq_127_a_sine_at_a_quarter_*` |
| rm4 | ARQ-128: cutoff ignores the speed | Killed by the stopband and passband tests |
| rm5 | ARQ-128: cutoff 0.97 | **Survived** (C14) |
| rm5b | ARQ-128: cutoff 0.93 | Killed by the passband test |
| rm6 | ARQ-128: emit one input early (`<=`) | Killed by `arq_128_chunks_of_any_size_*` |

Run 8, killed 5, survived 3. The unmutated baseline passed.

## Probes that found nothing

- `LoudnessNormalizer` applied the planned gain over 512-frame blocks, and the peak scaled as planned apart from C11.
- Tone at 0.9 of the lower Nyquist through 4 upsampling rows: everything else at -105.3 dB.
- 48 to 48 kHz at 1.0×: within 3e-7 of the input on a 1 kHz tone.
- Time reversal of a signal does not change its true peak beyond rounding.

Mutation: `mutation: archive`, so no report at this round and no finding. T4 stays owed at close.

Size: 2 232 insertions against a forecast of 830, not a finding, as instructed.

Disk: 30 GB free at the start, 28 GB at the end. Scratch build folders were freed with `cargo clean`.

next: specifier (W18, ambiguity), then tdd-apply (C9 to C14, W16, W17, implementation)
