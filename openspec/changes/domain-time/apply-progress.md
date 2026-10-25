Flow: full | pack: new capability or crate | tier: T3 | verify: V3 | decided-by: coordinator | reason: new layer-0 crate (full floor: new crate, V5 row, ordered deliveries); T3 per changed source file, T4 at archive; verify is left to the independent verifier

# Apply progress: domain-time (`gafira-time`)

Append-only. Each delivery records its RED evidence, the gates, the measured size and any deviation. Specs: `openspec/changes/domain-time/specs/time-model/spec.md` and `specs/timecode/spec.md`. Design and delivery table: `openspec/changes/domain-time/design.md`.

## D0 - crate skeleton and shared wiring

Status: done.

RED evidence (D0 has no Rust behavior; the acceptance criteria are command-level):

- Before the crate existed, `cargo tree -p gafira-time -e normal,build` failed with `error: package ID specification 'gafira-time' did not match any packages`.
- Before the V5 row existed, `grep -n gafira-time .github/workflows/ci.yml` returned no match while `crates/gafira-time/clippy.toml` already existed, which is exactly the V5 rejection the CI step is written for.

GREEN evidence:

- `cargo check -p gafira-time` -> `Finished dev profile`; `Cargo.lock` gained only the `gafira-time` package (serde, serde_json and proptest were already in the lock).
- `cargo tree -p gafira-time -e normal,build --target all --prefix none -f '{p}'` -> only `gafira-time`, `serde`, `serde_core`, `serde_derive`, and their proc-macro build dependencies. No other `gafira-*` crate.
- `cp crates/gafira-geom/clippy.toml crates/gafira-time/clippy.toml` + `diff -q` -> identical (V12).
- V5 row added: `gafira-time  -`.
- Root `[workspace.dependencies]` gained `gafira-time = { path = "crates/gafira-time" }`.
- `docs/specs/README.md` status row records C2c `domain-time`.

Gates (all from the repository root, before commit):

