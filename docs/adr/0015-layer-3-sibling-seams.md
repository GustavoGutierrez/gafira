# 0015. `gafira-render` and `gafira-media` are siblings that meet only through `gafira-ports`

- Status: accepted
- Date: 2026-11-03 (proposed); accepted 2026-11-05, after layer 3 and layer-3b were built on it
- Related: 02 §2.1, §2.2, §2.3, §4.5, §4.6, §4.8, §6.1, §6.5; ARQ-01, ARQ-06; ADR 0007, ADR 0008, ADR 0013; change `layer-3`

## Context

02 §2.1 puts both crates in layer 3 ("Aplicación"): `gafira-render` builds the render graph, the caches, the planner, the frame budget and the backend selection; `gafira-media` runs the media pipeline: keyframe index, frame cache, proxies, A/V sync, playback clock and video export orchestration. 02 §2.2 draws them as siblings under `gafira-ports`, and ARQ-01 requires an acyclic graph where a crate depends only on its own layer or lower.

Two seams appear to need a crate edge, in both directions.

- 02 §6.1 gives the render graph a `Source::VideoFrame` node, so the renderer needs decoded video frames. 02 §2.1 gives the frame cache to `gafira-media`. That points from `gafira-render` to `gafira-media`.
- FV-12 gives the video export to `gafira-media`, and its pipeline is decode → compose → encode → mux. "Compose" needs rendered pages, and 02 §4.5 says `gafira-render` implements `PageRenderer`. That points from `gafira-media` to `gafira-render`.

Both edges cannot exist. ARQ-06 also keeps the layer-4 backends away from `gafira-render`, so the renderer cannot be reached through an adapter either.

## Decision

`gafira-render` and `gafira-media` do not depend on each other. Both depend on `gafira-ports`, and every seam between them is a trait in `gafira-ports`:

- `ports::PageRenderer`, implemented by `gafira-render` and consumed by `gafira-media`'s video exporter.
- A frame-source trait in `gafira-ports`, implemented by `gafira-media` and consumed by `gafira-render`, when the `VideoNode` kind lands in `gafira-doc`.

The composition root binds the two: it takes the `PageRenderer` from `gafira-render` and hands it to `gafira-media`, and takes the frame source from `gafira-media` and hands it to `gafira-render`.

## Consequences

The dependency graph stays acyclic and the two crates can be built, verified and merged in parallel worktrees. The cost is that a trait in `gafira-ports` has to be written before either side can be wired, so a change to a seam is a change to `gafira-ports` and reaches both crates at once. `gafira-ports` stays the one place where the layer-3 contract is written, which is where ADR 0013 already puts it.

The frame-source trait is not written yet, because the `VideoNode` kind does not exist in `gafira-doc`. When it lands, the trait goes into `gafira-ports` and not into `gafira-media`, so the edge direction does not change. A direct edge would have been the mistake this record exists to prevent.

## Alternatives considered

- `gafira-media` depends on `gafira-render`, so the exporter calls the renderer directly. Rejected: it contradicts the sibling drawing of 02 §2.2 and couples the media crate to the render crate's API, so every render change recompiles the media crate and the streams stop running in parallel.
- `gafira-render` depends on `gafira-media`, so the graph pulls decoded frames. Rejected for the same reason, and it would make the render crate depend on the codec and container stack it does not otherwise need.
- One crate for both. Rejected: 02 §2.1 names two crates with two responsibilities and two V5 rows, and the layer-3 purity of ADR 0016 is easier to keep and to check in two crates.
- The renderer reaches the media crate through a layer-4 adapter. Rejected: ARQ-06 keeps the backends free of `gafira-render`, and an adapter that knows both crates would be a second composition root.
