# Verify report: anim-core D2b

D1: PASS in round 2 at `0a2f336` (0 CRITICAL, 1 WARNING W3 equivalent bezier mutants, 4 SUGGESTION); merged into `develop` at `261dd77`.
D2a: PASS in round 1 at `d9d2e50` (0 CRITICAL, 2 WARNING W1 `is_finite` missing from the §3.4 snippet and W2 f32/Vec2 overflow domain, both ambiguity and routed to specifier, 4 SUGGESTION); merged into `develop` at `37769a0`.

Flow: standard | mutation: archive | tier: T4 | verify: V2. No cargo-mutants report for this delivery (expected with `mutation: archive`; not a finding). Hand mutants only.
Acceptance source: Slice card as amended by `c5262bd` (`apply-progress.md`); `docs/specs/02-arquitectura-software.md` §3.4 with ARQ-16 (sampling table), ARQ-12 (structural part) and ARQ-18 (deserialization part). Spike reference: claim 3 (untagged enum over an unvalidated wire shape, then `try_from`).
Scope: delivery commits `71681b3` (Animatable and sample), `d8ab4cf` (wire shape and validation after) and `0061556` (D2 progress notes). Tree judged: `f30293b` (develop with D1 and D2a merged in), clean export. Out of scope: D3 (`Color`, `Transform2D`), the ARQ-12 benchmark (ARQ-40).

## Round 1 (commit `f30293b`)

### Verdict: PASS (delivery D2b; D3 remains)

Counts: 0 CRITICAL, 1 WARNING (ambiguity), 4 SUGGESTION.

### Method

- Disk free before building: 68 GB. Three `git archive` exports of `f30293b` in the scratchpad, files touched after export, one `CARGO_TARGET_DIR` per export, `CARGO_INCREMENTAL=0`; build folders freed with `cargo clean --target-dir` at the end.
- V2 probes: 17 checks (12 unit, 5 property with 4096 cases each) derived from §3.4, ARQ-16, ARQ-12 and ARQ-18 before reading the implementer's tests, run on an export with `tests/sampling.rs`, `tests/serde.rs` and the sampling regressions file removed. All 17 pass. They cover: every table row on a 3-key f32 track (with one flick before and after each key), the earlier key's `out` (EaseIn/EaseOut bit-compared with `interpolate(apply(u))`), bounce extrapolation, Hold on the first and middle key, last key's `out` unused, single key, `u` at keys `i64::MIN`/`i64::MAX` (0.25/0.5/0.75 within 1e-6) and 4-flick spans at both i64 ends, a thread-local call counter over a 1000-key track (0 calls before/after/at keys/inside Hold, 1 call inside a curve segment, 0 for `Static`), totality, ends held, key values bit-exact, Linear between neighbors, Hold value held, `u` monotone and `u` accuracy (1e-6) as properties, the golden format (`1.5`, bare f32 and Vec2 read as `Static`, Keyed shape and the Easing names), round-trip keeping the variant (f32 and Vec2), the residual ambiguity (`{"x":1,"y":2,"keys":[]}` and with a non-empty `keys` both read as `Static`), invalid tracks read as `Animatable<f32>` and `Animatable<Vec2>` naming rule and index with the same message as reading `KeyTrack` directly and never the generic "did not match" text, and wrong-typed values failing.
- Hand mutants: 5, on a third export, against the implementer's tests.
- Per-commit gates were not run (V3 only).

### Gates (export of `f30293b`)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (`tests/sampling.rs` 17 passed, `tests/serde.rs` 7 passed; no failures) |
| `cargo deny check bans licenses sources` | 0 |
| CI: Domain crates forbid unsafe code | 0 |
| CI: Domain crates respect the layer table (V5) | 0 (gafira-anim depends on gafira-geom and gafira-time, both in its row) |
| CI: Domain clippy.toml copies are in sync (V12) | 0 |

### Slice card and commit checks (fast lane, 3c)

