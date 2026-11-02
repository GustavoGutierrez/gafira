## Slice card
Flow: full | level: L3 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: layer-2 crate in the owner's parallel full route; one writer per worktree
Product IDs: AU1 ARQ-123, ARQ-124, ARQ-129, FV-10 (a, d), ARQ-110, ARQ-111; AU2 ARQ-125, ARQ-23, CAL-15, FV-20 (c); AU3 ARQ-126, ARQ-127, FV-10 (b), FV-20 (d); AU4 ARQ-128, FV-04 (b) (source: specs/02-arquitectura-software.md, specs/01-producto-y-calidad.md, specs/04-funcionalidad-diseno-imagen-exportacion.md, specs/05-funcionalidad-video-y-distribucion.md)
Scope: AU1 to AU4 of specs/changes/layer-2/design.md.      Non-goals: other streams; cancellation of loudness, Nivelar and resampling (D-43).
Deliveries: AU1 to AU4, serial; each verified and merged before the next.
Plan: specs/changes/layer-2/design.md, table and decisions; `crates/gafira-audio/` plus the files P2 assigns.
Owner decisions touched: D-40 to D-44 as recorded; none reopened.

# Apply progress: gafira-audio

## AU1

Branch `feat/gafira-audio`, from `develop` 9124700. Commits: `17cdde0` (contract, Gain), `5a540bd` (Fade, Pan, EqualPower), `894850d` (chain, mix, offsets), `173fdc9` (property tests, integer ramp). Every commit passed `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace --locked`, each with exit code 0.

### Done

- `AudioBlock`, `ParamSnapshot` (16 `Copy` pairs, no allocation), `AudioError::NonFinite` with `ToMessage` (`error.audio.non_finite`), `AudioProcessor` (02 §4.9) and `Describe` for every processor.
- `db_to_gain`, `percent_to_gain`, `Gain` with a 10 ms ramp (`sample_rate / 100` samples), `Fade` (In, Out), `Pan`, `crossfade_gains(EqualPower)`, `ProcessorChain`, `mix`, `flicks_to_samples`.
- Not in AU1: `Eq3Band` (ARQ-125), loudness (ARQ-126, ARQ-127), resampling (ARQ-128), `bench_mix_16` and the allocation guard (ARQ-23, CAL-15; AU2).

### RED evidence (failing assertion, stub first)

| Criterion | Test and assertion |
|---|---|
| ARQ-123 totality | `arq_123_a_non_finite_sample_counts_as_zero`: `[NaN, inf, -inf] != [0, 0, 0]`; `arq_123_no_output_is_subnormal_after_signal_then_zeros_or_from_tiny_input`; `arq_123_exactly_max_block_is_processed` |
| ARQ-123 parameters | `arq_123_parameter_out_of_range_is_clipped`: 500 gave 1.0, want 2.0; `arq_123_non_finite_parameter_keeps_the_last_valid_one`; `arq_123_a_non_finite_duration_keeps_the_last_valid_one` |
| ARQ-123 zero sizes | `arq_123_prepare_with_a_zero_or_an_oversized_block_leaves_the_block_intact` passed against the no-op stub by construction; killed by mutants 4a, 4c, 4d, 4e |
| ARQ-124 gain | `arq_124_db_to_gain_follows_ten_to_the_db_over_twenty`: `6 dB gave 1`; `arq_124_minus_infinity_db_is_exactly_zero`: `Ok(1.0) != Ok(0.0)`; `arq_124_two_hundred_percent_is_exactly_two`: `Ok(1.0) != Ok(2.0)`; `arq_124_ramp_lasts_ten_milliseconds_at_every_rate`: `44100: still ramping`; `arq_124_a_jump_from_zero_to_one_changes_at_most_one_480th_per_sample` |
| ARQ-124 fade, FV-10 (a) | `fv_10_a_two_second_fade_in_rises_from_zero_to_one_continuously`; `arq_124_a_fade_out_falls_from_one_to_zero_and_holds`; `arq_124_duration_zero_changes_nothing` (passed against the stub, killed by mutant 7a) |
| ARQ-124 EqualPower | `arq_124_equal_power_is_cos_for_the_outgoing_side_and_sin_for_the_incoming`; `arq_124_equal_power_clips_p_to_zero_and_one` |
| ARQ-124 pan, D-22 | `arq_124_pan_hard_left_silences_the_right_channel`; `arq_124_pan_half_right_gives_0_5412_on_the_left_and_the_right_equal`; `arq_124_pan_follows_the_minus_three_db_law_over_the_whole_range` |
| ARQ-129 chain, mix | `arq_129_the_chain_applies_its_processors_in_order`: `[1,1] != [3,3]`; `arq_129_the_latency_of_a_chain_is_the_sum`: `0 != 8`; `arq_129_the_mix_of_16_tracks_matches_the_manual_sum`; `arq_129_a_muted_track_adds_nothing` (strengthened after the stub passed it, killed by mutant 11) |
| ARQ-129, FV-10 (d) offsets | `fv_10_d_one_millisecond_is_44_samples_at_44_1_khz_and_48_at_48_khz`: `0 != 44`; `arq_129_a_half_sample_rounds_away_from_zero`; `arq_129_the_extremes_of_i64_do_not_overflow`; `arq_129_exact_halves_at_huge_offsets_round_away_from_zero` |
| ARQ-110 | `arq_110_db_to_gain_rejects_nan_and_plus_infinity_and_overflow`, `arq_110_percent_to_gain_names_percent_for_non_finite`, `arq_110_equal_power_rejects_a_non_finite_p` (all RED). `arq_110_finite_input_in_the_domain_gives_finite_normal_output` and the mix property are gap-closing, not RED, proved by mutant 2 (3 of 3 runs) |
| ARQ-111 | `arq_111_two_runs_give_the_same_bytes_with_and_without_a_reset`, `arq_111_reset_*`, `arq_111_the_same_tracks_give_the_same_bytes`: gap-closing (determinism holds by construction), no mutant targets them |
| ARQ-102 (new crate) | `arq_102_audio_error_gives_its_key_and_names_the_field`, `arq_23_*` snapshot tests, `adr_0005_block_frames_*`: written beside their types, not RED |

### Hand mutants (killed, then reverted; none committed)

