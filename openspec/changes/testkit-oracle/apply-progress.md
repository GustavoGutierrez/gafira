Flow: standard | test-only hardening | decided-by coordinator (Flow line not passed verbatim to tdd-apply; coordinator to confirm)

# testkit-oracle: apply progress

## Step 0: baseline (before the first test commit)

`cargo mutants -p gafira-testkit --jobs 4` on the branch base (spec commit
db8d06f, src unchanged): 143 mutants, 82 caught, 23 unviable, 38 missed, 0
timeouts, 5m10s. The archive figures (84 / 36) did not reproduce: two extra
misses, `lib.rs:652:80` `%` to `/` and `%` to `+` in `nested(65 + pick % 3)`
(re-run alone on a clean checkout: same). Reported to the coordinator, who
updated the spec (4e14161) with baseline of record 38 missed and the
"Extension payload depth" scenario. This is the RED evidence: every mutant
below survived the pre-change tests.

## Done

| Commit | Tests (file) | Baseline mutants killed |
|---|---|---|
| `test(testkit): pin colour spaces, negative channels and px page-side bands of document()` | `generator.rs`: `every_colour_space_is_generated` (4, 5), `negative_channels_are_generated` (1 to 3), `px_pages_reach_both_exact_limits` (16), `px_pages_reach_the_bands_next_to_the_limits` (6 to 11), `px_pages_reach_the_random_band` (12, 14, 15) | 1 to 12, 14 to 16 |
| `test(testkit): pin the corruption guards and the corrupt pick table` | `corruption.rs`: `a_guard_rejects_the_error_of_every_other_class` + `page_size_kinds_share_an_error_and_duplicate_id_kinds_share_it_by_id` (17, 18), `next_id_offset_follows_pick_modulo_3` (19, 20), `page_size_value_and_side_follow_the_pick_table` (21 to 23), `non_finite_field_and_value_follow_the_pick_table` (24 to 33), `skew_values_follow_the_pick_table` (34, 35), `extension_payload_depth_follows_pick_modulo_3` (37, 38; the two `652:80` misses) | 17 to 35, 37, 38 |

Draws: colour 2 000, px bands 20 000 (bands computed once, shared through a
`OnceLock`, minimum 10 per band), corruption docs: the empty document plus 2
generated ones, picks 0..1000 (deterministic runner, independent of
`PROPTEST_CASES`).

RED evidence method: each listed mutant is in the Step 0 missed list (survived
the old tests) and is absent from the final T4 missed list (killed by the new
test named above). No per-mutant T3 run was made beyond the full T4.

## Gates

Per commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets
--locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny
check` (advisories, bans, licenses, sources ok), and the CI `policy` job steps
(format, forbid unsafe, V5, V12, V13, V14) run locally: all green. One
`CARGO_TARGET_DIR` with `CARGO_INCREMENTAL=0`.

## Final T4

`cargo mutants -p gafira-testkit --jobs 4 -o <scratchpad>` on the tip: 143
mutants, 118 caught, 23 unviable, 2 missed, 0 timeouts, 6m12s wall (baseline
5m10s here; archive run 10m17s). `missed.txt`:

```
crates/gafira-testkit/src/lib.rs:235:18: replace + with * in axis_length
crates/gafira-testkit/src/harness.rs:84:9: delete field failure_persistence from struct Config expression in try_prop_command
```

## Deviations and findings

- Pre-existing flake, not touched: `tests/harness.rs`
  `each_broken_law_is_named_with_a_counterexample` fails intermittently (about
  half of runs) with `PROPTEST_CASES=1`; it passes with the variable unset.
  The new oracles pass in both settings and draw the same counts. The spec
  scenario "Oracles ignore PROPTEST_CASES" requires the whole package to pass
  with `PROPTEST_CASES=1`, which that existing test prevents. Left for the
  coordinator.
- `crates/gafira-testkit/src`, `.cargo/mutants.toml` and CI are untouched; no
  `mutants::skip`.

## Size

`git diff --shortstat main...HEAD`: 3 files changed, 890 insertions (spec 444
of those; tests 446).