- `c5262bd docs(anim-core): slice card amend` touches only `apply-progress.md` and states why (size split, owner OK). Allowed.
- `71681b3` and `d8ab4cf` touch only `crates/gafira-anim/` (src, tests, a proptest regressions file). `0061556` touches only `apply-progress.md` with one appended hunk (`## D2 progress`, after the card); it does not change the Slice card block. No implementer commit changed the card.
- The merge `f30293b` changes no crate file relative to `d8ab4cf` (crate diff `37769a0..f30293b` equals the delivery diff).
- No AI attribution or Co-Authored-By lines in `d9d2e50..0061556` or `37769a0..f30293b`. Conventional commit subjects.
- Size: `git diff --shortstat d9d2e50 d8ab4cf`: 638 insertions, 1 deletion (src ~96, tests ~542, regressions 10), plus 16 doc lines in `0061556`; the card says 640. Below ~700.
- No `#[allow]`, `unsafe`, `mul_add`, `HashMap`, `unwrap` or `expect` in `gafira-anim/src`; `#![forbid(unsafe_code)]` present.
- `git grep` traces: ARQ-16 in `tests/sampling.rs` (30 lines), ARQ-12 in `tests/sampling.rs` (5), ARQ-18 in `tests/serde.rs` (13).
- D2a S3 closed: `0061556` holds the D2a RED evidence (ARQ-19 formula/endpoint tests and ARQ-18 construction tests failing against stubs; between-ends properties without RED by construction) and it agrees with the D2a report. It also records D2b RED: 12 of 17 sampling tests against a first-key stub, and `arq_18_invalid_tracks_fail_naming_rule_and_index` failing twice (plain derive accepted an empty track; validation inside a plain untagged enum gave the generic error, spike claim 3).

### Criteria map

| # | Criterion | Proof | Status |
|---|---|---|---|
| 1 | ARQ-16 `Static(v)` is `v` for any `t` | `arq_16_static_is_constant`; probe `p_static_any_t` | covered |
| 2 | ARQ-16 `t <= keys[0].at` gives `keys[0].value` (incl. one flick before) | `arq_16_before_the_first_key_holds_its_value`, property `arq_16_ends_are_held` | covered |
| 3 | ARQ-16 `t >= keys[n-1].at` gives the last value; last `out` unused | `arq_16_from_the_last_key_holds_its_value` (Hold, Linear, EaseIn as last `out`), `arq_16_ends_are_held` | covered |
| 4 | ARQ-16 `t == keys[i].at` gives `keys[i].value` without interpolating | `arq_16_at_a_key_returns_its_value`, property `arq_16_sample_at_each_key_returns_its_value` (bits), counting test; M1 killed | covered |
| 5 | ARQ-16 between keys: `u` formula and the earlier key's `out` | `arq_16_between_keys_uses_the_previous_keys_curve` (Linear 0.25/0.5/0.75 exact, EaseIn vs EaseOut), `arq_16_one_flick_from_each_key`; M2 killed | covered |
| 6 | ARQ-16 `Hold` keeps the value until the next key and jumps there | `arq_16_hold_keeps_the_previous_value_until_the_next_key`, property `arq_16_hold_holds_until_the_next_key`; M4 killed | covered |
| 7 | ARQ-16 total: no panic or overflow, incl. `i64::MIN`/`i64::MAX` for `t` and `at` | `arq_16_adjacent_and_extreme_keys_are_total` (`u = 0.5` at `t = -1`, `0` over MIN..MAX), property `arq_12_sample_is_deterministic_and_total` (MIN/MAX in the generators and probes); M3 (wrapping i64 subtraction) killed | covered |
| 8 | ARQ-16 `u` does not decrease as `t` grows | property `arq_16_progress_is_monotone_in_t` | covered |
| 9 | ARQ-16 single key returns its value for any `t` | `arq_16_single_key_is_constant` | covered |
| 10 | ARQ-16 property: Linear result between neighbors (ARQ-19 tolerance) | property `arq_16_linear_stays_between_neighbors` | covered |
| 11 | ARQ-12 structural: at most one `interpolate`; none at a key, outside the range or with Hold | `arq_12_sample_calls_interpolate_at_most_once` (shared `Arc<AtomicUsize>` counter over all copies); M1, M4 killed; O(log n) by `partition_point` (binary search), not behavior-testable | covered |
| 12 | ARQ-12 bit-identical on the same platform | property `arq_12_sample_is_deterministic_and_total` (bits, clone and same track) | covered |
| 13 | ARQ-18 format: `Static` bare, bare values read as `Static`, Keyed shape and Easing names | `arq_18_static_is_written_bare_and_read_back` (f32 and Vec2), `arq_18_keyed_wire_format` | covered |
| 14 | ARQ-18 round-trip keeps value and variant (f32, Vec2) | properties `arq_18_f32_round_trips`, `arq_18_vec2_round_trips` (`discriminant` compared) | covered |
| 15 | ARQ-18 invalid track read as `Animatable<T>` (and as `KeyTrack`) fails naming rule and index | `arq_18_invalid_tracks_fail_naming_rule_and_index` (0 keys, equal `at`, decreasing `at`, x1 above, x2 below; both readers); M5 (plain untagged derive) killed; probe adds Vec2 and equal messages | covered (see S3) |
| 16 | ARQ-18 invalid or wrong-shaped data never yields a track or is repaired | `arq_18_wrong_shape_fails`; structural: `KeyTrack` reads only through `try_from = "KeyTrackWire<T>"`, `Animatable` only through `RawAnimatable` + `KeyTrack::try_from` | covered |
| 17 | ARQ-18 residual ambiguity: `T` fields plus `keys` read as `Static` | `arq_18_hand_written_object_with_keys_reads_as_static` | covered |