| # | Mutation | Killed by |
|---|---|---|
| 1 | `clean_input` passes all / only NaN / only `±∞` | `arq_123_a_non_finite_sample_counts_as_zero`, `arq_123_non_finite_samples_count_as_zero_and_nothing_is_subnormal`, `arq_123_pan_cleans_non_finite_samples_on_any_block` |
| 1 | `clean_output` leaves `±∞` | `arq_123_the_mix_stays_finite_when_the_sum_overflows_f32`, `arq_123_there_is_no_clipping_at_zero_dbfs` |
| 1 | `clean_output` leaves NaN | Survives: equivalent. Inputs are cleaned and every gain is finite, so no public path yields a NaN output. The branch guards processors with state (AU2) |
| 2 | no subnormal flush | `arq_123_no_output_is_subnormal_after_signal_then_zeros_or_from_tiny_input`, `arq_123_a_chain_is_total_on_hostile_input`, `arq_110_finite_input_in_the_domain_gives_finite_normal_output` |
| 3 | NaN or `±∞` parameter applied (`!is_nan`, `true`) | `arq_123_non_finite_parameter_keeps_the_last_valid_one`, `arq_123_a_non_finite_duration_keeps_the_last_valid_one`, `arq_123_pan_out_of_range_is_clipped_and_non_finite_is_ignored` |
| 3 | out-of-range not clipped | `arq_123_parameter_out_of_range_is_clipped`, `arq_123_pan_out_of_range_is_clipped_and_non_finite_is_ignored` |
| 4 | `sample_rate` 0 / `channels` 0 processed | `arq_123_prepare_with_a_zero_or_an_oversized_block_leaves_the_block_intact` |
| 4 | `max_block` 0 processed (`\|\| max_block == 0`) | `arq_123_prepare_with_a_zero_or_an_oversized_block_leaves_the_block_intact`, `arq_123_a_processor_that_was_never_prepared_leaves_the_block_intact` |
| 4 | `<` instead of `<=` (exactly `max_block` unprocessed); no upper check; unequal channels processed | `arq_123_exactly_max_block_is_processed`; `arq_123_prepare_with_a_zero_or_an_oversized_block_leaves_the_block_intact`; `arq_123_channels_of_different_lengths_are_an_invalid_block_and_stay_intact` |
| 5 | clip to ±1 (in `clean_output`) | `arq_123_there_is_no_clipping_at_zero_dbfs`, `arq_123_the_mix_does_not_clip_at_zero_dbfs`, `arq_123_parameter_out_of_range_is_clipped` |
| 6 | `db / 10`, `db * 20`; `-∞` not 0; 200 % as 4.0 or 1.0 | `arq_124_db_to_gain_follows_ten_to_the_db_over_twenty`; `arq_124_minus_infinity_db_is_exactly_zero`; `arq_124_two_hundred_percent_is_exactly_two` |
| 6 | ramp 11 ms, 9 ms, 20 ms; step uses `ramp_len + 1`; ramp ends off target; no ramp | `arq_124_ramp_lasts_ten_milliseconds_at_every_rate`, `arq_124_a_jump_from_zero_to_one_changes_at_most_one_480th_per_sample`, `arq_124_a_downward_change_ramps_too`, `arq_111_reset_ends_a_running_ramp_at_its_target` |
| 7 | duration 0 silences; direction reversed; no end hold; `reset` no-op | `arq_124_duration_zero_changes_nothing`; `arq_124_a_fade_out_falls_from_one_to_zero_and_holds`, `fv_10_a_two_second_fade_in_rises_from_zero_to_one_continuously`; `arq_111_reset_restarts_the_fade` |
| 8 | `cos` and `sin` swapped; `p` unclamped, lower only or upper only | `arq_124_equal_power_is_cos_for_the_outgoing_side_and_sin_for_the_incoming`, `arq_124_equal_power_clips_p_to_zero_and_one` |
| 9 | `θ` with `π/2` or without `+1`; no `min(1, …)` on L or R; no `√2`; L and R swapped | `arq_124_pan_follows_the_minus_three_db_law_over_the_whole_range`, `arq_124_pan_centre_leaves_the_block_equal`, `arq_124_pan_half_right_gives_0_5412_on_the_left_and_the_right_equal`, `arq_124_pan_hard_left_silences_the_right_channel` |
| 9 | pan changes a non-stereo block (`>= 1`, `>= 2`, `<= 2`) | `arq_124_pan_changes_nothing_on_a_block_that_is_not_stereo`, `arq_123_pan_cleans_non_finite_samples_on_any_block` |
| 10 | chain reversed; latency `max`, first only, wrapping; `prepare` or `reset` not forwarded | `arq_129_the_chain_applies_its_processors_in_order`; `arq_129_the_latency_of_a_chain_is_the_sum`; `arq_129_a_latency_sum_that_overflows_saturates_without_panic`; `arq_129_prepare_and_reset_reach_every_processor` |
| 11 | mix adds a muted track; mix accumulates into `out`; mix clips | `arq_129_a_muted_track_adds_nothing`, `arq_129_with_every_track_muted_or_none_the_output_is_silent`, `arq_129_the_mix_of_16_tracks_matches_the_manual_sum`, `arq_123_the_mix_does_not_clip_at_zero_dbfs` |
| 12 | half toward 0 (`>` for `>=`); `+1` for the sign; `i64` multiply; floor rounding; `f64` formula (two forms) | `arq_129_a_half_sample_rounds_away_from_zero`, `arq_129_exact_halves_at_huge_offsets_round_away_from_zero`, `arq_129_the_extremes_of_i64_do_not_overflow`, `arq_129_the_result_matches_the_independent_oracle` |

Round-0 survivors, fixed before the last commit: a `max_block != 0` clause that no input could reach (removed), a rounded `f64` ramp length that could not differ from `floor` at 44.1, 48 and 96 kHz (now integer `sample_rate / 100`), and a muted-track test that the stub passed (now also asserts a non-zero reference).

### Deviations and decisions the specs leave open

- `ParamSnapshot` pairs are `(&'static str, f64)`: ids are the `ParamDef` ids, values are `f64` like `ParamValue::Float`. ADR 0005 does not fix the types. `set` replaces an existing id and refuses the 17th id (returns `false`).
- Parameter ids: `volume` (percent, clipped to 0..200 as FV-10), `duration` (seconds, `>= 0`), `pan` (`-1..1`). `Gain` takes percent because `-∞ dB` cannot travel as a parameter (ARQ-123 ignores non-finite ones); `db_to_gain` is the pure function (`-∞` gives 0 while NaN, `+∞` and an overflow give `Err` naming `db`).
- `AudioError` has one variant, `NonFinite { field }`; message key `error.audio.non_finite` with argument `field`, following `LayoutError`. No spec names it.
- `crossfade_gains(mode, p)` returns `(outgoing, incoming) = (cos, sin)` as `Result<(f64, f64), AudioError>`; `p` is clipped to `[0, 1]`.
- Overflow of a finite sample (for example `f32::MAX` at 200 %, or a mix sum) saturates at `±f32::MAX`: ARQ-123 says the output stays finite and does not say how. A NaN or `±∞` input sample is 0.
- Channels of different lengths count as an invalid block and stay intact (the same rule as a 0 in `prepare`). ADR 0005 says only "equal lengths".
- Gain ramps from 1.0 on the first change (no jump to the first value). `reset` ends a running ramp at its target and keeps parameters; `Fade` counts samples from `prepare` or `reset` and holds its end value, so `gafira-media` schedules a fade-out by resetting at its start.
- `ProcessorChain` does not override `tail()`; no AU1 processor has a tail.
- `mix` takes `MixTrack { channels: &[&[f32]], muted }` and overwrites `out`; tracks with fewer channels or frames add what they have.
- `flicks_to_samples(Flicks, SampleRate) -> i64` uses `i128`; a valid `SampleRate` is at most 705 600 000 Hz, so the result never exceeds the flick count in magnitude.
- The block and snapshot tests (`block.rs`) and the determinism tests were not RED (see the table). The 16-track `bench_mix_16`, `assert_no_alloc` guard and EQ are left to AU2 as the card says.

### Size

`git diff --shortstat 9bfba1c..HEAD` before this note: 21 files, 2 022 insertions, of which `src/` is 714 lines and `tests/` about 1 300. Source is over the 670 forecast at x1.4 and the delivery is over ~700 lines in total; the coordinator decides.

### AU1 fix round 1 (verify round 1 FAIL)

Commits: `3c9481d` (unprocessed blocks, empty chain, chain tail), `f5b9bd8` (first gain value, `Fade` latch, C2, C3), `3e6cc3b` (`params()`, P13), `29c620d` (W6, mix 2 onto 3), `ca5bfbb` (prepare re-arm), plus this note. Base for the amended rules: `develop` merged at `a96494a`. Every code commit passed `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace --locked`, each alone with exit code 0.

