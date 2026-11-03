# 0010. Pixel goldens are PNG files compared on raw bytes

- Status: accepted
- Date: 2026-10-30
- Related: ARQ-02, ARQ-113, ARQ-132, 02 §10, ADR 0007, changes `testkit-golden`, `gafira-raster`, `gafira-segment`

## Context

ARQ-113 requires an analytic value first, then a lossless stored golden generated on `x86_64-unknown-linux-gnu`, regenerated only by an explicit action, with a tested comparator. 02 §10 names `gafira-testkit::golden`, which does not exist. Domain crates cannot touch the file system, in tests too (`clippy.toml`, `--all-targets`). `gafira-cli` has no regeneration command yet.

## Decision

- **Format.** PNG, 8 bits, RGBA for image buffers and grayscale for one-channel masks, written with the `png` crate (02 §9.2), which becomes a normal dependency of `gafira-testkit` only. No colour chunks. Channel bytes are stored verbatim, including premultiplied values, and read with no transformation. A viewer may show premultiplied data slightly off; the bytes are what count.
- **`f32` output.** Analytic checks first (ARQ-113). If a golden is needed, the buffer is quantized with `round(c × 255)` on `[0, 1]` before comparing, because the tolerance of 02 §10 is in 1/255 steps.
- **API.** `gafira_testkit::golden` takes width, height, channels and `&[u8]`. It has no dependency on `gafira-raster`.
- **Comparator.** Pure function with two modes. `Exact`: every byte equal. `CrossPlatform`: at most `floor(pixels / 1000)` pixels differ by more than 1/255 on any channel. The mode is selected by target: exact on `x86_64-unknown-linux-gnu`, tolerant elsewhere.
- **Location and loading.** `crates/<crate>/tests/golden/<name>.png`, loaded with `include_bytes!`, so domain crates need no `std::fs`.
- **Regeneration.** A failing golden writes the candidate through `gafira-testkit` under the integration test's `CARGO_TARGET_TMPDIR`. No test writes under `tests/golden/`. Accepting a candidate is a manual copy, made on `x86_64-unknown-linux-gnu`, in a commit that says why.
- **Size.** A stored golden is at most 256×256 px.

## Consequences

- Goldens can be opened and diffed with any viewer, which helps Q-34.
- `gafira-testkit` gains the `png` tree; nothing in a release binary does, since testkit is dev-only.
- The file bytes of a PNG are not compared, only decoded pixels, so an encoder upgrade does not invalidate goldens.
- `gafira-cli`'s "regeneración de golden" (02 §2.1) becomes a convenience on top of the same copy, once it exists.

## Alternatives considered

- **Raw `.bin` with a small header.** No dependency, but a reviewer cannot look at it.
- **QOI.** Lossless and simple, but less tool support, and its crate is not in 02 §9.2 for this use.
- **`insta` snapshots.** Right for text formats (02 §10), wrong for pixels: no per-pixel tolerance.
- **Hashes only.** Cannot express the cross-platform tolerance.
- **`#[ignore]` test that rewrites references.** ARQ-113 forbids any test rewriting its reference.

## Amendment (owner decision, 2026-11-01)

Findings of `testkit-golden` verify round 1 (W1, W3, W4, C1). The decisions above stand; these rules complete them.

- **`f32` input.** Non-finite input is an error, never a pixel. Finite values outside `[0, 1]` saturate.
- **Test entry.** `check` is the entry for tests. `compare` stays public as the pure comparator and writes nothing. A macro in the calling test crate supplies `CARGO_TARGET_TMPDIR`, so the caller does not choose the directory. Only that entry is for stored goldens.
- **Candidate names.** A name is one path component of `[a-z0-9_-]`, with no separators. A candidate directory that resolves into a `tests/golden` folder is rejected.
- **When a candidate is written.** On any failure with a usable actual buffer: pixel mismatch, size or channel mismatch, and an empty or undecodable reference. An actual buffer above the size limit fails with no candidate, because it could not be stored.
- **First golden.** Create an empty file so `include_bytes!` compiles, run the test, review the candidate, and copy it by hand on `x86_64-unknown-linux-gnu`. Only the accepted PNG is committed, in a commit that says why.