Total 17, covered 17. IDs left out on purpose: ARQ-12 benchmark (ARQ-40), ARQ-12 cross-platform golden (needs the four §3.4 types, see W1), ARQ-19 `Color` and `Transform2D` (D3). ARQ-17 (D1) and ARQ-18 construction, ARQ-19 f32/Vec2 (D2a) are done.

### Hand mutants (5, all killed)

| # | Target | Mutant | Killed by |
|---|---|---|---|
| M1 | binary search | `partition_point(\|k\| k.at <= t)` -> `k.at < t` (at a key, picks the previous segment) | `arq_12_sample_calls_interpolate_at_most_once`, `arq_16_hold_keeps_the_previous_value_until_the_next_key`, `arq_16_sample_at_each_key_returns_its_value`, `arq_16_hold_holds_until_the_next_key` |
| M2 | segment selection | curve of the later key: `to.out.apply(u)` | `arq_16_between_keys_uses_the_previous_keys_curve` |
| M3 | `u` arithmetic | i64 `wrapping_sub` instead of i128 subtraction | `arq_16_adjacent_and_extreme_keys_are_total` |
| M4 | Hold branch | drop `\|\| from.out == Easing::Hold` (Hold evaluated as a step curve) | `arq_12_sample_calls_interpolate_at_most_once`, `arq_16_hold_holds_until_the_next_key` |
| M5 | wire validation | drop `try_from = "RawAnimatable<T>"` (plain untagged derive) | `arq_18_invalid_tracks_fail_naming_rule_and_index` |

A linear scan in place of `partition_point` would be equivalent to the behavioral contract (only the complexity changes, which belongs to the ARQ-40 benchmark); not counted.

### Implementer deviations, judged

The D2 progress deviations either belong to D2a (already judged: `is_finite`, private `keys`, `KeyTrackError`, f32 overflow = D2a W1/W2, not re-raised) or agree with the spec: non-finite values via JSON are covered only by construction because JSON cannot carry them (§3.4 states the same). D2b follows S2 of the D2a report: the deserialization error is the same `KeyTrackError` Display as construction.

### CRITICAL

None.

### WARNING

- W1. class: ambiguity. ref: Slice card (`apply-progress.md`, Deliveries) and ARQ-12 ("Prueba golden entre plataformas"). The cross-platform golden of ARQ-12 (a file with tracks of the four §3.4 types and `t` values, sampled on the 3 CI targets; f32, Vec2 and curves bit-exact, Color and Transform2D within the §3.4 tolerances) is not assigned to any delivery: D2b is "ARQ-12 structural" and D3 is "ARQ-19 rest". The D2a report said it belonged to D2b; that was wrong, because it needs `Color` and `Transform2D`. D2b's bit-exact unit assertions run on the 3-target matrix but are not a stored golden file. The coordinator should assign it (D3 is the natural place) with a `slice card amend`, or record it as left out with a reason. Not blocking D2b.

### SUGGESTION

- S1. `u` is an `f32` (it feeds `Easing::apply(f32)`), so in a segment longer than 2^24 flicks (~24 ms) the last flicks before the next key round to `u = 1.0` and give the next key's value a few flicks early (Hold is handled before and keeps its jump exact). This agrees with the closed range `[0, 1]` in the table; the specifier may state it when touching §3.4.
- S2. Size: the card's 640 matches the measured 639 changed code and test lines (638 + 1); with the 16 progress lines the delivery is 655. No action.
- S3. `arq_18_invalid_tracks_fail_naming_rule_and_index` checks that both messages contain the rule and the index, not that they are equal, and does not check that the 0-key case names no index. The probe confirmed both; asserting equality of the two messages (minus serde's position suffix) would pin "the same rule and the same index" exactly. A Vec2 invalid case would cover the Static attempt failing on an object.
- S4. `arq_18_wrong_shape_fails` could also cover a wrong-typed `value` inside a Keyed key, which the spec allows to fail with the generic message.

next: tdd-apply (D3); W1 (ambiguity) goes to the coordinator's Slice card amend without blocking D3.