| Finding | Fix | Evidence |
|---|---|---|
| C1 | An empty `ProcessorChain` cleans the block; a non-empty one lets each stage clean | RED `arq_123_an_empty_chain_cleans_the_block_and_keeps_normal_samples`: `[NaN, inf, -inf, 5.88e-39, ...]` where `[0.0, 0.0, 0.0, 0.0, ...]` was wanted. The old `arq_129_an_empty_chain_changes_nothing` (pinned `is_nan()`) is rewritten |
| ARQ-123 amended | The four unprocessed cases (and never prepared) only clean, per channel over its own length, and leave the state untouched; `Setup::accepts` also checks the channel count | RED `arq_123_an_unprocessed_block_is_only_cleaned` (`gain 0 64 2 2`), `arq_123_an_unprocessed_block_leaves_no_trace_in_the_state`, `arq_123_channels_of_different_lengths_are_cleaned_by_their_own_length`, `arq_123_a_processor_that_was_never_prepared_only_cleans_the_block` |
| ARQ-124 first value (W1) | First accepted block after `prepare` or `reset` applies its value from sample 0; no value means last valid or default; a refused block does not consume it | RED `arq_124_the_first_value_applies_from_sample_zero_without_a_ramp`, `arq_124_after_reset_the_next_value_applies_at_once_too`. Not RED: `arq_124_a_first_block_without_a_value_uses_the_last_valid_one`, `arq_124_a_first_block_without_a_value_then_a_zero_ramps` (the old code did the same through `reset`) |
| ARQ-124 `Fade` (W3) | `duration` latched with the first accepted block of a ramp until `reset`; In and Out are separate instances; fades moved out of the chains in `total.rs` and `property.rs` | RED `arq_124_the_duration_is_read_once_per_ramp_and_a_change_waits_for_reset`, `arq_124_going_from_duration_zero_to_two_seconds_waits_for_reset_too`. Not RED: `fv_10_the_fade_in_and_the_fade_out_have_independent_durations` |
| P13 (W2) | `params()` required on the trait; `schema: ParamSchema` field built in `new` from `dsp::Param::def` (range and default exist once); chain returns an empty schema; `new` of `Gain`, `Fade`, `Pan` no longer `const` | RED (compile stub with an empty schema) `p13_every_processor_schema_passes_validate_definition_and_matches_the_table`: `params().len()` 0, want 1. Not RED: `p13_a_chain_has_an_empty_schema_whatever_it_holds` (proved by mutant P1), `arq_123_values_beyond_the_schema_bounds_are_clipped_to_them` |
| C2 | Stereo fade tests, In and Out, 2 s, slope and ends | Not RED, mutant H1 |
| C3 | Mid-ramp target change tests, and a repeated same target | Not RED, mutant H2 |
| W5 | `ProcessorChain::tail()` is the saturating sum of its stages | RED `arq_129_the_tail_of_a_chain_is_the_sum_and_saturates`: left 0, right 18 |
| W6 | Double-run tests for `db_to_gain`, `percent_to_gain`, `crossfade_gains`, `flicks_to_samples` | Not RED; pure functions, determinism holds by construction, no mutant targets them |
| C4, mix | No code change: the amended ARQ-129 makes `mix` total. Added `arq_129_a_two_channel_track_on_a_three_channel_destination_feeds_the_first_two` | Not RED (already true) |

### Mutants of the report and of the new rules (killed, then reverted)

| Mutation | Killed by |
|---|---|
| H1 fade clock advances per channel sample | `fv_10_a_stereo_fade_runs_at_the_same_speed_on_both_channels` |
| H2 ramp step from the old target | `arq_124_a_change_during_a_ramp_continues_from_the_current_gain`, `arq_124_a_downward_change_ramps_too` |
| H3 saturation loses its sign | `arq_123_the_mix_stays_finite_when_the_sum_overflows_f32`, `arq_123_there_is_no_clipping_at_zero_dbfs` |
| Empty chain not cleaned; `clean_block` a no-op | `arq_123_an_empty_chain_cleans_the_block_and_keeps_normal_samples`, `arq_123_a_non_finite_sample_counts_as_zero` |
| Unprocessed block not cleaned (gain, fade, pan); only the first channel cleaned | `arq_123_an_unprocessed_block_is_only_cleaned`, `arq_123_channels_of_different_lengths_are_cleaned_by_their_own_length` |
| Channel count not checked; `channels == 0` accepted; `sample_rate` 0 processed | `arq_123_an_unprocessed_block_is_only_cleaned`, `arq_123_an_unprocessed_block_leaves_no_trace_in_the_state` |
| Parameter read before the accept check | `arq_123_an_unprocessed_block_leaves_no_trace_in_the_state` |
| Refused block advances the fade clock | `arq_123_a_refused_block_does_not_advance_the_fade` |
| Chain tail 0, `max`, wrapping | `arq_129_the_tail_of_a_chain_is_the_sum_and_saturates` |
| First value ramps; `reset` or `prepare` does not re-arm | `arq_124_the_first_value_applies_from_sample_zero_without_a_ramp`, `arq_124_after_reset_the_next_value_applies_at_once_too`, `arq_124_prepare_again_makes_the_next_value_a_first_one` |
| Duration re-read each block; `reset` or `prepare` keeps the latched length | `arq_124_the_duration_is_read_once_per_ramp_and_a_change_waits_for_reset`, `arq_124_going_from_duration_zero_to_two_seconds_waits_for_reset_too`, `arq_124_prepare_again_reads_the_duration_afresh` |
| Chain returns a stage schema; `animatable` flipped (gain, pan, fade); id, label, min, max or default changed in `def`; gain range changed at the source | `p13_a_chain_has_an_empty_schema_whatever_it_holds`, `p13_every_processor_schema_passes_validate_definition_and_matches_the_table`, `arq_123_parameter_out_of_range_is_clipped` |

The 12 round-0 mutants (list 1 to 12) were re-applied on this code and all stay killed, except the one equivalent noted before: `clean_output` leaving NaN, still unreachable through the public API. The `channels != 0` clause was equivalent after the channel-count check and is removed.

### Deviations and decisions in this round

- `ProcessorChain::tail()` is a saturating sum, like `latency()`. No spec says sum or max; a sum is the safe upper bound for stages in series.
- `Fade` reads `duration` on every accepted block (to keep the last valid value current) but latches the length on the first one after `prepare` or `reset`.
- A refused block does not read parameters, so it cannot change the last valid value.
- The `Op` double in `tests/chain.rs` has an empty schema.
- Size of this round: `git diff --shortstat e1afac9..ca5bfbb`: 22 files, 1 036 insertions, 104 deletions, mostly tests. The AU1 total stays over the ~700 line signal.

### AU1 fix round 3 (verify round 2: C5, W9; amended ARQ-123 and ARQ-124, ac411a3)

Commit: the `fix(audio)` commit of this round. Gates for it: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, each exit code 0.

- Code: `Setup::accepts` also requires `frames() > 0` (`crates/gafira-audio/src/dsp.rs`). Gain, Fade and Pan share it, so no other change was needed. `Fade` already read `duration` on each processed block, so the amended rule (last valid value counts at the latch, later ones only become the last valid) held; W9 needed a test only.
- Tests: `crates/gafira-audio/tests/empty.rs`.

RED evidence (before the production change):

