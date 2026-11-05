## Slice card
Flow: standard | level: L2 | pack: product-spec slice | mutation: off | tier: T0 | verify: V1 | decided-by: owner | reason: the media stream of layer-3b opens with the playback clock that keeps the audio and the video together; O8 carries O4 and O5 of layer 3 to this cut
Product IDs: FV-11, FV-10 (c) (source: specs/05-funcionalidad-video-y-distribucion.md §A6, specs/02-arquitectura-software.md §6.5)
Scope: `ME4` of `specs/history/2026-11-04-layer-3/design.md`: in `gafira-media`, the playback clock whose master is the audio, the A/V drift that follows from it, and the drop decision of §6.5 with its 40 ms tolerance.      Non-goals: the exporter (`ME5`), the frame cache (`ME2`, done), the keyframe index (`ME1`, done), the audio mixer itself (layer 2, `gafira-audio`), and any change to `gafira-ports`.
Acceptance:
  AC1 Given the playback, When the clock is read, Then the master is the audio and the video never blocks it (02 §6.5) [unit]
  AC2 Given a video frame that arrives 39 ms late, When the drop decision runs, Then no frame is dropped; given one that arrives 40 ms late or later, Then that frame is dropped and the next one is presented (02 §6.5) [unit]
  AC3 Given the preview, When the drift between the audio and the video is measured, Then it does not exceed 40 ms, the same tolerance that FV-12 (b) fixes for the exported file (02 §6.5) [unit]
  AC4 Given the speeds of FV-11, When the playback runs at 0,5×, 1× and 2×, Then the presentation time of every frame scales with the speed and the master clock stays the audio [unit]
  AC5 Given a frame stepped one by one in both directions, When the clock advances, Then the step follows the direction and stays inside the range (FV-11) [unit]
  AC6 Given a zero-length range, When it plays, Then it completes without panicking and without dropping any frame (02 §6.5) [unit]
  AC7 Given the clock, When it is read twice, Then the reading is monotonic and comes from the injected `ports::Clock`, not from a clock of the crate (ARQ-02, ARQ-138) [unit]
  AC8 Given a long playback, When it runs, Then the cancel token is checked per frame and the progress only reaches `1.0` on a normal end (ARQ-139) [unit]
  AC9 Given the crate, When the CI policy steps run, Then `gafira-media` still has `#![forbid(unsafe_code)]`, its V5 row still passes and the crate has no `std::fs` and no clock of its own [command]
Plan: the delivery adds the playback clock module to `crates/gafira-media/src/`, on top of the frame cache of `ME2`. It adds no dependency, touches no shared file, and reaches the time through `ports::Clock`. The critical code is the monotonicity of the clock, the drop decision, the drift computation and the zero-length range (design of layer 3, "Critical code"), so those carry the hand mutants.
Owner decisions touched: O8 of 2026-11-05 (the rigor of layer 3 carries over); no D-nn is contradicted.

# Apply progress: layer-3b-media

## ME4 (tdd-apply)

Commits:

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps (forbid, CLI/UI, V5, V12, V13, V14, ARQ-107, and the testkit `ports` step), each by extracting its `run:` from `ci.yml`.

### RED evidence

### Negative controls

### Choices and deviations
