# Verify report: anim-core D2a

D1: PASS in round 2 at `0a2f336` (0 CRITICAL, 1 WARNING W3 equivalent bezier mutants, 4 SUGGESTION); merged into `develop` at `261dd77`.

Flow: standard | mutation: archive | tier: T4 | verify: V2. No cargo-mutants report for this delivery (expected with `mutation: archive`; not a finding). Hand mutants only.
Acceptance source: Slice card as amended by `c5262bd` (`apply-progress.md`), `docs/specs/02-arquitectura-software.md` §3.4, ARQ-18 (construction part) and ARQ-19 (`f32` and `Vec2` part).
Scope: delivery commits `c2b26f9` (Interpolate, f32, Vec2) and `d9d2e50` (Keyframe, KeyTrack, invariant). Out of scope: D2b (`Animatable`, `sample`, serde, ARQ-16, ARQ-12, ARQ-18 deserialization) and D3 (`Color`, `Transform2D`).

## Round 1 (commit `d9d2e50`)

### Verdict: PASS (delivery D2a; D2b and D3 remain)

Counts: 0 CRITICAL, 2 WARNING (both class ambiguity), 4 SUGGESTION.

### Method

- Clean `git archive` export of `d9d2e50` in the session scratchpad, files touched after export, own `CARGO_TARGET_DIR` per export, `CARGO_INCREMENTAL=0`. Disk free before building: 68 GB.
- V2 probes: 18 checks (16 unit, 2 property with 4096 cases) derived from §3.4 and ARQ-18/ARQ-19 and written before reading the implementer's tests, run on a second export with `tests/track.rs` and `tests/interpolate.rs` removed. All 18 pass.
- Hand mutants: 5, on a third export, against the implementer's tests.
- Per-commit gates were not run (V3 only).

### Gates (export of `d9d2e50`)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (no failures; the known flaky gafira-testkit test did not fail) |
| `cargo deny check bans licenses sources` | 0 |
| CI: Domain crates forbid unsafe code | 0 |
| CI: Domain crates respect the layer table (V5) | 0 (gafira-anim depends on gafira-geom and gafira-time, both in its row) |
| CI: Domain clippy.toml copies are in sync (V12) | 0 |

### Slice card and commit checks (fast lane, 3c)

- `c5262bd docs(anim-core): slice card amend` touches only `openspec/changes/anim-core/apply-progress.md` and says why (D2 measured 1107 lines; owner OK to split). Allowed.
- Implementer commits `c2b26f9` and `d9d2e50` do not touch `apply-progress.md` or any spec. The Slice card block was not changed by the implementer.
- No AI attribution or Co-Authored-By lines in `261dd77..c5262bd`. Conventional commit subjects.
- Size: 454 lines (`git diff --shortstat 261dd77 d9d2e50`: 180 + 274), below ~700; matches the card.
- No `mul_add`, `#[allow]` or `unsafe` in gafira-anim; `#![forbid(unsafe_code)]` present.
- `git grep` traces: ARQ-18 in `tests/track.rs` (4) and `tests/interpolate.rs`; ARQ-19 in `tests/interpolate.rs` (7 tests).

### Criteria map

| # | Criterion | Proof | Status |
|---|---|---|---|
| 1 | ARQ-18: 0 keys rejected, rule named | `arq_18_construction_names_the_rule_and_index` ("empty", `AtLeastOneKey`, `None`); probe `p_empty` | covered |
| 2 | ARQ-18: two equal `at` rejected at the key index | same test ("equal at", `Some(2)`); hand mutant M1 killed | covered |
| 3 | ARQ-18: decreasing `at` rejected, not sorted | same test ("decreasing at", "decreasing at i64"); probe `p_not_sorted_silently` | covered |
| 4 | ARQ-18: `CubicBezier` `x1` or `x2` outside [0, 1] rejected | same test (x1/x2 below and above, at index 0, 1, 2); boundary values valid in `arq_18_valid_tracks_are_kept_untouched`; probe `p_bezier_bounds` (adjacent floats) | covered |
| 5 | ARQ-18: non-finite bezier value rejected | same test (y1 NaN, y2 inf, x1 NaN) | covered |
| 6 | ARQ-18: non-finite `value` component rejected (f32, Vec2) | same test (NaN, -inf); `arq_18_non_finite_vec2_component_is_rejected`; `arq_18_finiteness_covers_every_component`; M3 killed | covered |
| 7 | ARQ-18: error names the rule and the key index | `KeyTrackError { rule, index }` compared in every case; `arq_18_error_message_names_rule_and_index` for the Display text | covered |
| 8 | ARQ-18: no silent sort or repair; no public operation leaves an invalid track | `arq_18_valid_tracks_are_kept_untouched` (keys returned as given); structural: `keys` private, only `new` and `keys()`, no `Deserialize` yet, no `&mut` accessor | covered (D2b must keep this when it adds `Deserialize`) |
| 9 | ARQ-19: `t = 0` gives `a`, `t = 1` gives `b` (f32, Vec2) | `arq_19_f32_endpoints_are_exact` (bits, includes -0.0), `arq_19_vec2_endpoints_are_exact`; M5 killed | covered |
| 10 | ARQ-19: for `t` in [0, 1], within `eps * max(abs(a), abs(b))` of the ends | `arq_19_f32_stays_between_ends`, `arq_19_vec2_stays_between_ends` (domain ±1e30 / ±1e300, see W2) | covered |
| 11 | ARQ-19: formula `a + (b - a) * t`; Vec2 per component in f64 with `t` widened | `arq_19_f32_follows_the_formula_and_extrapolates`, `arq_19_vec2_follows_the_formula_per_component` (bit equality); M4 killed | covered |
| 12 | ARQ-19: extrapolation without clamping | same tests (`t` = 1.5, -0.5, 2.0, -1.0); M2 killed | covered |
| 13 | ARQ-12/Q-29 for f32 and Vec2: bit-identical, IEEE only, no FMA | bit-equality formula tests run on the 3-target CI matrix; no `mul_add` in the crate | covered for D2a (the cross-platform golden of ARQ-12 belongs to D2b) |