| Criterion | Test and assertion |
|---|---|
| ARQ-123 empty block, valid prepare | `arq_123_an_empty_block_after_a_valid_prepare_leaves_the_next_block_untouched`: next block not x 1.0 |
| ARQ-123 empty block, prepare with a zero, then valid prepare | `arq_123_an_empty_block_after_a_prepare_with_a_zero_leaves_a_later_prepare_untouched`: same assertion |
| ARQ-124 gain first block | `arq_124_an_empty_block_is_not_the_first_block_of_a_gain`: `[0.9979, 0.9958, ...] != [0; 8]` (empty then volume 0 ramped) |
| ARQ-124 fade latch | `arq_124_an_empty_block_does_not_latch_the_fade_length`: `out[0]` 1.0, want 0.0 |
| ARQ-124 1/960 (empty block mid-ramp) | `arq_124_a_duration_in_an_empty_block_mid_ramp_is_not_read`: step error above 1e-6 |
| W9, 1/240 and 1/1920 | `arq_124_a_duration_in_a_processed_block_mid_ramp_becomes_the_last_valid_one` and `arq_124_the_block_after_reset_sets_the_length_with_its_own_duration` passed already (behaviour held); pinned by mutants 5 and 6 |

Hand mutants (killed, then reverted; none committed):

| Mutation | Killed by |
|---|---|
| 1 `frames() == 0` refusal removed | all five RED tests above |
| 2 empty block consumes the first value (Gain) | `arq_124_an_empty_block_is_not_the_first_block_of_a_gain` |
| 3 empty block latches the fade length | `arq_124_an_empty_block_does_not_latch_the_fade_length`, `arq_123_an_empty_block_after_a_valid_prepare_leaves_the_next_block_untouched` |
| 4 refused non-empty (oversized) block latches the length | `arq_123_an_unprocessed_block_leaves_no_trace_in_the_state` (existing) |
| 5 duration read only at the latch | `arq_124_a_duration_in_a_processed_block_mid_ramp_becomes_the_last_valid_one` |
| 6 later processed duration not stored as last valid | same test |

Deviations: none. Size: see `git diff --shortstat` of the round (test file about 175 lines, 4 lines of code); well under 300.

## AU2

Branch `feat/gafira-audio`, from `develop` d437b8a. Commits: `29d4314` (test-only: AU1 follow-ups W10 to W13, S4), `0f7d798` (`Eq3Band`, ARQ-125, FV-20 (c)), and the commit of this note (ARQ-23 guard, testkit negative control, `bench_mix_16`, CAL-15). Gates for each commit, each read alone: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, all exit code 0.

### Follow-ups of AU1 (first commit, tests only)

`tests/refused.rs` (4 tests) pins behaviour that already held; each test passed from the start and was proved by scratch mutants, all killed and reverted:

| Finding | Test | Mutants killed |
|---|---|---|
| W10 | `arq_124_a_refused_block_does_not_consume_the_first_block_of_a_gain` (six refused shapes) | F2 (non-empty refused block, and any refused block, sets `first = false`) |
| W11 | `arq_123_a_refused_block_after_a_prepare_with_a_zero_reads_no_parameter` (the three skipped rows, then a valid `prepare`); comment at `total.rs` corrected | R4 on `Gain`, `Fade`, `Pan` |
| W12 | `arq_123_a_refused_block_during_a_running_state_leaves_the_next_samples_as_they_were` (running ramp and latch; block 2's duration differs from block 1's so a cleared latch shows) | F3, F4, F5, L3, L4, L4b, L5 |
| W13 | `arq_124_the_last_valid_value_survives_prepare` | D4, V1, V3 |
| S4 | doc comments of `ProcessorChain::new` and `Pan` | n/a |

### Done

- `Eq3Band` (`src/eq.rs`): ids `low`, `mid`, `high` (dB, `[-15, 15]`, default 0, not animatable, labels `audio.eq3band.*`); low shelf 100 Hz `S = 1`, peak 1 kHz `Q = 1`, high shelf 8 kHz `S = 1`; RBJ cookbook coefficients and direct form I in `f64`, one history per channel and band; a band at 0 dB, or with `2 * f0 >= fs`, passes the signal.
- Tests: `tests/eq.rs`, `eq_contract.rs`, `eq_state.rs`, `alloc.rs` (ARQ-23 positive, 16 tracks of Gain, Eq3Band, Pan plus both fades, 512 frames, 8 rounds of changing parameters), `gafira-testkit/tests/alloc_guard.rs` (negative control in a subprocess: a clean child succeeds, an allocating child dies with SIGABRT), `eq3band` added to the makers of `total.rs`, `empty.rs`, `refused.rs`.
- `benches/mix.rs`, `bench_mix_16` (CAL-15): 16 tracks, Gain, Eq3Band, Pan, 512 frames at 48 kHz, plus the mix. **Figure: 255 us per block (interval 235 to 280 us) against a 2.67 ms budget (25 % of 10.67 ms): about 2.4 % of one core.** Measured with `cargo bench` on the developer machine, not the reference one.

### RED evidence (a stub that only cleans the block, with the right API, existed before the tests)

| Criterion | Test and assertion |
|---|---|
| ARQ-125 response | `arq_125_the_impulse_response_matches_h_within_half_a_db_from_20_hz_to_20_khz`: `fs 44100 gains [15.0, 0.0, 0.0] f 20.0: got 0.000 dB, want 14.962 dB` |
| FV-20 (c) | `fv_20_c_a_sine_sweep_follows_the_theoretical_curve`: `f 20: got -0.000 dB, want 8.976 dB` |
| ARQ-125 omission | `arq_125_a_band_at_or_above_half_the_sample_rate_is_omitted`: `!run_pair(16_001, ...)` failed (an active band gave the output of 0 dB) |
| P13, 02 4.17.5 | `p13_the_schema_has_the_three_bands_of_the_table`: `left == right` failed (1 parameter, 3 wanted) |
| ARQ-123 clip | `arq_123_a_band_beyond_15_db_is_clipped_to_15_and_at_the_extremes_it_is_kept`: `band 0 +15` left != right failed |
| ARQ-125 gain change | `arq_125_a_gain_change_applies_from_the_next_block_and_keeps_the_history`: `fs 44100 step 1 sample 0: got 3.265037, want 3.571753970696067` |
| Not RED (pass against the pass-through stub by construction; each proved by the mutants below) | `arq_125_with_every_band_at_0_db_the_output_is_bit_identical`, `arq_125_every_pole_is_inside_the_unit_circle`, `arq_111_reset_*`, `arq_111_prepare_*`, `arq_125_each_channel_has_its_own_history`, `arq_123_the_history_is_flushed_below_the_smallest_normal_sample`, the non-finite and unknown-id tests, the three new makers, `alloc.rs` and the testkit control (the code was already allocation-free; the control is proved by removing the guard) |

### Hand mutants (killed, then reverted; none committed)

