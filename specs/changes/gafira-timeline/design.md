# Design: `gafira-timeline` — the timeline model of `gafira-doc`

**Status:** proposed · **Date:** 2026-11-05 · **Mode:** full · **Crate:** `gafira-doc` (layer 1)

This change builds the timeline model that `specs/02-arquitectura-software.md` §3.1 already gives to
`gafira-doc`: `Sequence`, `Track` and `Clip`. Today the model of `gafira-doc` is `Document → Page →
Node` (`crates/gafira-doc/src/document.rs:19-24`); there is no sequence, no track and no clip
anywhere in the crate, and the video engines of layer 3 work per asset and per time range because of
that gap (`specs/history/2026-11-04-layer-3/design.md`, P11 and O1).

O1 of the layer-3 design decided that the timeline model stays out of that cut and lands later, in
its own `gafira-doc` change at layer 1, with `gafira-media` gaining a resolver then. This is that
change.

Evidence read: `AGENTS.md` (Strict TDD and the gate scope of ADR 0017), `specs/00-decisiones.md`
(D-36, D-37), `specs/01-producto-y-calidad.md` (RF-36, RF-37, RF-38, RF-42), `specs/02-arquitectura-software.md`
(§2.1, §2.2, §3.1, §3.2, §3.5, §4.1, §4.3, §6.5, §7.1, ARQ-10, ARQ-13, ARQ-30 to ARQ-33, ARQ-136, ARQ-144, ARQ-145,
Q-54), `specs/05-funcionalidad-video-y-distribucion.md` (Parte A, FV-01 to FV-04, FV-09, V-1 to V-4),
ADR 0006, ADR 0007, ADR 0008, `specs/history/2026-11-04-layer-3/design.md` (O1, P11), the V5 row of
`.github/workflows/ci.yml`, and the code of `crates/gafira-doc`, `crates/gafira-edit`,
`crates/gafira-testkit`, `crates/gafira-time`, `crates/gafira-media` and `crates/gafira-ports`.

Precedent followed: `specs/history/2026-11-04-layer-3/design.md` (its delivery table, its "Why each
mark holds", its P-numbered decisions with alternatives, cost and reversibility, its "Critical code"
section and its "Open questions (owner)").

## Quick path

1. Run `TL1` alone. It brings the `gafira-doc → gafira-time` edge that ADR 0007 named as the trigger
   of that edge (`Page.duration`) and the V5 rows that edge forces. It is one commit across nine
   crates and it is the only delivery that touches `ci.yml`.
2. Run `TL2`: the model and its laws, in `crates/gafira-doc`, with the tests of the frame grid, the
   clip bounds, the track rules and the ids.
3. Run `TL3`: the persistence — the `Document` holds the sequences and the assets, the wire form is
   written and the goldens are updated.
4. The commands (`MoveClip`, `TrimClip`, `SplitClip`) and the resolver are **not** in this change;
   they are the two changes that follow it (see P6 and P7).

## Dependency state

State at `develop` cfcd03e. The layer-3 crates (`gafira-render`, `gafira-media`) are merged; the
`gafira-render-cpu` change is in flight in another worktree and owns `crates/gafira-render-cpu/**`,
the root `Cargo.toml` and `.github/workflows/ci.yml`. This change touches `crates/gafira-doc/**`,
`crates/gafira-edit/**` (the `Cargo.toml` edge only, in `TL1`), the V5 rows of `ci.yml` (in `TL1`) and
the goldens of `gafira-doc`. It does not touch `gafira-render`, `gafira-media`, `gafira-ports`,
`gafira-time`, `gafira-anim` or the testkit.

Two facts of the current code shape this change:

- `gafira-doc` may depend on `gafira-geom` and `gafira-color` only (V5 row, `crates/gafira-doc/Cargo.toml`).
  `Sequence.fps: FrameRate` and `Clip.place: TimeRange` are `gafira-time` types, so the model as
  §3.1 describes it needs the `doc → time` edge.
