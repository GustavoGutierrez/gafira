# Spike: layer-2 third-party claims (E1 to E5)

Facts only, for the claims listed in `design.md`. Probes live in the session scratchpad, folder `e-spike/`, outside the repo. Toolchain 1.93.1. The repo was not modified except this file. Timings are by hand on one machine and are not a benchmark.

| # | Claim | Verdict | Blocker left |
|---|---|---|---|
| E1 | parley fits the text requirements | Confirmed with wrapper code; three-platform determinism not measured | Spec gap on `.notdef` |
| E2 | `assert_no_alloc` is usable | Confirmed; its licence needs an exception | None |
| E3 | Own sinc meets ARQ-128 | Confirmed | Spec gap on ties |
| E4 | Test fonts are licensed and can be subset | Confirmed | None |
| E5 | `png` round-trips bytes | Confirmed | None |

## E1. Text stack

Setup: parley 0.12 with `default-features = false, features = ["std"]`. Plain `default-features = false` does not compile. `complex-scripts` was not enabled in the probe.

| Topic | Result |
|---|---|
| System-font crates in the tree | 0 on the three targets |
| `cargo deny` | Passes |
| Font loading | Fonts register from bytes; stack order must come from the caller |
| Fallback | Mixed Latin, Arabic and emoji works |
| Uncovered character | Gives glyph id 0 with no flag. ARQ-116 must detect id 0 |
| Spacing | parley adds spacing after the last cluster of a line. Measured N·t = 26.0 against FD-41(e) (N−1)·t = 24.0. The wrapper subtracts it |
| Cluster overflow | Not reported. The wrapper compares advance with box width |
| Columns | Not provided; our own code |
| Ligatures | Non-zero spacing disables `liga`. Inter has no `liga`, so the FD-41(e) test uses Manrope |
| `FitBox`, 500 chars | 1.43 ms median (budget 16 ms) |
| 5 000 chars in 4 columns | 1.24 ms median (budget 16 ms) |
| Determinism | Metrics are `f32`, with no transcendental calls in the shaping path and no platform cfg |

Timing setup: one i7-12700H, warm cache, hand timing. This is not the reference hardware of D-03.

**Not measured:** three-platform determinism. It needs the CI golden.

**Open:** when no font covers a cluster, parley picks the best partial-coverage font. FD-41 does not say which font supplies `.notdef`.

## E2. Allocation guard

| Topic | Result |
|---|---|
| Licence | `assert_no_alloc` 1.1.2 is BSD-1-Clause, not in `deny.toml` |
| `cargo deny` | cargo-deny 0.20.2 does not licence-check dev-only dependencies of private crates, so the pass is vacuous |
| `forbid(unsafe_code)` | `#[global_allocator]` compiles under it in all four placements tried |
| Failure mode | A real allocation aborts, so a "guard catches" test needs a subprocess |
| Release | Default `disable_release` removes `AllocDisabler` in release: gate with `#[cfg(debug_assertions)]` or use `warn_release` |
| Negative probes | Need `black_box` |

## E3. Resampler

Own Kaiser-windowed sinc in `f64`, cutoff 0.95× lower Nyquist, stop edge 1.0×.

| Taps | Pass error | Stop band | ARQ-128 |
|---|---|---|---|
| 108 at ratio ≥ 1 (scaled by 1/ratio, 1080 at speed 10) | 0.0003 dB | −83.1 dB | Pass |
| 96 | not relevant | −55.7 dB | Fail |

- 1 kHz sine, 44.1 to 48 kHz: SNR 106 dB.
- The length formula with `f64` speed differs from exact at ties in 183 of 8 386 147 grid cases.
- `rubato` 5.0.1 is MIT OR Apache-2.0, but returns 48 001 of 48 000 samples and only reaches its figures with the stop band measured from 1.1×.

## E4. Test fonts

Inter, Manrope, Noto Sans Arabic and Noto Emoji are OFL 1.1 with no Reserved Font Name.

| Subset (`pyftsubset`) | Size |
|---|---|
| Inter | 252 KB |
| Manrope | 58 KB |
| Noto Sans Arabic | 267 KB |
| Noto Emoji | 6.9 KB |

- Advances match the full fonts and Arabic joining survives.
- Manrope's upstream repo returned 404; only the `google/fonts` copy was checked.
- Hebrew was not tested.

## E5. `png`

`png` 0.18.1 passes `cargo deny`. RGBA8 and Gray8 round trips are byte-identical. Goldens compare decoded pixels, or pin the encoder, since cross-platform encoded bytes are not measured.