| Mutation | Killed by |
|---|---|
| Shelf frequencies swapped (low 8 kHz; high 100 Hz); mid at 1.2 kHz | `arq_125_the_impulse_response_*`, `fv_20_c_*`, `arq_125_a_band_at_or_above_*`, `arq_125_a_gain_change_*` |
| Peak `Q` 2 and 0.5 | `arq_125_the_impulse_response_*`, `fv_20_c_*` |
| Shelf `S` not 1 (low and high separately; factors 1.0, 1.2, 1.6) | the same two |
| Range bound strict (`abs < 15`, so +-15 is ignored) | `arq_123_a_band_beyond_15_db_*`, the response tests |
| History not cleared by `reset`; kept by `prepare` | `arq_111_reset_clears_the_history_of_every_band`; `arq_111_prepare_clears_the_history_of_every_band` |
| Coefficients only on `prepare` (change ignored); only for the first non-zero gain; change clears the history; change applied one block late | `arq_125_a_gain_change_applies_from_the_next_block_and_keeps_the_history` (and the response tests for the first) |
| Subnormal history not flushed; flushed at the `f64` threshold | `arq_123_the_history_is_flushed_below_the_smallest_normal_sample` |
| Channels share one history | `arq_125_each_channel_has_its_own_history` |
| Refused non-empty block clears the history, or is filtered anyway, or reads parameters | `arq_123_a_refused_block_during_a_running_state_*`, `arq_123_a_refused_block_after_a_prepare_with_a_zero_*`, `arq_123_an_unprocessed_block_leaves_no_trace_in_the_state` |
| Empty block clears the history, or reads parameters | `arq_123_a_refused_block_during_a_running_state_*`, `arq_123_an_empty_block_after_*` |
| 0 dB band not bypassed (spec demands identity, so it is checked) | first survived the bit-identical test: with identity coefficients `f64` rounding returns the same `f32`. Fixed with a block of `1e30` followed by small samples, which the cancellation corrupts: now `arq_125_with_every_band_at_0_db_the_output_is_bit_identical`. Also `a_gain_change_*` |
| Bypass without history; omission `>` for `>=`; `a1` sign; peak `b0` with `alpha / a`; `db / 20` for `db / 40` | `a_gain_change_*`; `arq_125_a_band_at_or_above_*`; the response tests |
| An allocation (`vec!`, `Box`) in `Eq3Band::process`, `mix`, `Gain::process`, `ProcessorChain::process` | `arq_23_sixteen_tracks_*` (the process aborts with SIGABRT) |
| Guard removed from the testkit child | `arq_23_an_allocation_inside_the_guard_aborts_the_process` |

### Adversarial sweep (throwaway test, not committed)

155 907 checks over `Gain`, both `Fade`s, `Pan`, `Eq3Band` and a Gain, Eq, Pan chain: six stream shapes (rates 1 Hz to 705.6 MHz, `max_block` 1 to 512, 1 and 2 channels), six parameter sets (0, +-15, 200, 1e300, `f64::MAX`, NaN, infinities, `-0.0`, out of range), block sizes 1, 7 and 500 (output bit-identical whatever the split), five kinds of refused or empty block inserted every third block (output unchanged), `reset` and `prepare` after 1 to 4 blocks (output equals a fresh processor), Eq3Band at rates 1 to 705.6 MHz around the `fs / 2` boundaries, the 0 dB bypass at sizes 1, 7, 500, and the pure functions at extremes. Result: no finding; one failure in the sweep itself (a one-channel "lopsided" block is valid), fixed in the sweep. No code change followed.

### Deviations and decisions the specs leave open

- A gain change of `Eq3Band` applies from the first sample of the next processed block, with no ramp, and the histories carry on (ARQ-125 is silent; the bands are not animatable). A band at 0 dB, or omitted, passes the signal and its history follows it, so a later gain starts from a consistent state.
- The history is flushed to 0 when its output is below `f32::MIN_POSITIVE` (the same bound as the output rule of ARQ-123); a flushed sample bends the tail by a few samples, so the test starts 100 samples after the ring-down.
- "All poles inside the unit circle" is proved from outside by the ring-down of the impulse response (a pole on the circle would keep it ringing), because the coefficients are private. At fs = 16 001 Hz the 8 kHz shelf decays slowly, so the test asks for half the amplitude per 16 384 samples.
- `Eq3Band` has `tail() == 0` (default): ARQ-125 declares none.
- The `makers` of `total.rs`, `empty.rs`, `refused.rs` now include `eq3band`, and their snapshots carry `low`, `mid`, `high`.
- Size: `git diff --shortstat d437b8a` on `crates/`: 14 files, 1 380 insertions (268 in `src/`, the rest tests and the bench), before this note. Forecast 400 (560): over it, and over ~700 lines in total; the coordinator decides.

### AU2 fix round 1 (verify round 1: C6, C7, C8, W14; W15 is process-only)

Commits: `9ecc6c3` (tests for C6, C7, C8; no code change) and `595487b` (`Eq3Band::tail`, ARQ-125 as worded in `b9442ae`). Each commit passed `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace --locked`, each exit code 0 and read alone.

Tests added:
- C6: `arq_123_a_non_finite_sample_counts_as_zero_now_and_in_the_history` (`total.rs`, over every maker: NaN, +inf, -inf equals a 0, in the block and in the next one).
- C7: `arq_125_prepare_at_another_rate_designs_for_the_new_rate_with_the_last_valid_value` (`refused.rs`, every maker) and `arq_125_a_band_omitted_at_the_new_rate_does_nothing_after_prepare` (`eq_state.rs`).
- C8: `arq_124_the_last_valid_value_survives_reset` (`refused.rs`, over `makers()`, twin of the `prepare` test).
- Same three gaps on Gain, Fade and Pan: C6 on all three was already killed (`arq_123_a_non_finite_sample_counts_as_zero`, `arq_123_pan_cleans_non_finite_samples_on_any_block`); C7 (Gain `ramp_len`, Fade `reset` in `prepare`) already killed (`arq_129_prepare_and_reset_reach_every_processor`, the ramp tests, `arq_124_prepare_again_reads_the_duration_afresh`); Pan has no state to redesign. C8 on Gain, Fade and Pan was only killed incidentally by tests that do not name it; the new reset twin now kills it for all.

RED for W14 (tests before code, `tail()` was the default 0): `arq_125_tail_is_forty_milliseconds_in_frames_with_a_band_active` (`left: 0, right: 1764`, fs 44100), `arq_125_tail_is_zero_without_an_active_band` (`left: 0, right: 641`, high at 16 001 Hz), `arq_125_tail_follows_the_last_valid_gain` (`left: 0, right: 1920`), `arq_125_tail_is_kept_by_reset_prepare_at_the_same_rate_and_a_refused_block` (`left: 0, right: 1920`), `arq_125_after_an_impulse_every_sample_from_tail_is_below_1e_6` (panic at frame 0 of the ring-down). Not RED by construction: `arq_125_a_gain_change_applies_from_sample_0_of_the_block_that_carries_it` (1e-6 against the reference at 44.1, 48 and 96 kHz; behaviour already held).

Tail: active band = centre < fs/2 and last valid gain not 0 dB (`Band::is_active`, `Param::value`); `tail = ceil(4 fs / 100)` in `u64`. Figures checked: 1 764, 1 920, 3 840, 1 765, 171 798 692; 0 for 0 dB, -0 dB, not prepared, fs 0, high only at 16 000 Hz; 641 at 16 001 Hz; below 1e-6 from frame `tail()` over the 26 active combinations at three rates.

Hand mutants (applied, killed, reverted):

| Mutant | Killed by |
|---|---|
| C6 `clean_input` removed in `Eq3Band` | `arq_123_a_non_finite_sample_counts_as_zero_now_and_in_the_history` |
| C6 removed in Gain, Fade, Pan | the existing non-finite tests, and the new one for Gain and Fade |
| C7 no redesign in `Eq3Band::prepare` | `arq_125_a_band_omitted_at_the_new_rate_*`, `arq_125_prepare_at_another_rate_*` |
| C7 Gain `ramp_len` not set in `prepare`; Fade `prepare` without `reset` | `arq_129_prepare_and_reset_reach_every_processor`, ramp tests; `arq_124_prepare_again_reads_the_duration_afresh` |
| C8 `Eq3Band::reset` forgets the gains; same on Gain, Fade, Pan | `arq_124_the_last_valid_value_survives_reset` (all four) |
| tail: `floor` instead of `ceil`; band omission removed; `<=` for `<`; 0 dB not excluded; always active | `arq_125_tail_is_forty_milliseconds_*`, `arq_125_tail_is_zero_without_an_active_band`, `arq_125_tail_follows_the_last_valid_gain`, `arq_125_tail_is_kept_by_reset_*` |
| tail: `reset` forgets the gains | `arq_125_tail_is_kept_by_reset_*`, `arq_124_the_last_valid_value_survives_reset` |
| tail: `unwrap_or(u32::MAX)` to `unwrap_or(0)` | Survives, equivalent: `4 fs / 100` never exceeds `u32::MAX`, so the branch cannot run |