- `Document` holds `pages: Vec<Arc<Page>>` and a single id counter (`document.rs:21-24`,
  `document.rs:73-77`); the arena is per page (`page.rs:50-52`) and the validation rule P2 asks for
  the ids to be unique in the whole document (`validate.rs:43-53`).

## Scope of the cut

### In

- The types `Sequence`, `Track`, `Clip`, `TrackKind`, `ClipSource`, `Marker` and the id types
  `SequenceId`, `TrackId`, `ClipId`, `AssetRef`.
- The laws of the model: the frame grid, the bounds of a clip, the overlap rule per track kind, the
  cap of the audio tracks, the relation between `speed` and the duration, and the retiming rule when
  the project rate changes.
- The persistence: `Document` holds the sequences and the assets, the wire form writes every field
  (ADR 0006, rule 3) and reads with defaults (ADR 0006, rule 1).
- The `doc → time` edge and `Page.duration: Option<Flicks>`, the trigger ADR 0007 named.
- The minimal asset table the clips need: an id, the content hash, the name and the storage.
- The `DocMut` accessors for the timeline, under the `EditToken` discipline of ARQ-30.

### Out, and why

| Out | Reason |
|---|---|
| The commands `MoveClip`, `TrimClip`, `SplitClip` | They live in `gafira-edit` (FV-01, FV-03) and need the model's laws to exist first; P6 says why they follow |
| The resolver of "what is visible at this instant" | It needs the model to exist; P7 says where it lands and what it needs |
| `Project`, the `.gafira` container, `versions/` | Layer 4 (`gafira-format`, ARQ-50 to ARQ-59); the model lands in `Document`, which is what exists |
| The import feature, the asset metadata (duration, rate, legal class) | The table of this change carries the identity and the storage; the metadata comes with the import |
| The animation of the clip (`Animatable<f32>` volume, `Animatable<Transform2D>`) | The code of `ElementBase` is behind §3.2 today (`element.rs:12-32`); P3 keeps the bare values and names the deviation |
| The node kinds of video (`FrameNode`, `TextNode`, …) | Layer 1 work in `gafira-doc`, but not of this change; `NodeKind` is a closed enum and every new variant breaks `visit::dispatch` (`visit.rs:243-249`) |
| Drop-frame timecode, NTSC pulldown in the UI | A product question of Fase 2; the frame grid itself is exact (P4) |
| The A/V sync behaviour, the proxies, the export of a sequence | `gafira-media` owns them (P7); the model only says which clip covers which instant |

### What is reversible

The types, the laws and the persistence are additive: a field added to the document while schema 1 is
open is removed by removing the field, and the goldens move back with it. What is **not** reversible
inside this change is the `doc → time` edge (P2): once the domain names `Flicks`, un-naming it means
re-typing every instant of the model. And the shape of the persisted timeline is the owner's to
change later: the schema rule of D-36 closes when the first build that writes a `.gafira` reaches a
user, and from then on a new field of user data is a version bump (Q1).

## Delivery table

Sizes are authored lines with the planning figure ×1.4 applied. Every delivery is serial: `TL2`
needs the types of `gafira-time` that `TL1` brings, and `TL3` persists what `TL2` defines.

| # | Scope | Requirements | Depends on | Parallel | Size (×1.4) |
|---|---|---|---|---|---:|
| `TL1` | The `gafira-doc → gafira-time` edge, `Page.duration: Option<Flicks>`, the nine V5 rows that must gain `gafira-time`, the wire form and the goldens | ARQ-01, ARQ-02, ARQ-10, ARQ-13, ARQ-30, ARQ-136, V-2, V-5, V-12 | — | serial | 200 (280) |
| `TL2` | The model: `Sequence`, `Track`, `Clip`, `ClipSource`, `Marker`, the ids, the laws (the frame grid, the bounds, the overlap rule, the audio cap, the speed relation, the retiming), the `DocMut` accessors | ARQ-01, ARQ-10, ARQ-13, ARQ-30, ARQ-136, V-1, V-2, V-4, FV-01, FV-03, FV-04 | `TL1` | serial | 430 (600) |
| `TL3` | The persistence: `Document.sequences`, the minimal `AssetTable`, the wire form of both, the validation of the new ids, the goldens | ARQ-10, ARQ-13, ARQ-50, ARQ-53, ARQ-136, V-5, D-36 | `TL2` | serial | 330 (460) |