Total 13, covered 13. IDs left out on purpose: ARQ-16, ARQ-12 structural and ARQ-18 deserialization (D2b), ARQ-19 `Color` and `Transform2D` (D3), ARQ-17 (D1, done).

### Hand mutants (5, all killed)

| # | Mutant | Killed by |
|---|---|---|
| M1 | `key.at <= prev.at` -> `key.at < prev.at` (equal `at` allowed) | `arq_18_error_message_names_rule_and_index`, construction test |
| M2 | f32: clamp `t` to [0, 1] before the formula | `arq_19_f32_follows_the_formula_and_extrapolates` |
| M3 | Vec2 `is_finite` checks only `x` | `arq_18_finiteness_covers_every_component` |
| M4 | Vec2: result rounded through f32 | `arq_19_vec2_stays_between_ends`, formula test |
| M5 | f32: drop the `t == 1.0` exact-end shortcut | `arq_19_f32_endpoints_are_exact` |

Rule order inside one key (at, then out, then value) is not fixed by the spec, so a mutant swapping it would be equivalent to the contract; not counted.

### Implementer deviations, judged

1. `Interpolate::is_finite` (required method, not in the §3.4 snippet): needed. ARQ-18 requires "cada `value` es finito en todos sus componentes" for a generic `T`, and the snippet's bounds (`Clone + PartialEq + Send + Sync`) give no way to test finiteness. The implementation is right; the spec snippet is incomplete. See W1.
2. `KeyTrack.keys` private with `new` and `keys()`: required by the normative text ("Ninguna operación pública deja una pista inválida"); a `pub` field as in the snippet would break it. Snippet drift only. See S1.
3. `KeyTrackError { rule: KeyTrackRule, index: Option<usize> }`: meets "nombra la regla y el índice"; `None` only for 0 keys, where no index exists. No finding. See S2.
4. f32 overflow for finite opposite-sign ends near ±3e38: the prescribed formula gives `-inf` (probe: `3e38.interpolate(&-3e38, 0.5) == -inf`), which contradicts the ARQ-19 between-ends property for that domain. Spec gap. See W2.

### CRITICAL

None.

### WARNING

- W1. class: ambiguity. ref: `docs/specs/02-arquitectura-software.md` §3.4 snippet; `crates/gafira-anim/src/interpolate.rs`. The `Interpolate` trait in the snippet has only `interpolate`, but ARQ-18 needs a finiteness check over generic `T`. The implementer added a required `fn is_finite(&self) -> bool` to the public trait. Correct choice; the specifier should add it to the snippet so the public API and the spec agree (D3 `Color`, `Transform2D` and later `GradientStops` must implement it). Not blocking.
- W2. class: ambiguity. ref: ARQ-19 (`f32` and `Vec2` rows, between-ends property). With finite ends whose difference overflows (`abs(b - a) > f32::MAX`, e.g. 3e38 and -3e38), `a + (b - a) * t` is infinite for `t` in (0, 1), so "each component is between `a` and `b`" does not hold for every finite pair; the same applies to Vec2 near `f64::MAX`. The tests cap the domain at ±1e30 / ±1e300, which the spec never states. The specifier should either state the value domain for the property or pick a formula that cannot overflow (that changes the bit-level result, so it is a spec decision). Not blocking: real document values are far from that range.

### SUGGESTION

- S1. When updating the §3.4 snippet (W1), also show `KeyTrack` with a private `keys`, `KeyTrack::new` returning `Result<_, KeyTrackError>` and `keys()`.
- S2. D2b should reuse `KeyTrackError`/`KeyTrackRule` for the deserialization error so "same rule and same index" (ARQ-18) compares the same values.
- S3. The D2a RED evidence and deviations live in `0061556 docs(anim-core): D2 progress` on `feat/anim-core-d2`, not on this branch (the split left them after D2b). They agree with this report (RED against stubs for the formula, endpoint and construction tests; between-ends properties have no RED by construction, closed by the formula tests). Carry that note with D2b so `apply-progress.md` holds it after merge.
- S4. `arq_19_vec2_endpoints_are_exact` compares with `PartialEq`, which treats -0.0 and 0.0 as equal; the spec asks only for `PartialEq`, so it is enough, but a bit comparison like the f32 test would also pin the sign of zero.

next: tdd-apply (D2b); W1 and W2 (ambiguity) can go to specifier without blocking D2b.