Deviation: `Eq3Band` tail is read from the last valid gains, so `tail()` before the first `process` is 0 even if a later snapshot will set a band (ARQ-125 says the last valid value, or the default). W15: nothing to change in this code.

## AU3

Order: one commit per unit (meter, true peak and leveling plan, `LoudnessNormalizer`). For each unit the tests were written first and run against a compiling stub that returns the empty answer (`Ok(None)`, `0.0`, a no-op `process`); the failing line is pasted below. Every commit passed `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace --locked`, each exit code read alone.

### Unit 1: integrated loudness (ARQ-126, FV-20 d, ARQ-110, ARQ-111)

RED, stub `integrated_loudness -> Ok(None)` and `integrated_from_blocks -> Ok(None)` (`tests/loudness.rs`; 18 of 21 failed):

| Criterion | Failing line |
|---|---|
| EBU Tech 3341 cases 1 to 5 | `arq_126_ebu_3341_case_1_*` .. `case_5_*`: panicked at `.expect("a measure")` (the meter said `None`) |
| 997 Hz stereo, -23.0 at 44.1, 48, 96 kHz | `arq_126_a_997_hz_stereo_sine_*`: the same `expect` |
| 6 channels, one tone at index 0..5 | `arq_126_six_channels_*`: `channel 0: got Ok(None), want Some(-23.0)` |
| 399 ms / 400 ms | `arq_126_a_signal_under_400_ms_*`: `left: None, right: Some(())` |
| 3 channels `Err` | `arq_126_a_channel_count_*`: `left: Ok(None) right: Err(ChannelCount { channels: 0 })` |
| non-finite sample | `arq_110_a_non_finite_sample_*`: `left: Ok(None) right: Err(NonFinite { field: "samples" })` |
| unequal lengths | `arq_110_channels_of_different_lengths_*`: `left: Ok(None) right: Err(OutOfRange { field: "channels" })` |
| non-finite energy | `arq_110_a_non_finite_or_negative_block_energy_*`: `left: Ok(None) right: Err(NonFinite { field: "energies" })` |
| absolute gate at exactly -70 | `arq_126_a_block_at_exactly_minus_70_lufs_*`: `expect` on `None` at the last assertion line (the "kept above" half) |
| relative gate | `arq_126_the_relative_gate_*`: `expect("a measure")` on `None` |
| K weighting at other rates (+-0.1 dB, 40 Hz to 15 kHz, 5 rates) | `arq_126_the_k_weighting_at_other_rates_*`: `expect` on `None` |

Passed from the start (the stub answer is the right one), each proved by a mutant below: `arq_126_silence_has_no_measure_*`, `arq_126_no_blocks_or_only_silent_ones_*`, `arq_111_the_meter_gives_the_same_bytes_twice`.

The K weighting at other rates passed on the first implementation (the bilinear design of the BS.1770-4 analogue prototype is within 0.1 dB of the 48 kHz response for 32, 44.1, 88.2, 96 and 192 kHz up to 15 kHz); it is proved by mutant M1.

Hand mutants (applied, killed, reverted):

| Mutant | Killed by |
|---|---|
| M1 K weighting designed at 48 kHz for every rate | `arq_126_the_k_weighting_at_other_rates_*`, `arq_126_a_997_hz_stereo_sine_*` |
| M2 hop 200 ms; block of 3 hops; block of 2 hops | `arq_126_a_signal_under_400_ms_*`; the 997 Hz, mono, six-channel and EBU cases |
| M3 absolute gate `>=` for `>` (also the shared gate function) | `arq_126_a_block_at_exactly_minus_70_lufs_*` |
| M3 absolute gate -69 | the same test |
| M4 relative gate against the mean of all blocks; against the mean of the loudness values | `arq_126_the_relative_gate_is_10_lu_under_the_mean_energy_*`, `case_3` |
| M4 relative gate -9, -11, -10.1, -9.9, -10.5 | `arq_126_the_relative_gate_is_exactly_10_lu_under_the_mean` (first -10.5 survived; two blocks 12.7 and 12.85 LU under, a 0.08 LU gap, kill it) |
| M5 surround weight 1.0 | `arq_126_six_channels_*` |
| M6 LFE counted | `arq_126_six_channels_*` |
| M7 3 channels accepted | `arq_126_a_channel_count_*` |
| M8 silence gives `-inf` | `arq_126_silence_has_no_measure_*` |
| M9 399 ms measured | `arq_126_a_signal_under_400_ms_*` |
| determinism: a call counter added to the result | `arq_111_the_meter_gives_the_same_bytes_twice` |
| `ChannelCount` message argument fixed to 0 | `arq_102_the_new_errors_give_a_key_with_their_data` |

A first version kept the absolute gate in two places (the second pass was strict, the first was the one a mutant changed): the `>=` mutant survived. The gate is now one function, `louder_than`, used twice.

Decisions the specs leave open (unit 1):
- The gate comparisons are strict (`>`), as BS.1770-4 writes them; ARQ-126 does not say. "A block exactly at -70 LUFS" is not reachable by a filtered signal, so `integrated_from_blocks` (energies in, loudness out) is public as the seam where the boundary is tested, with the energy found by search so that `-0.691 + 10 log10(e)` is -70.0 to the last bit.
- The error set grew by `OutOfRange { field }` (unequal channel lengths, a negative energy) and `ChannelCount { channels }`; both have message keys.
- Channels of unequal length are an `Err`, not measured over the shortest.
- A rate whose 100 ms is not a whole number of samples rounds it to the nearest sample (`(fs + 5) / 10`), and the block is 4 hops. For 44.1, 48 and 96 kHz it is exact.
- EBU Tech 3341 cases 1 to 5 are synthesised from the published description as I know it: 1 and 2 are single levels (-23, -33 LUFS); 3 is -36, -23, -36 (10, 60, 10 s); 4 and 5 are my own sequences that exercise the gates (4: -72, -40, -23, -40, -72 for 10, 10, 40, 10, 10 s; 5: -26 for 20 s then -20 for 10 s), each with the expected value worked out from the gates. The files were not at hand, so 4 and 5 may differ from the EBU originals.

### Unit 2: true peak and the Nivelar gain (ARQ-127, FV-10 b, ARQ-110, ARQ-111)

RED, stub `true_peak -> Ok(0.0)`, `plan_leveling` and `level -> Ok(Leveling { gain_db: 0.0, reached: false })` (`tests/leveling.rs`; 11 of 16 failed):

