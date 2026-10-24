# Proposal: Domain Color (C2a, first delivery of the domain phase)

## Intent

The document model (`domain-doc`) and every later engine need one pure, serializable color vocabulary (02 §2 layer 0, §4.10). Today no color type exists. This change adds `gafira-color` so that `domain-doc` can store colors and `gafira-raster`/`gafira-render-cpu` can later convert them, without I/O and without new third-party dependencies.

## Change Chain

| Order | Change | Content |
|---|---|---|
| C1 | `workspace-scaffold` (merged) | Workspace, CI, `gafira-geom`, `gafira-cli --version` |
| C2a | `domain-color` (this) | `gafira-color` + parameterized V5 |
| C2b | `domain-doc` | `gafira-doc` model, `gafira-cli validate` |
| C2c | doc invariants (name TBD) | Document invariants (open question a) |
| C3 | `edit-history` | `gafira-edit` |

This replaces C1's single `domain-color-doc`. C2a MUST NOT decide `gafira-doc` or `gafira-anim` APIs; it only keeps `Color` compatible with a future untagged `Animatable::Static`.

## Scope

### In Scope
- `Rgba` (encoded sRGB), `LinearRgba`, `DisplayP3Rgba`, `ColorSpaceId`; all `f32`.
- Analytic conversions sRGB <-> linear <-> Display-P3 (transfer curves + matrices).
- `Color`: `#[serde(tag = "space")]` enum, serialized as a bare value. `space` uses the CSS Color 4 names `srgb`, `srgb-linear`, `display-p3` (shared with `ColorSpaceId`). Unknown fields are ignored on deserialization (02 §3.5).
- `trait ColorTransform` (`src`, `dst`, `apply_f32`), a minimal `PixelLayout`, and one analytic implementation.
- `#![forbid(unsafe_code)]`, `clippy.toml` byte-identical to `gafira-geom`'s (V12).
- CI V5 rewritten as a per-crate allowed-dependency table (geom: none; color: none).

### Out of Scope
- **Deferred to the next change (size)**: `Lab` and `Cmyk` with nominal conversion (design D12).
- ICC/`moxcms`, gradients, `bake_lut3d`/`Lut3d`/`LutTransform`, palettes/swatches, `ColorRegistry`, `Interpolate`.
- ΔE2000 production accuracy; any change to `docs/` or `gafira-geom`.

## Capabilities

### New Capabilities
- `color-model`: color types, `ColorSpaceId`, `Color` wire format.
- `color-conversions`: analytic conversions, `ColorTransform`, `PixelLayout`, `PixelBufferError`.

### Modified Capabilities
- `workspace-tooling`: V5 is driven by a per-crate table of allowed dependencies and matches line prefixes with no trailing space. The base spec exists only in C1's change folder until C1 is archived.

## Approach

Strict TDD; reference values from published sRGB/P3 matrices checked with tolerances (`powf` is not bit-identical across platforms), never bit-exact goldens. Dependencies: `serde` only; `proptest`, `serde_json` in dev.

**PROPOSAL deviations from 02 (design must confirm):**
- `apply_f32` needs `PixelLayout`; a minimal one is defined in `gafira-color` (layer 0 cannot depend on raster).
- `apply_f32` returns `Result<(), PixelBufferError>` instead of `()`. It errors when the buffer length is not a multiple of the channel count, and leaves the buffer untouched (design D9).
- `bake_lut3d` is omitted entirely, with no placeholder `Lut3d`. Adding a defaulted method later is non-breaking.
- The 02 §4.10 acceptance test (ΔE2000 < 1.0, 1 000-color chart) is deferred; C2a verifies against reference matrices and round trips.

## Affected Areas

| Area | Impact |
|---|---|
| `crates/gafira-color/**` (+ `clippy.toml`) | New |
| `Cargo.toml` (`[workspace.dependencies]`) | Modified (+1 path) |
| `.github/workflows/ci.yml` (V5) | Modified |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Projected ~893 measured lines, over 800 even without Lab/CMYK | High | Two PRs: PR1 ~720 (types, conversions, `Color`, V5), PR2 ~175 (`ColorTransform`, `PixelLayout`), or one PR under `size:exception`; `git diff --shortstat` after PR1's last task |
| Shell V5 passes silently if `cargo tree` fails | Med | Manual negative tests (forbidden edge, misspelled or missing row) during apply |
| `workspace-tooling` main spec absent | Med | Archive C1 before spec, or delta against C1's spec |

## Rollback Plan

Revert the change's commit(s): delete `crates/gafira-color/`, drop its workspace entry, restore the previous V5 step. No persisted data exists yet.

## Dependencies

- C1 merged on `main`; no new third-party crates.

## Success Criteria

- [ ] `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check` pass.
- [ ] CI green on the three ARQ-03 targets.
- [ ] `cargo tree -p gafira-color` has no `gafira-*`, UI, engine or I/O crate; V5 fails on a forced forbidden edge.
- [ ] sRGB -> linear -> sRGB and sRGB -> P3 -> sRGB round trips within stated tolerance; `Color` JSON round trip with CSS `space` names.
- [ ] `apply_f32` on a buffer whose length is not a multiple of the channel count returns `Err` and leaves the buffer unchanged.

## Proposal question round

Not asked interactively (executor cannot prompt). For the holder:

1. **(a)** Approve splitting C2 into three deliveries: color, doc model, doc invariants?
2. **(b)** Store `Color` as a bare value now, for future `Animatable::Static` (untagged) compatibility?
3. Accept deferring the ΔE2000 < 1.0 acceptance test until a reference/ICC path exists?
4. Delivery: two PRs (~720 + ~175) or one PR under `size:exception` (~893)? The review workload guard decides after `sdd-tasks`.
