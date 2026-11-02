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