| Criterion | Failing line |
|---|---|
| fs/4 sine peaks at full scale between samples | `arq_127_a_sine_at_a_quarter_of_the_rate_*`: `8000 Hz: ... dBFS` assertion (the stub gave 0) |
| peak never under the sample peak, sees every channel | `arq_127_the_true_peak_is_never_under_the_sample_peak_*` |
| a lone sample peaks at itself | `arq_127_a_lone_sample_peaks_at_itself`: `got 0, want 0.5 +-0.000001` |
| smaller of request and ceiling | `arq_127_the_applied_gain_is_the_smaller_*`: `left: (0.0, false) right: (2.0, false)` |
| equality reaches the target | `arq_127_a_request_equal_to_the_ceiling_*`: `left: (0.0, false) right: (6.0, true)` |
| Nivelar example, -30 LUFS and -3 dBTP | `fv_10_b_a_quiet_signal_with_a_high_peak_*`: `got 0, want 2 +-0.01` |
| Nivelar example, -20 LUFS with headroom | `fv_10_b_a_signal_at_minus_20_lufs_*`: assertion `p.reached` |
| channel count and samples of the peak | `arq_110_the_peak_takes_the_same_channel_counts_*`: `left: Ok(0.0) right: Err(ChannelCount { channels: 0 })` |
| plan names its input | `arq_110_the_plan_names_the_non_finite_input`: `left: Ok(Leveling {..}) right: Err(NonFinite { field: "target" })` |
| request overflow | `arq_110_a_request_that_overflows_is_an_error`: `right: Err(NonFinite { field: "gain" })` |
| `level` checks its input | `arq_110_level_checks_the_target_and_the_signal` |

Passed from the start (the stub answer is the right one), each proved by a mutant: `arq_127_silence_and_an_empty_signal_have_a_peak_of_zero`, `arq_127_without_a_measure_the_gain_is_zero_*`, `arq_127_silence_and_a_signal_under_400_ms_*`, `arq_111_the_peak_gives_the_same_bytes_twice`, `arq_111_level_gives_the_same_bytes_twice`.

Hand mutants (applied, killed, reverted):

| Mutant | Killed by |
|---|---|
| True peak without oversampling at 48 kHz or less (x1) | `arq_127_a_sine_at_a_quarter_of_the_rate_*` |
| x2 at 48 kHz or less | the same test; it survived the first version (only the 45 degree phase, which x2 also sees) and the phases 67.5 and 22.5 degrees now kill it |
| x8 at 48 kHz or less | survives, equivalent: x8 only measures a higher crest by a hair |
| Applied gain the larger of request and ceiling | `arq_127_the_applied_gain_is_the_smaller_*`, both `fv_10_b_*` |
| Equality does not reach the target (`<` for `<=`) | `arq_127_a_request_equal_to_the_ceiling_*` |
| Ceiling -2 dBTP | `arq_127_a_request_equal_to_the_ceiling_*`, `fv_10_b_a_quiet_signal_*` |
| No measure: gain 1; reached true | `arq_127_without_a_measure_*`, `arq_127_silence_and_a_signal_under_400_ms_*` |
| LFE left out of the peak | `arq_127_the_true_peak_is_never_under_the_sample_peak_*` |
| Silence peak 1e-9 | `arq_127_silence_and_an_empty_signal_*` |
| Counter added to the peak; to the target of `level` | `arq_111_the_peak_gives_*`; `arq_111_level_gives_*` |
| `level` ignores the peak | `fv_10_b_a_quiet_signal_with_a_high_peak_*` |

Decisions the specs leave open (unit 2):
- API: `true_peak` (linear, all channels, LFE included), `plan_leveling(target, measured: Option, peak_dbtp: Option) -> Leveling { gain_db, reached }` (the pure decision, so the equality is testable) and `level(channels, rate, target)`, which measures and plans. A missing peak means no ceiling.
- The interpolation is a Kaiser-windowed sinc of 24 taps per phase (beta 8), each phase normalised to sum 1; not the 48-tap table of Annex 2, whose coefficients are not reproduced here. Its error on a sine crest is under 0.01 dB (the test asks for 0.01). The signal is zero outside its ends, so an abrupt start or end overshoots; that is part of the peak.
- Above 48 kHz: x2 up to 96 kHz, none above (Annex 2). ARQ-127 pins only x4 at 48 kHz or less.
- The "peak <= -1 dBTP" check after applying the gain allows 1e-3 dB, for the rounding of the `f32` samples.

### Unit 3: `LoudnessNormalizer` (ARQ-127, SP19, ARQ-123, ARQ-23)

RED, stub with the right API and an empty schema, and a `process` that does nothing (`tests/normalizer.rs`, plus the new `loudness_normalizer` entry in `params.rs`, `total.rs`, `refused.rs`, `empty.rs`, `alloc.rs`):

| Criterion | Failing line |
|---|---|
| +16 dB with the Volume at 100 %, Volume untouched | `arq_127_a_gain_of_16_db_is_applied_with_the_volume_at_100_percent_untouched`: `[0.1, 0.1, 0.1, 0.1]` against x6.3096 |
| the clip's Volume does not reach the normalizer | `arq_127_the_volume_of_the_clip_does_not_reach_the_normalizer`: `assertion failed: ... (x - want).abs() <= 1e-6` |
| constant over the signal, across blocks | `arq_127_the_gain_is_constant_over_the_whole_signal_across_blocks`: `sample 1: 0.039983336 vs 0.017253572` |
| a new gain from sample 0 of the next block | `arq_127_a_new_gain_applies_from_the_first_sample_*`: `assertion failed: ... <= 1e-6` |
| non-finite gain ignored; huge gains finite | `arq_123_a_non_finite_gain_keeps_the_last_valid_one_*`: `assertion failed` |
| SP19 row (id, range, default, animatable, label) | `p13_every_processor_schema_passes_validate_definition_and_matches_the_table`: `loudness_normalizer`, `left: 0 right: 1` |
| `process` is total | `arq_123_a_non_finite_sample_counts_as_zero`: `loudness_normalizer`, `left: [NaN, inf, -inf] right: [0.0, 0.0, 0.0]`; the same family in `total.rs` (8 tests) |
| last valid value survives `prepare` and `reset` | `arq_124_the_last_valid_value_survives_prepare` and `_reset`: `loudness_normalizer: the values must matter` |

Passed from the start (the stub does nothing, which is right for these): `arq_123_an_empty_block_after_a_valid_prepare_*`, `arq_123_an_empty_block_after_a_prepare_with_a_zero_*` (`empty.rs`), `arq_23_sixteen_tracks_*` (`alloc.rs`). Proved by the mutants "reads the parameter on a refused block", "no accepts guard", "default gain 1 dB" and "an allocation in `process`" below.

Hand mutants (applied, killed, reverted):

| Mutant | Killed by |
|---|---|
| Volume changed by Nivelar: the normalizer also reads `volume` | `arq_127_the_volume_of_the_clip_does_not_reach_the_normalizer`, `arq_124_the_last_valid_value_survives_*` |
| Gain limited to the 200 % of Volume (linear gain capped at 2.0) | `arq_127_a_gain_of_16_db_is_applied_with_the_volume_at_100_percent_untouched` |
| Power law (`db / 10`) | the same, `arq_127_a_new_gain_applies_*` |
| Gain not constant across the signal (a drift per sample) | `arq_127_the_gain_is_constant_over_the_whole_signal_across_blocks` and the other value tests |
| Input not cleaned; no `accepts` guard; the parameter read on a refused block | `total.rs`; `refused.rs`, `total.rs`; `empty.rs` |
| Default gain 1 dB; range +-200 dB; animatable | `arq_127_the_volume_of_the_clip_*`, `empty.rs`; `arq_123_a_non_finite_gain_*`, `p13_*`; `p13_*` |
| `reset` forgets the gain | `arq_124_the_last_valid_value_survives_reset` |
| An allocation in `process` | `arq_23_sixteen_tracks_*` (SIGABRT, the harness shows no `FAILED` line) |
| Linear gain saturated at `f64::MAX` instead of `+inf` | survived, equivalent: `clean_output` already saturates a product and turns `0 x inf` into 0. The saturation was removed. |