Total: 960 authored lines (×1.4: 1 350). The largest delivery is `TL2` at 430 authored lines.

### Why each mark holds

| Mark | Holds because |
|---|---|
| `TL1` is its own commit | ADR 0007 says the edge touches the rows of doc, edit, testkit and the five engines "as one commit"; the V5 step reads the whole `cargo tree`, so the rows and the `Cargo.toml` entries cannot land apart (the lesson of ADR 0007). It also brings `Page.duration`, the field ADR 0007 named as the trigger |
| `TL1` before `TL2` | The model names `Flicks` and `FrameRate`; the crate cannot name them before the edge exists (ADR 0007, Q-54's option (a)) |
| `TL2` before `TL3` | The laws are worth having before the data reaches the disk: a persisted clip that breaks a law is a document that fails to load, and the laws are cheaper to fix while the model is not yet written |
| `TL3` last | The goldens of the document move with the fields; the byte-pinned golden (`crates/gafira-doc/tests/golden/full.json`) is the evidence that the wire form writes every field (ADR 0006, rule 3) |
| All three serial | Each one touches `crates/gafira-doc/src/` and the same goldens; there is nothing to parallelise at this size |

## Decisions

### P1. The model lives in `gafira-doc`, and the `Document` holds the sequences next to the pages

Options: **(a)** `Document { sequences: Vec<Arc<Sequence>> }`, flat, like the pages; **(b)** a
`Timeline` wrapper type inside `Document`; **(c)** a new crate `gafira-timeline` at layer 1.

Decision: **(a)**. The ids of the sequences, tracks and clips come from the **same** counter as the
pages and the nodes (`document.rs:73-77`), so the rule "the ids are unique in the whole document"
(`validate.rs:43-53`) keeps holding with one check, and `next_id` stays above every id used. The
sequences are `Arc`, like the pages, so the history of `gafira-edit` (which snapshots the whole
document, `history.rs:219-242`) shares the sequences a command did not touch.

Reason: (b) buys a wrapper with no second piece of state, and §3.1's own prose has `Project.sequences`
as a flat list. (c) cannot hold the data: the persisted document must carry the sequences, the
`Document` is what is serialized (`json.rs:39-46`), and a crate that the document depends on while
depending on the document is a cycle. (c) is only possible if the `.gafira` container grows a second
file, which is layer 4 (`gafira-format`) and does not exist.

Cost: a field on the document, three new id types (`ids.rs:7-34` has the macro), one new arena-free
tree (the tracks and clips are nested inside the sequence, so they need no arena of their own).

Reversible: yes, while schema 1 is open (Q1).

### P2. The instants are `Flicks` and the rate is `FrameRate`: the change brings the `gafira-doc → gafira-time` edge

Options: **(a)** the edge, with `Flicks`, `FrameRate` and `TimeRange` named in the domain, as ADR 0007
anticipated ("it arrives with `Page.duration`") and as Q-54's recommendation (a) says; **(b)** the
domain keeps its own integer unit and converts at the boundary (Q-54's option (b)); **(c)** the model
stays out of `gafira-doc` and lives in a crate that already depends on `gafira-time`.

Decision: **(a)**. V-2 fixes the internal time to the flick (1/705 600 000 s), so the domain that
holds the timeline holds flickS; (b) would duplicate the unit and put a conversion on every field,
and (c) is P1's cycle.

Cost: the V5 rows of the nine crates whose tree gains the edge — `gafira-doc`, `gafira-edit`,
`gafira-testkit`, `gafira-pathops`, `gafira-layout`, `gafira-text`, `gafira-raster`, `gafira-segment`
and `gafira-render-cpu` — gain `gafira-time`, and their `Cargo.toml`s gain the edge, in one commit
(ADR 0007). `gafira-audio`, `gafira-ports`, `gafira-render` and `gafira-media` already list
`gafira-time`, so their rows do not move. The `clippy.toml` copies are unaffected (V12 only checks
that they are byte-identical).

Reversible: **no**, inside this change. Un-naming `Flicks` means re-typing every instant of the model.

### P3. `Page.duration: Option<Flicks>` arrives with the edge, and the clip keeps bare values where §3.1 names `Animatable`

§3.1 gives the clip `volume: Animatable<f32>` and §3.2 gives `ElementBase` an `Animatable`
transform; the code of `ElementBase` has bare values today (`element.rs:12-32`), and `gafira-anim` is
not a dependency of `gafira-doc`.

Decision: `Page.duration: Option<Flicks>` arrives in `TL1` — it is the field ADR 0007 named as the
trigger of the edge, and §3.1 says a video scene is a page with a duration. The clip's `volume` and
the base's transform stay **bare values** in this change, and the deviation is recorded: the
animation of the timeline arrives with the change that brings `ElementBase` up to §3.2.

Reason: a second edge (`doc → anim`) in the same cut doubles the V5 blast radius for a field the
spec itself has not reached in the code.

Cost: the clip's `volume: f32` and the base's bare values; when the animation arrives, those fields
become `Animatable` and the wire form changes inside schema 1.

Reversible: yes, while schema 1 is open.

### P4. The frame grid is the sequence's rate, and a cut is a frame boundary of that grid

Options: **(a)** the instants are `Flicks` and a clip's start and end must satisfy
`Flicks::is_frame_boundary(rate)`; **(b)** the instants are stored as frame indices and the ticks are
derived; **(c)** the instants are stored as ticks and the grid is only advisory.

Decision: **(a)**, with the rate a `FrameRate` of `gafira-time` (`rate.rs:143-256`). The 23.976 case
needs no rule of its own: `gafira-time` already carries the NTSC rates as exact rationals
(24 000/1 001 → 29 429 400 flickS per frame, exact, `rate.rs:52-56`), and `FrameRate::new` refuses
a rate whose frame is not an integral number of flickS (`rate.rs:208-226`). So there is no drift at
23.976 and no rounding rule to own.

The law: the start and the end of a clip are on the grid of the sequence, and the duration of a clip
is a whole number of frames. A clip that breaks it is a **validation error**, not something the model
repairs in silence: `Document::validate` crops nothing today (`validate.rs:18-25`) and the same
holds here.

Reason: (b) would make the tick the derived value and break the "the time is measured in flickS" of
V-2; (c) would let the model hold cuts that no frame can show. The grid at 23.976 fps is exact and
needs no rule of its own: `FrameRate::FPS_23_976` is 24 000/1 001 (`rate.rs:52-56`), so a frame is
29 429 400 flickS exactly, with no drift over an hour.

Cost: the rate is a field of the sequence and the validation grows a rule per clip.

Reversible: yes.

### P5. The ripple is a property of the track, and the commands carry the effective mode

Options: **(a)** `Track { ripple: bool }`, true by default on the video track; **(b)** the ripple is a
parameter of each command; **(c)** the ripple is implied by the track kind and cannot be changed.

Decision: **(a) for the state, and the commands carry the mode explicitly.** FV-01 says the video
track ripples *by default*, which is a default value and therefore a field; FV-03 says trimming can
ripple or not, which is a choice of the operation. So the track says what it does, and the command
receives the mode it applies: the command stays a pure function of its parameters (the laws of the
harness, `harness.rs:4-13`), and the interface fills the parameter from the track.

Reason: (b) alone would lose the default of the track (the user would have to ask for the ripple on
every operation); (c) would take away the choice that FV-03 names.

Cost: a field on the track; a parameter on the commands of the following change.

Reversible: yes.

### P6. The commands are a following change; the model can exist without them

The commands live in `gafira-edit` (FV-01, FV-03: `MoveClip`, `TrimClip`, `SplitClip` in
`gafira-edit::cmd`). They need the model's laws to exist, and they need two extensions in
`gafira-edit` that this change does not touch: a variant of `Property` for the time and the clip
(`set_property.rs:22-44`) and a `MergeKey` that names a track and a clip instead of a page and a
list of nodes (`command.rs:64-73`).

Decision: the commands are a **separate change**, after this one. The model without the commands is
a foundation, the way layer 3 delivered the machinery before the timeline (O1, P11). The alternative
— putting the commands here — grows the change by roughly 700 authored lines, mixes `gafira-edit`
into a cut that is about `gafira-doc`, and lands the commands before the laws they rely on are
verified against a persisted document.

Cost: the model is not user-visible until the commands change lands. That is the same trade layer 3
made and the owner accepted (O1).

Reversible: yes.

### P7. The resolver needs no new port and no change to the existing API of `gafira-media`

The question a resolver answers is "what is visible at this instant". It splits in two:

- **Which clips cover the instant** — a question about the model. `Sequence::active_clips(instant)`
  answers it, with the source time of each active clip (`place.start + (t − place.start) / speed +
  source_in`). This is pure model code and lands in `gafira-doc`.
- **Which frame of the asset is at that source time** — a question for `gafira-media`, and it is
  already answered: `Medium::seek(decoder, time, SeekMode::Exact)` lands on the frame
  (`medium.rs:101-135`) and `KeyframeIndex::keyframe_at_or_before` gives the keyframe to decode
  forward from (`index.rs:81-88`).

Decision: the resolver lands in `gafira-media` in a **following change**, taking the sequence from
`gafira-doc` (already a dependency of `gafira-media`) and answering with the media's existing
operations. No port changes, no change to the existing API of `gafira-media` — P11 of the layer-3
design says exactly this.

One gap is named here, because it is not solvable by the model: the frame cache is keyed by
`(content hash, proxy level, frame index)` (`cache.rs:38-42`), and **no crate maps a source time to
a frame index** — the keyframe index knows only the keyframes (`index.rs:45-60`) and the container's
declared rate can be wrong (the lesson of the item 90 of the debt list). The first cut of the
resolver therefore answers with **times** and lets the decoder land, and the time→frame-index
mapping is a named follow-up: either `gafira-media` builds a pts table at import, or the cache is
keyed by time.

Cost: the resolver change is small (a loop over the model's answer plus the media's existing calls)
but it waits for this one.

Reversible: yes.

### P8. The persisted timeline joins schema 1; the trigger is the first `.gafira` written for a user

ADR 0006's rule 1: schema 1 stays open until the first build that writes a `.gafira` reaches a user;
until then, additive fields join schema 1, `Document::SCHEMA_VERSION` stays 1 and no migration is
written. The facts of the repo: `gafira-format` does not exist (`crates/gafira-format` is not a
member), `gafira-cli` has only `--version`, and `v0.1.0-beta.1` attaches no binaries. No build has
ever written a `.gafira`, so no build has reached a user with one.

Decision: the timeline's fields join schema 1, with no migration, and every field is always written
(ADR 0006, rule 3) with `#[serde(default)]` on read (rule 1). The rule to re-check is ADR 0006's
own: the coordinator checks it when `gafira-format` is planned. Q1 asks the owner to fix the order.

Cost: the goldens of the document move in `TL3` (the byte-pinned golden
`crates/gafira-doc/tests/golden/full.json` is the evidence of rule 3), and the old bytes stay as an
input-only fixture that must load and re-save in the new canonical form (ADR 0006's consequence).

Reversible: yes, while schema 1 is open.

### P9. The asset table of this change carries the identity and the storage; the metadata comes with the import

`ClipSource::Media(AssetRef)` needs an id that the document can resolve, and §3.1's `AssetTable`
(id → content hash and metadata and storage) does not exist. The frame cache separates the two
(`MediumRef { asset, content }`, `cache.rs:80-93`) and ARQ-145 asks for two ids for the same file,
so the identity cannot be the content hash.

Decision: this change brings `AssetRef` and the minimum table the timeline needs: the id, the name,
the content hash and the storage (`Embedded | Linked`, the enum of §3.1; the bytes of an embedded
asset are the container's business, `gafira-format`). The metadata of the import — the duration, the
rate, the legal class — comes with the import feature.

Reason: a clip that points at an id the document cannot locate is a dangling reference, and the
validation would have nothing to check.

Cost: one more persisted structure in `TL3`, and the import feature later fills the same entries.

Reversible: yes, while schema 1 is open.

## Critical code

The code below is the critical code of each delivery; the coordinator writes the hand-mutant lists
from it, rule by rule, with the wrong behaviour and no code.

### `TL1` — the edge and `Page.duration`

- The conversion of the duration: a page duration is a `Flicks` value ≥ 0, and the wire form writes it
  always and reads it with a default (`None`).
- The V5 rows: the nine rows that must gain `gafira-time` (`gafira-doc`, `gafira-edit`, `gafira-testkit`,
  `gafira-pathops`, `gafira-layout`, `gafira-text`, `gafira-raster`, `gafira-segment`,
  `gafira-render-cpu`), and the `Cargo.toml` entries land in the same commit (ADR 0007). A row
  without the crate, or a crate without the row, fails the V5 step. The row and the `Cargo.toml` of
  `gafira-render-cpu` are owned by the change in flight today, so `TL1` sequences behind it.
- The golden: the byte-pinned document changes because the field is always written; the old bytes
  load and re-save in the canonical form.

### `TL2` — the model and its laws

- The frame grid: the boundary test of a tick against the sequence's rate, the conversion of a frame
  index to a tick and back, and the 23.976 case (24 000/1 001).
- The bounds of a clip: the start and the end on the grid, the duration ≥ 0, the source offset ≥ 0,
  the clip inside the sequence.
- The overlap rule: the clips of a video, audio or caption track do not overlap; the clips of an
  overlay track may.
- The cap of the audio tracks: sixteen, and the seventeenth is refused with a message.
- The relation between `speed` and the duration: the place of the clip is the source duration over
  the speed, rounded to the grid, and the source offset absorbs the remainder.
- The retiming rule: changing the rate of the sequence moves every cut to the nearest frame of the
  new grid and loses no clip (FV-01's acceptance (c)).
- The `DocMut` accessors: the timeline is mutable only with the `EditToken` (ARQ-30), and the new
  accessors are the only path to it.

### `TL3` — the persistence

- The wire form of the sequences and of the assets: the ids as plain numbers, the enums with their
  tags, every field written, the defaults on read.
- The validation: the ids of the sequences, tracks and clips are unique in the whole document and
  below `next_id` (the rule P2 of the existing validation, extended).
- The goldens: the canonical bytes of a document with a timeline, and the old bytes re-saved.

## What this change cannot prove

- **The video editor.** The five lanes, the magnet, the ripple as the user feels it, the keyboard
  navigation, the fps picker and the 16-audio-tracks message are layer 4 (`gafira-ui`); this change
  proves the model, not the interface.
- **The commands.** That moving a clip is one history entry, that trimming recovers the original
  frames and that the split is lossless (FV-03) belongs to the commands change; the laws of the
  harness apply there, not here.
- **The resolver.** That the preview and the export agree on a sequence (ARQ-42) needs the resolver
  and the two qualities; this change proves only that the model can say which clip covers an instant.
- **The persisted container.** The `.gafira` container, the migrations and the read-only guard are
  `gafira-format` (ARQ-50 to ARQ-59); this change proves the wire form of the document, not the
  container.
- **The A/V sync and the frame dropping.** They are `gafira-media`'s (`PlaybackClock::decide`,
  `playback.rs:214-222`); the model says which clip covers an instant, the clock says whether it is
  presented.
- **The time→frame-index mapping.** Named in P7: the cache is keyed by a frame index and no crate
  computes it from a time yet.
- **The real rate of a container.** The declared rate of a container can be wrong (the debt list's
  item 90); the model trusts the sequence's rate, which the project sets.

## Open questions (owner)

**Q1. The schema: may the timeline join schema 1, and does it land before the first `.gafira` reaches a user?**

Recommendation: **yes to schema 1, and yes to the order.** ADR 0006's rule 1 holds today: no build
has ever written a `.gafira` (`gafira-format` does not exist, `gafira-cli` has only `--version`,
`v0.1.0-beta.1` attaches no binaries), so the timeline's fields are additive and
`Document::SCHEMA_VERSION` stays 1 with no migration. The rule closes when the first build that
writes a `.gafira` reaches a user; the check belongs to the moment `gafira-format` is planned
(ADR 0006's own trigger). If the owner prefers to have the container first, the timeline's fields
become schema 2 with a `migrate_1_to_2` and its golden, and this change grows by the migration and
by the fixtures it needs.

**Q2. The `gafira-doc → gafira-time` edge: is touching the V5 rows of nine crates acceptable?**

Recommendation: **yes.** ADR 0007 already decided the edge arrives with `Page.duration` and that it
touches those rows as one commit; V-2 fixes the internal time to the flick, so the domain that holds
the timeline holds flickS; Q-54's recommendation (a) says the same. The alternative — the domain
keeping its own integer unit — duplicates a type that already exists and puts a conversion on every
field of the model. The cost is one commit that touches nine `Cargo.toml`s and the V5 table of
`ci.yml`, which is why `TL1` exists as a delivery of its own.

**Q3. The clip's animated volume and the base's animated transform: bare values now, or wait for `gafira-anim`?**

Recommendation: **bare values now** (P3). The code of `ElementBase` is behind §3.2 today
(`element.rs:12-32` has `opacity: f32`, not `Animatable<f32>`), and the animation of the timeline is
not in the acceptance of any row this change closes. Waiting for `gafira-anim` would make the change
wait for a second edge and a second layer-2 crate. The deviation is recorded in the notes of `TL2`.

**Q4. The commands and the resolver: one change or three?**

Recommendation: **three.** The model is the foundation; the commands need the model's laws and two
extensions in `gafira-edit` (`Property`, `MergeKey`); the resolver needs the model and adds nothing
to the ports. Folding them in would grow this change by roughly 1 400 authored lines (×1.4: about
2 000) and would put the commands before the laws they rely on are verified against a persisted
document. The owner may prefer to pay the size once; the order of the deliveries would then be
`TL1`, `TL2`, `TL3`, then the commands, then the resolver, with the commands' delivery sized
against the model that exists by then.

**Q5. The asset table: the identity and the storage now, or the whole table of §3.1?**

Recommendation: **the minimum now** (P9): the id, the name, the content hash and the storage. The
metadata of the import (duration, rate, legal class) comes with the import feature, which is where it
is produced. The owner may prefer the whole table in this change, which would move the import's
metadata into a change that does not import anything.

## Notes for the coordinator

- The gates of this change run with the **workspace** scope, because `gafira-doc` is not a leaf crate
  (ARQ-01) and because `TL1` touches `Cargo.toml` files and the V5 rows of `ci.yml`. ADR 0017's
  `-p <crate>` scope applies to the leaf crates only.
- `TL1` is the only delivery that touches `.github/workflows/ci.yml` and the root `Cargo.toml`. The
  `gafira-render-cpu` change owns both today; the two must be sequenced, not merged blind.
- The hand-mutant lists of `TL2` and `TL3` name the laws above; each law is one mutant, and the
  mutants are killed by the unit tests of the law, not by the golden.
- The verify depth of this change is V2 (code, with the laws). The change touches the document model
  and the wire form, so its golden is part of the evidence.
