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