Decisions the specs leave open (unit 3):
- `LoudnessNormalizer` is an `AudioProcessor` with `gain` in dB, range the whole `f64` (SP19), default 0, not animatable, label `audio.loudness_normalizer.gain`. It applies the gain from the first sample of the block that carries it, with no ramp: ARQ-124 puts the ramp on `Gain` only, and Nivelar sets one gain for the whole signal. Latency and tail are 0.
- The engine feeds it `Leveling::gain_db` (`level` or `plan_leveling`); nothing in the crate writes a `ParamSnapshot`, so the Volume cannot change.

### AU3 closing

Commits: `7283248` (meter), `341ad24` (true peak and plan), `d0cbc8e` (`LoudnessNormalizer`). The AU3 sweep is recorded after AU4.

## AU4

### Resampler (ARQ-128, FV-04 b, ARQ-110, ARQ-111)

RED, against a compiling stub (`resampled_len -> Ok(0)`, `Resampler::new -> Ok`, `process` and `finish` doing nothing, `resample -> Ok(vec![])`, `latency -> 0`), `tests/resample.rs`, 11 of 18 failed:

| Criterion | Failing line |
|---|---|
| 44 100 samples 44.1 -> 48 kHz give 48 000 | `arq_128_44100_samples_*`: `left: 0 right: 48000` |
| 96 000 samples at 2.0x give 48 000 (FV-04 b) | `arq_128_96000_samples_at_2x_*`: `left: 0 right: 48000` |
| length is the f64 expression, half away from 0 | `arq_128_the_length_is_the_f64_expression_*`: `left: 0 right: 1` (first tie row) |
| speed 0, negative, 0.09, 10.1 are `Err` | `arq_128_a_speed_outside_0_1_to_10_*`: `left: Ok([]) right: Err(OutOfRange { field: "speed" })` |
| non-finite speed | `arq_110_a_non_finite_speed_*`: `left: Ok([]) right: Err(NonFinite { field: "speed" })` |
| non-finite sample, nothing written, state intact | `arq_110_a_non_finite_sample_*`: `left: Ok([]) right: Err(NonFinite { field: "samples" })` |
| SNR >= 80 dB, 1 kHz 44.1 -> 48 kHz | `arq_128_a_1_khz_sine_*`: panic at the SNR (empty output) |
| passband +-0.1 dB to 0.9 of the lower Nyquist | `arq_128_the_passband_is_flat_*`: panic at the first row |
| stopband >= 80 dB from 1.0 of the lower Nyquist | `arq_128_the_stopband_*`: panic at the first row |
| impulse at the declared latency +-1 | `arq_128_an_impulse_comes_out_*`: panic at the first row |
| constant ratio over 20 s | `arq_128_the_ratio_is_constant_over_a_long_run`: panic |

Passed from the start (the stub answer, empty, is right for them), each proved by a mutant below: `arq_128_an_empty_input_*`, `arq_128_the_latency_is_declared_*`, `arq_128_the_output_has_the_length_of_the_formula_*` (an empty stub equals itself only for empty; it failed on the first non-empty size in the real run of the stub? it passed: the stub gave 0 for n in the table, which `len` also gave through the stub), `arq_128_chunks_of_any_size_*`, `arq_128_a_resampler_used_twice_*`, `arq_111_resampling_twice_*`, `arq_110_a_finite_input_*`.

One test was added after the implementation, as the fix for a surviving mutant: `arq_128_the_images_of_an_upsampled_tone_are_down_at_least_80_db` (the mutant "cutoff not limited by the lower rate when upsampling" survived the downsampling stopband test).

Hand mutants (applied, killed, reverted):

| Mutant | Killed by |
|---|---|
| Length in `f32` | `arq_128_the_length_is_the_f64_expression_*` (rows 9 and 10) |
| Rounding `floor`, `ceil`, half to even | the same test (ties and the 2.5 row) |
| Evaluation order `n * (to / from) / speed` | the same test (rows 6 to 8) |
| Ratio inverted in the length; in the position of the filter | `arq_128_the_output_has_the_length_*`; `arq_128_a_1_khz_sine_*`, the impulse test |
| Speed upper bound exclusive; lower bound exclusive; max 10.1; min 0.09; only `> 0` | `arq_128_a_speed_outside_0_1_to_10_*` (and `arq_110_a_finite_input_*` for the exclusive ones) |
| Speed finiteness check removed | `arq_110_a_non_finite_speed_*` |
| Non-constant ratio: position computed in `f32`; ratio drifting by 1e-6; a `trim` that drops one input too many (output depends on the chunks) | `arq_128_the_ratio_is_constant_over_a_long_run`, `arq_128_a_1_khz_sine_*`; `arq_128_chunks_of_any_size_*` |
| State reused: `finish` without `reset`; `reset` keeps `next_out`, the buffer or the count | `arq_128_a_resampler_used_twice_starts_each_run_from_nothing` |
| Cutoff 1.1; cutoff 0.8; 20 lobes; Kaiser beta 4; table of 8 entries per lobe | stopband and passband tests; the SNR tests |
| Cutoff not limited by the lower rate when upsampling | `arq_128_the_images_of_an_upsampled_tone_*` (survived until that test was added) |
| Latency declared 5 | `arq_128_an_impulse_comes_out_at_the_declared_latency_*` |
| No sample check; check after the samples were buffered | `arq_110_a_non_finite_sample_*` |
| No `clean_output` | `arq_110_a_finite_input_gives_a_finite_output_*` |
| Tail not flushed; empty input gives 1 frame | `arq_128_44100_samples_*`, `arq_128_96000_samples_*`; `arq_128_an_empty_input_*` |

Decisions the specs leave open (AU4):
- API: `resampled_len` (the formula alone), `Resampler` (`new`, `process` per chunk, `finish`, `reset`, `latency`) and `resample` (one call). The speed is fixed at `new`; there is no way to change the ratio in a run, so a non-constant ratio cannot be asked for. The bytes do not depend on where the chunks end.
- Speed bounds are inclusive: 0.1 and 10 are valid, as ARQ-128 and FV-04 say "de 0,1 a 10". NaN and `+-inf` are `NonFinite` naming `speed`; a finite value outside is `OutOfRange` naming `speed`.
- With a speed the clip is read as at `from x speed`, so the "lower Nyquist" of ARQ-128 is `min(from x speed, to) / 2` in output Hz (`min(to / speed, from) / 2` in input Hz). Tests use that.
- The latency is declared 0: the filter is centred, the output at frame `j` is the signal at input position `j x from x speed / to`, so the impulse comes out at `round(k x ratio)`. The filter looks ahead by `60 / (0.95 x min(1, ratio))` input samples; `process` waits for them and `finish` pays them with zeros, so the total is always the length of the formula.
- Filter: Kaiser-windowed sinc, 60 lobes of half-length, beta 9.6, cutoff at 0.95 of the lower Nyquist (the middle of the 0.9 to 1.0 transition), kernel read from a 2 048-per-lobe table by linear interpolation. Measured: passband within 0.1 dB to 0.9, stopband under -80 dB from 1.0, SNR over 80 dB.
- `resample` reserves the output before it works (`try_reserve_exact`), so a length that cannot be allocated is `OutOfRange` naming `samples`, not an abort. A length over `2^63` is the same error. Output samples go through `clean_output` (finite, no subnormal).
- The signal is zero outside its ends, so the edges ring like a filtered step; the SNR tests leave out 4 000 frames at each end.
