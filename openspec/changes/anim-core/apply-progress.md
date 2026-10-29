## Slice card
Flow: standard | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: coordinator | reason: new crate gafira-anim already described in 02 (layer 1, deps time/geom/color, so the V5 row follows 02); spec gaps fixed by specifier first (724bdcb)
Product IDs: ARQ-12 (partial: structural proof; the 1 µs benchmark stays with ARQ-40), ARQ-16, ARQ-17, ARQ-18, ARQ-19 (source: docs/specs/02-arquitectura-software.md §3.4, §2.1, §2.2)
Scope: new crate `crates/gafira-anim`: `Interpolate`, `Easing` with its curves, `Keyframe`, `KeyTrack` with its invariant, `Animatable<T>` with serde and `sample`, and `Interpolate` for `f32`, `Vec2`, `Color` and `Transform2D`.
Non-goals: `GradientStops` (arrives with the gradient type); the ARQ-12 benchmark and `perf-budgets.toml` (ARQ-40); `ToMessage` for anim errors (ARQ-102, belongs above gafira-doc); wiring `Animatable` into gafira-doc nodes; animation presets; `SetKeyframe`.
Deliveries (serial):
  D1 crate scaffold (workspace entry, V5 row `gafira-anim gafira-time gafira-geom gafira-color`, clippy.toml copy, forbid(unsafe_code)), `Easing` and its curves (ARQ-17), ~300 lines
  D2 `Interpolate`, `Keyframe`, `KeyTrack` (ARQ-18), `Animatable` serde and `sample` (ARQ-16, ARQ-12 structural), `Interpolate` for `f32` and `Vec2` (ARQ-19 part), ~400 lines
  D3 `Interpolate` for `Color` and `Transform2D` (ARQ-19 rest); waits for owner answers to Q-27, Q-28, Q-29 and the explorer spike, ~350 lines
Plan: D1 -> verify -> merge to develop; D2 -> verify -> merge; D3 -> verify -> merge; T4 mutants on gafira-anim; archive (also fixes the README "Detalle" entry that lists ARQ-12 under domain-doc).
Owner decisions touched: none (checked D-03: the 1 µs figure is unchanged); open questions Q-27, Q-28, Q-29 gate D3 only.
