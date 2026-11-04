## Slice card
Flow: full | level: L2 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: the media stream opens with the keyframe index, the first piece of `gafira-media`, and it is independent of the render stream
Product IDs: ARQ-138, ARQ-139, ARQ-140, ARQ-144, FV-11, CAL-14 (partial) (source: specs/02-arquitectura-software.md §4.17.8 and §6.5, specs/05-funcionalidad-video-y-distribucion.md §A6)
Scope: `ME1` of `specs/changes/layer-3/design.md`: in `gafira-media`, the keyframe index built by scanning the container packets, the seek by keyframe, the exact seek with forward decode, and the crate-level contract the rest of the stream inherits (purity through ports, cancellation and progress, determinism).      Non-goals: the frame cache, the proxies, the playback clock, the exporter, any change to `gafira-ports`, the render crate, and the timeline model (`O1`).
Acceptance:
  AC1 Given a medium with three keyframes at 0 s, 2 s and 4 s, When a seek to 3,5 s runs, Then `SeekMode::Keyframe` lands on the 2 s keyframe and `SeekMode::Exact` lands on the 3,5 s frame [integration]
  AC2 Given a medium whose first keyframe is at 5 s and a seek to 3 s, When the index is searched, Then the 5 s keyframe is returned [unit]
  AC3 Given an empty index, When it is searched, Then it does not panic and reports the absence of a keyframe [unit]
  AC4 Given a medium that is closed and opened again, When the index is searched, Then the index is rebuilt and the search results are the same [integration]
  AC5 Given a project that has been opened, When the user cache is inspected, Then it holds no new index file [command]
  AC6 Given the crate, When its sources are read, Then there is no `std::fs`, no `std::net`, no `std::thread::spawn`, no `std::time::Instant::now`, no `std::time::SystemTime::now` and no `std::env`; the time comes from `ports::Clock` and the file system from `ports::FileSystem` [command]
  AC7 Given a long operation, When it runs, Then it checks the cancel token at least once per frame, emits a `Progress` with a non-decreasing fraction in `[0, 1]` that only reaches `1.0` on success, and the fraction follows units done over units planned and not elapsed time [unit]
  AC8 Given the same input, When the crate runs the operation twice, Then the bytes are the same, and with a pool of 1, 2 and 4 threads the index is identical [property]
  AC9 Given the crate, When the CI policy steps run, Then `gafira-media` still has `#![forbid(unsafe_code)]`, its V5 row still passes, and the testkit doubles this delivery fills compile with the `ports` feature on [command]
Plan: the delivery adds the keyframe index module to `crates/gafira-media/src/` and fills the media doubles of the testkit. The testkit is the single writer of `crates/gafira-testkit/src/ports/**` for the media files: this delivery owns `container.rs`, `codec.rs`, `decoder.rs`, `encoder.rs` and `muxer.rs`; the render stream owns `render_backend.rs` and `page_renderer.rs`. `crates/gafira-testkit/src/lib.rs` is not touched, because `SC1` already declared the whole module list. The crate declares `gafira-testkit` with `features = ["ports"]` in its `dev-dependencies`, which adds two lines to `Cargo.lock`; that is why this delivery is serial in the media stream. The critical code is the keyframe lookup boundaries, the seek direction, the empty index and the time before the first keyframe (design "Critical code"), so those carry the hand mutants.
Owner decisions touched: none (checked D-01 to D-57: the keyframe index is a domain capability and ARQ-144 keeps it in memory, so no project format or persistence change).

# Apply progress: gafira-media

## ME1 (tdd-apply)

Commits:

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps (forbid, CLI/UI, V5, V12, V13, V14, and the ARQ-107 step), each by extracting its `run:` from `ci.yml`.

### RED evidence

### Negative controls

### Choices and deviations