- `cargo fmt --check` -> exit 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` -> exit 0.
- `cargo test --workspace --locked` -> exit 0 (gafira-time has no tests yet; the crate compiles).
- `cargo deny check` -> `advisories ok, bans ok, licenses ok, sources ok`, exit 0.
- Local replay of the CI `policy` job -> `policy status=0`: ARQ-07 tree clean, `forbid(unsafe_code)` present, V5 table clean with 6 crates and 5 clippy copies, V12 sync clean, V13 `EditToken` scan clean, `fmt` clean.

Measured size: `git diff --cached --shortstat -- crates` = 3 files, 70 insertions; whole D0 change = 7 files, 82 insertions, 1 deletion.

Deviation: none.

## D1 - Flicks, constants and checked arithmetic

Status: done.

RED evidence (first run of `cargo test -p gafira-time --test flicks` against compile-clean stubs that returned zero; no compile errors):

- `seconds_are_exact` -> `left: 0, right: 705600000`.
- `milliseconds_are_exact` -> `left: 0, right: 705600` (and the 1,234 ms case exposed a wrong expected value in the spec; see deviation).
- `seconds_and_milliseconds_overflow_instead_of_wrapping` -> expected `Err(Overflow)`, got `Ok(0)`.
- `addition_overflow_is_an_error`, `subtraction_overflow_is_an_error`, `multiplication_overflow_is_an_error`, `negation_overflow_is_an_error` -> expected `Err(Overflow)`, got `Ok(0)`.
- `whole_seconds_floor_towards_negative_infinity` -> `left: 0, right: 1` / `right: -1`.
- `f64_seconds_are_the_lossy_display_value` -> `left: 0.0, right: 1.0`.
- `serde_uses_a_plain_integer` -> stub value serialized as `0` instead of `705600000`.
- Proptest: `add_then_subtract_is_an_identity`, `seconds_round_trip`, `milliseconds_round_trip`, `seconds_floor_matches_euclidean_division` failed with minimal input `value = 1`.
- 4 tests passed on the stubs (constants, tick divisibility, negative tick preservation), which is expected: those rules are constants or trivial accessors, not arithmetic.
- Result of the RED run: `15 failed; 4 passed`.

GREEN evidence:

- `cargo test -p gafira-time --test flicks` -> `19 passed; 0 failed` (unit tests plus 5 proptest properties).
- `cargo fmt --check` -> exit 0; `cargo clippy --workspace --all-targets --locked -- -D warnings` -> exit 0; `cargo test --workspace --locked` -> exit 0 (all workspace suites); `cargo deny check` -> all sections ok; local CI policy replay -> `policy status=0`.

Measured size: 5 files, 435 insertions under `crates/` (including the proptest regression seed).

Deviation: the spec example for 1,234 ms said 870,854,400 flicks; the correct product is 870,710,400 (1,234 x 705,600). The RED unit test exposed it before the implementation commit. The spec was corrected in commit `3d5ace9` (`docs(spec): correct the 1,234 ms example to 870,710,400 flicks`), and the test asserts the corrected value. No other criterion changed.

## D2 - Frame rates, sample rates and alignment conversions

Status: done.

RED evidence (`cargo test -p gafira-time --test rates` against compile-clean stubs that returned 1/1 rates and zero conversions; no compile errors):

- `standard_frame_rates_are_accepted_with_exact_durations` -> `left: 0, right: 29429400` for 23.976 fps.
- `ntsc_frame_durations_are_exact` -> all three durations were 0.
- `non_integral_frame_durations_are_rejected` -> 11 fps and 2398/100 were accepted instead of rejected.
- `zero_rates_are_rejected` -> `left: Ok(FrameRate { num: 1, den: 1 }), right: Err(ZeroDenominator)`.
- `rates_below_one_frame_per_second_are_rejected` -> 1/2 was accepted.
- `equivalent_fractions_normalize` -> 48/2 did not equal 24/1.
- `nominal_frame_rate_rounds_to_nearest_with_ties_up` -> every nominal rate was 0.
- `fps_f64_is_the_lossy_display_value` -> `left: 0.0`.
- `frames_convert_to_flicks_exactly`, `frame_conversion_overflow_is_an_error`, `instants_floor_to_the_containing_frame`, `frame_boundaries_are_exact_multiples`, `cross_rate_conversion_floors_instead_of_rounding` -> conversion stubs returned 0/false.
- Sample-rate tests and both serde tests failed the same way; `sample_rate_serde_uses_hertz` and the invalid-input guards exposed the stub normalization.
- Proptest `frames_round_trip_when_representable` failed with minimal input `frames = -1`; `samples_round_trip_when_representable`, `to_frames_floor_matches_euclidean_division`, `frame_boundary_matches_the_remainder` and `the_tick_divides_every_accepted_frame_duration` failed on the zero durations.
- Result of the RED run: `24 failed; 4 passed`.

GREEN evidence:

- `cargo test -p gafira-time --test rates` -> `28 passed; 0 failed` (unit tests plus 7 proptest properties).
- `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check` -> all exit 0; local CI policy replay -> `policy status=0`.

Measured size: 4 files, 618 insertions under `crates/` (including the proptest regression seed).

Deviation: none. The two proptest regression seeds committed with this delivery record the cases the stubs rejected; they pass now and stay as regression guards.

## D3 - TimeRange

Status: done.

RED evidence (`cargo test -p gafira-time --test ranges` against compile-clean stubs that returned a zero range; no compile errors):

- `construction_exposes_start_duration_and_end` and `from_bounds_computes_the_duration` -> all three accessors returned 0.
- `an_empty_range_contains_nothing_and_never_intersects` -> `is_empty()` was false for every range, including zero duration.
- `inverted_bounds_are_rejected` -> `from_bounds(1 s, 0 s)` returned `Ok`.
- `a_negative_duration_is_rejected` -> `new(0, -1)` returned `Ok`.
- `an_overflowing_end_is_rejected` -> both extremes returned `Ok`.
- `a_negative_start_is_allowed`, `containment_is_half_open`, `intersection_covers_overlap_containment_and_disjoint_cases` -> zero start and false/absent intersections.
- `serde_uses_start_and_duration` -> serialized `{"start":0,"duration":0}`.
- `serde_rejects_invalid_ranges` -> accepted a negative duration and an overflowing end.
- Proptest `construction_invariants` and `from_bounds_matches_new` failed with minimal inputs (`start = 0, end = 1`).
- Result of the RED run: `13 failed; 3 passed` (the three that passed were precisely the cases where "always false / always absent" matches the expected answer: adjacency, symmetry of a missing intersection, and a zero-range serde round trip).

GREEN evidence:

- `cargo test -p gafira-time --test ranges` -> `16 passed; 0 failed` (unit tests plus 4 proptest properties).
- `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check` -> all exit 0; local CI policy replay -> `policy status=0`.

Measured size: 4 files, 346 insertions under `crates/`.

Deviation: none.

## D4 - Timecode

Status: done.

RED evidence (`cargo test -p gafira-time --test timecode` against compile-clean stubs that returned a zero label and an empty string; no compile errors):

- `non_drop_formatting_uses_hh_mm_ss_ff`, `non_drop_parsing_accepts_the_shape_and_sign`, `negative_timecode_formats_with_a_leading_minus`, `hours_are_a_running_count_and_never_wrap`, `non_drop_29_97_drift_is_explicit`, `frames_beyond_nine_hundred_are_three_digits` -> every formatted label was the empty string and every parsed label was 0.
- `malformed_and_out_of_range_fields_are_rejected`, `the_frames_separator_is_part_of_the_mode`, `skipped_drop_frame_labels_are_rejected` -> the stub accepted every text.
- `drop_frame_is_rejected_for_unsupported_rates` -> the stub returned a non-drop 24 fps format for every request.
- `drop_frame_reference_labels_at_29_97`, `drop_frame_reference_times_at_29_97`, `drop_frame_reference_at_59_94`, `tenth_minutes_do_not_skip`, `negative_drop_frame_timecodes_round_trip` -> empty labels and zero times.
- `from_flicks_floors_to_the_containing_frame` -> every label was 0.
- `to_flicks_overflow_is_an_error` and `parsing_overflow_is_an_error` -> returned `Ok` instead of `Err(Overflow)`.
- `exhaustive_drop_frame_round_trip_over_twenty_blocks` failed on the second assertion (real frame 0 recovered as label 0 but the produced label was not accepted by `Timecode::new`, then the loop mis-compared).
- Proptest `non_drop_parse_round_trip`, `non_drop_from_flicks_floors`, `drop_frame_from_flicks_round_trip` and `drop_frame_from_flicks_round_trip_5994` failed on the zero label.
- Result of the RED run: `23 failed; 1 passed`.

GREEN evidence:

- `cargo test -p gafira-time --test timecode` -> `24 passed; 0 failed`, including the exhaustive bidirectional drop-frame check over twenty ten-minute blocks at 29.97 and 59.94 (about 1.08 million labels) and the reference vectors from `spike.md` C4.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check` -> all exit 0; local CI policy replay -> `policy status=0`.

Measured size: 4 files, 696 insertions under `crates/` (under the ~700 delivery ceiling).

Deviation: the first D4 gate run exposed a wrong D2 property, not a D4 defect. `the_tick_divides_every_accepted_frame_duration` asserted that one second is a whole number of frames, which is false for accepted rates such as 245/3 fps (frame duration 8,640,000 flicks does not divide 705,600,000). The correct invariant is `frame_duration * num == FLICKS_PER_SECOND * den`. The test was renamed to `accepted_frame_rates_have_exact_frame_durations`, corrected and committed as `2a57705`; the new proptest counterexample (`num = 19845, den = 243`, normalized 245/3) is kept in the regression file. The D4 gate also caught `clippy::manual_is_multiple_of`, fixed with `!total_minutes.is_multiple_of(10)` before the commit. No production behavior and no spec criterion changed.




