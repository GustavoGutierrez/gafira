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
(D-36, D-37), `specs/01-producto-y-calidad.md` (RF-36, RF-37, RF-38, RF-42),
`specs/02-arquitectura-software.md` (§2.1, §2.2, §3.1, §3.2, §3.5, §4.1, §4.3, §6.5, §7.1, ARQ-10,
ARQ-13, ARQ-30 to ARQ-33, ARQ-136, ARQ-144, ARQ-145, Q-54),
`specs/05-funcionalidad-video-y-distribucion.md` (Parte A, FV-01 to FV-06, V-1 to V-8), ADR 0006,
ADR 0007, ADR 0008, `specs/history/2026-11-04-layer-3/design.md` (O1, P11), the V5 rows of
`.github/workflows/ci.yml`, and the code of `crates/gafira-doc`, `crates/gafira-edit`,
`crates/gafira-testkit`, `crates/gafira-time`, `crates/gafira-media` and `crates/gafira-ports`.

The limits are stated before the decisions. This change proves the model and its persisted wire form,
and nothing about the interface, the commands, the resolver, the container or the frame clock (see
"What this change cannot prove"). Two of its steps are also not its to take: the owner's answers to
Q1 and Q2, and the rewrite of ARQ-136's acceptance criterion that the edge forces (P10, Q6).

Precedent followed: `specs/history/2026-11-04-layer-3/design.md` (its delivery table, its "Why each
mark holds", its P-numbered decisions with alternatives, cost and reversibility, its "Critical code"
section and its "Open questions (owner)").

## Quick path

1. Run `TL1` alone. It brings the `gafira-doc → gafira-time` edge — ADR 0007 named `Page.duration`
   as its trigger, and that field arrives here — and the V5 rows the edge forces. It is one commit with one
   manifest edit and nine allow-list rows, and the only delivery that touches `ci.yml`.
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
its manifest, the V5 rows of `ci.yml` (in `TL1`) and the goldens of `gafira-doc`. It does not touch
`gafira-render`, `gafira-media`, `gafira-ports`, `gafira-time`, `gafira-anim`, `gafira-edit` or
`gafira-testkit`; of the last two, only their `ci.yml` rows move, because the V5 step reads the
transitive tree and their `Cargo.toml` needs no new entry (P2).

Two facts of the current code shape this change:

- `gafira-doc` may depend on `gafira-geom` and `gafira-color` only (V5 row of `ci.yml`, line 97;
  `crates/gafira-doc/Cargo.toml` names exactly those two).
  `Sequence.fps: FrameRate` and `Clip.place: TimeRange` are `gafira-time` types, so the model as
  §3.1 describes it needs the `doc → time` edge.
- `Document` holds `pages: Vec<Arc<Page>>` and a single id counter (`document.rs:21-24`,
  `document.rs:73-77`); the arena is per page (`page.rs:50-52`) and the validation rule P2 asks for
  the ids to be unique in the whole document (`validate.rs:43-53`).

## Scope of the cut

### In

- The types `Sequence`, `Track`, `Clip`, `TrackKind`, `ClipSource`, `Marker` and the id types
  `SequenceId`, `TrackId`, `ClipId`, `AssetRef`.
- The laws of the model: the frame grid, the bounds of a clip, the order and the non-overlap rule of
  every lane (P11), the cap of the audio tracks, the relation between `speed` and the duration, and
  the retiming function when the project rate changes (FV-01 (c)).
- The persistence: `Document` holds the sequences and the assets, the wire form writes every field
  (ADR 0006, rule 3) and reads with defaults (ADR 0006, rule 1).
- The `doc → time` edge and `Page.duration: Option<Flicks>`, the trigger ADR 0007 named, with the
  ARQ-136 guard rewritten to what survives the edge (P10).
- The minimal asset table the clips need, keyed by `AssetRef(ContentHash)`, with a name and the
  storage (P9).
- The `DocMut` accessors for the timeline, under the `EditToken` discipline of ARQ-30.
- The pure retiming function `Sequence::retimed`; writing its result is the commands change's job.

### Out, and why

| Out | Reason |
|---|---|
| The commands `MoveClip`, `TrimClip`, `SplitClip` | They live in `gafira-edit` (FV-01, FV-03) and need the model's laws to exist first; P6 says why they follow |
| The resolver of "what is visible at this instant" | It needs the model to exist; P7 says where it lands and what it needs |
| `Project`, the `.gafira` container, `versions/` | Layer 4 (`gafira-format`, ARQ-50 to ARQ-59); the model lands in `Document`, which is what exists |
| The import feature, the asset metadata (duration, rate, legal class) | The table of this change carries the hash key, the name and the storage; the metadata comes with the import (P9) |
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
| `TL2` | The model: `Sequence`, `Track`, `Clip`, `ClipSource`, `Marker`, `AssetRef`, the ids, the laws (the frame grid, the bounds, the order and non-overlap of P11, the audio cap, the speed relation, the retiming function), the resolver query, the `DocMut` accessors | ARQ-01, ARQ-10, ARQ-13, ARQ-30, ARQ-136, V-1, V-2, FV-01, FV-03, FV-04 | `TL1` | serial | 480 (670) |
| `TL3` | The persistence: `Document.sequences` and the asset table keyed by `AssetRef`, the wire form of both, the validation of the new ids and of the clip sources, the seven goldens plus the new timeline golden | ARQ-10, ARQ-13, ARQ-30, ARQ-53 (the fixture of the previous bytes only), V-5, D-36, FV-02 | `TL2` | serial | 330 (460) |

Total: 1 010 authored lines (×1.4: 1 415). The largest delivery is `TL2` at 480 authored lines
(×1.4: 670), still under the ~700 signal of `AGENTS.md`.

### Why each mark holds

| Mark | Holds because |
|---|---|
| `TL1` is its own commit | ADR 0007 says the edge touches the rows of doc, edit, testkit and the five engines "as one commit"; the V5 step reads the whole `cargo tree`, so the `gafira-doc` dependency and the nine allow-list rows cannot land apart (the lesson of ADR 0007). It also brings `Page.duration`, the field ADR 0007 named as the trigger |
| `TL1` before `TL2` | The model names `Flicks` and `FrameRate`; the crate cannot name them before the edge exists (ADR 0007, Q-54's option (a)) |
| `TL2` before `TL3` | The laws are worth having before the data reaches the disk: a persisted clip that breaks a law is a document that fails to load, and the laws are cheaper to fix while the model is not yet written |
| `TL3` last | The goldens of the document move with the fields; the byte-pinned golden (`crates/gafira-doc/tests/golden/full.json`) is the evidence that the wire form writes every field (ADR 0006, rule 3) |
| All three serial | Each one touches `crates/gafira-doc/src/` and the same goldens; there is nothing to parallelise at this size |

## Decisions

### P1. The model lives in `gafira-doc`, and the `Document` holds the sequences next to the pages

Options: **(a)** `Document { sequences: Vec<Arc<Sequence>> }`, flat, like the pages; **(b)** a
`Timeline` wrapper type inside `Document`; **(c)** a crate `gafira-timeline` that `Document` depends
on; **(d)** a sibling crate `gafira-timeline` above `gafira-doc`, with its own root, referring to
pages by `PageId` and persisted beside the document.

Decision: **(a)**. The ids of the sequences, tracks and clips come from the **same** counter as the
pages and the nodes (`document.rs:73-77`), so the rule "the ids are unique in the whole document"
(`validate.rs:43-53`) keeps holding with one check, and `next_id` stays above every id used. The
sequences are `Arc`, like the pages, so the history of `gafira-edit` (which snapshots the whole
document, `history.rs:12-18`) shares the sequences a command did not touch.

Reason: (b) buys a wrapper with no second piece of state, and §3.1's own prose has `Project.sequences`
as a flat list. (c) is a cycle: the persisted document must carry the sequences, the `Document` is
what is serialized (`json.rs:39-46`), and the crate would depend on the document it is a field of.
(d) is the only real alternative, and it is a good one on one axis: nothing in `gafira-doc` would name
a time type, so ARQ-136 would survive without a spec change (P10). It loses on three: §2.1 assigns
`Sequence`, `Track` and `Clip` to `gafira-doc`, so it needs a spec change anyway; `History` snapshots
one `Arc<Document>` (`history.rs:12-18`), so a second root means a second snapshot and an undo order
defined across two documents, which is ARQ-30's and ARQ-33's ground; and the sequences would still be
persisted data needing a container story that `gafira-format` has not built. It is the option to take
if the owner answers Q6 with "keep the crate-wide guard".

Cost: a field on the document, three new id types (`ids.rs:6-34` has the macro), one new arena-free
tree (the tracks and clips are nested inside the sequence, so they need no arena of their own).
Nested `Arc<Track>` and `Arc<Clip>` keep an edit to one clip from copying the whole sequence, and the
wire form does not change: `serde`'s `rc` feature is on (`crates/gafira-doc/Cargo.toml`), so a vector
of `Arc<T>` writes exactly what a vector of `T` writes. The history of `gafira-edit` shares the whole
snapshot `Arc`, so the copy-on-write path is what a drag pays for, not the snapshot.

**The shape of a clip.** §3.1 gives `Clip` a `base: ElementBase`, and `ElementBase` requires a
`Placement` (`element.rs:12-32`): a 2D transform on a page, which a clip does not have, and which the
placement pass of `validate.rs` would then be asked to accept as meaningless. So the clip carries a
minimal `ClipBase { id, name, visible, locked }` — the four fields of `ElementBase` that make sense
for a strip — and the timeline keeps its own fields (`source`, `place`, `source_in`, `speed`) beside
it. When the effects and the transitions arrive, the base grows the rest of §3.1's sketch; the wire
form still changes inside schema 1 while Q1 holds. This is a deviation from the sketch and it belongs
in the delta specs.

Reversible: yes, while schema 1 is open (Q1).

### P2. The instants are `Flicks` and the rate is `FrameRate`: the change brings the `gafira-doc → gafira-time` edge

Options: **(a)** the edge, with `Flicks`, `FrameRate` and `TimeRange` named in the domain, as ADR 0007
anticipated ("it arrives with `Page.duration`") and as Q-54's recommendation (a) says; **(b)** the
domain keeps its own integer unit and converts at the boundary (Q-54's option (b)); **(c)** the model
stays out of `gafira-doc` and lives in a crate that already depends on `gafira-time`.

Decision: **(a)**. V-2 fixes the internal time to the flick (1/705 600 000 s), so the domain that
holds the timeline holds flicks; (b) would duplicate the unit and put a conversion on every field,
and (c) is P1's cycle.

Cost: `crates/gafira-doc/Cargo.toml` gains `gafira-time` with `gafira-time.workspace = true` (the root
`Cargo.toml` already declares it, line 17 of `[workspace.dependencies]`, so no workspace manifest
moves), and the V5 rows of the nine crates whose `cargo tree` gains the edge list `gafira-time` among
their allowed deps: `gafira-doc`, `gafira-edit`, `gafira-testkit`, `gafira-pathops`, `gafira-layout`,
`gafira-text`, `gafira-raster`, `gafira-segment` and `gafira-render-cpu`. Only the first of the nine
changes a manifest. The V5 step compares `cargo tree -p <crate> -e normal,build` against the allowed
list, so the other eight need the allow-list entry and **no** dependency of their own; adding one would
be an unused dependency and a false edge in the layer diagram. `gafira-audio`, `gafira-ports`,
`gafira-render` and `gafira-media` already list `gafira-time`, so their rows do not move. The
`clippy.toml` copies are unaffected (V12 only checks that they are byte-identical).

Reversible: **no**, inside this change. Un-naming `Flicks` means re-typing every instant of the model.

### P3. `Page.duration: Option<Flicks>` arrives with the edge, and the clip keeps bare values where §3.1 names `Animatable`

§3.1 gives the clip `volume: Animatable<f32>` and §3.2 gives `ElementBase` an `Animatable`
transform; `ElementBase` carries bare values today (`element.rs:12-32`), and `gafira-anim` is not a
dependency of `gafira-doc`.

Decision: `Page.duration: Option<Flicks>` arrives in `TL1` — it is the field ADR 0007 named as the
trigger of the edge, and §3.1 says a video scene is a page with a duration. The clip's `volume` stays a
**bare `f32`** in this change, and the base is the `ClipBase` of P1 — it has no transform, so there is
no `Animatable` transform to carry either. The deviation is recorded: the animation of the timeline
arrives with the change that brings `ElementBase` up to §3.2 and a second edge.

Reason: a second edge (`doc → anim`) in the same cut doubles the V5 blast radius for a field the
spec itself has not reached in the code.

Cost: the clip's `volume: f32`; when the animation arrives, that field becomes `Animatable` and the
wire form changes inside schema 1.

Reversible: yes, while schema 1 is open.

### P4. The frame grid is the sequence's rate, and a cut is a frame boundary of that grid

Options: **(a)** the instants are `Flicks` and a clip's start and end must satisfy
`Flicks::is_frame_boundary(rate)`; **(b)** the instants are stored as frame indices and the ticks are
derived; **(c)** the instants are stored as ticks and the grid is only advisory.

Decision: **(a)**, with the rate a `FrameRate` of `gafira-time` (`rate.rs:21-137`), the boundary test
already written there (`Flicks::is_frame_boundary`, `rate.rs:219-222`) and the exact frame
conversions beside it (`Flicks::from_frames`, `rate.rs:206-212`; `Flicks::to_frames_floor`,
`rate.rs:214-217`). The 23.976 case needs no rule of its own: `gafira-time` already carries the NTSC
rates as exact rationals (24 000/1 001 → 29 429 400 flicks per frame, exact, `rate.rs:52-56`), and
`FrameRate::new` refuses a rate whose frame is not an integral number of flicks (`rate.rs:88-106`). So
there is no drift at 23.976 and no rounding rule to own. A fractional cut is not repaired either: a
tick that is not a boundary is a validation error.

The law: the start and the end of a clip are on the grid of the sequence, and the duration of a clip
is at least one frame and therefore a whole number of frames. A clip that breaks it is a **validation
error**, not something the model repairs in silence: `Document::validate` crops nothing today
(`validate.rs:18-23`) and the same holds here. There is no "the clip inside the sequence" clause to
add, because a sequence stores no length: `Sequence::duration()` is the end of its last clip, so the
bounds a clip answers to are its track's order, the non-overlap rule of P11 and the grid.

Reason: (b) would make the tick the derived value and break the "the time is measured in flicks" of
V-2; (c) would let the model hold cuts that no frame can show. A clip of zero frames is refused for
the reason `TimeRange` is half-open: it would never be active, so it could neither be shown nor
selected, and no command could bring it back.

Cost: the rate is a field of the sequence and the validation grows a rule per clip.

Reversible: yes.

### P5. The ripple is a property of the track, and the commands carry the effective mode

Options: **(a)** `Track { ripple: bool }`, true by default on the video track; **(b)** the ripple is a
parameter of each command; **(c)** the ripple is implied by the track kind and cannot be changed.

Decision: **(a) for the state, and the commands carry the mode explicitly.** FV-01 says the video
track ripples *by default*, which is a default value and therefore a field; FV-03 says trimming can
ripple or not, that reordering ripples, and that deleting leaves a gap unless "Eliminar y cerrar
hueco" closes it — all of which are choices of the operation, not of the model. So the track says
what it does, and the command receives the mode it applies: the command stays a pure function of its
parameters (the laws of the harness, `harness.rs:4-13`), and the interface fills the parameter from
the track. The default is set by `Track::new(kind)` (true for `Video`, false otherwise), not by
`#[serde(default)]`, because the wire form writes every field (ADR 0006, rule 3).

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
  answers it, one entry per track, with the source time of the clip that covers it:
  `source_in + (instant − place.start) / speed`, computed in `i128` over flicks and floored to the
  earlier flick. This is pure model code and it is the model's half of the resolver, so it lands in
  `gafira-doc` in `TL2`; the caller maps the answer to `FrameRequest { medium: MediumRef { asset,
  content }, level, index, time }` (`cache.rs:80-95`, `cache.rs:97-125`) — the `content` is the clip's
  `AssetRef`, the `asset` is the provider handle the asset provider holds at runtime (P9), and the
  model names neither.
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
but it waits for this one. And the query is the one piece of `TL2` with no consumer today: it lands
here because it is the model's own question, and because its arithmetic is exactly the part a later
change must not improvise.

Reversible: yes.

### P8. The persisted timeline joins schema 1; the trigger is the first `.gafira` written for a user

ADR 0006's rule 1: schema 1 stays open until the first build that writes a `.gafira` reaches a user;
until then, additive fields join schema 1, `Document::SCHEMA_VERSION` stays 1 and no migration is
written. The facts of the repo: `gafira-format` does not exist (`crates/gafira-format` is not a
member), `gafira-cli` prints only `--version` and `--help` (`apps/gafira-cli/src/main.rs:41-57`), and
no code outside the tests and `gafira-testkit` calls `Document::to_json`. No build has ever written a
`.gafira`, so no build has reached a user with one.

Decision: the timeline's fields join schema 1, with no migration, and every field is always written
(ADR 0006, rule 3) with `#[serde(default)]` on read (rule 1). The rule to re-check is ADR 0006's
own: the coordinator checks it when `gafira-format` is planned. Q1 asks the owner to fix the order.

Cost: the goldens of the document move in `TL3` (the byte-pinned golden
`crates/gafira-doc/tests/golden/full.json` is the evidence of rule 3), and the old bytes stay as an
input-only fixture that must load and re-save in the new canonical form (ADR 0006's consequence).

Reversible: yes, while schema 1 is open.

### P9. A clip identifies its media by content hash, and that is what a resource that changes means

`ClipSource::Media(AssetRef)` needs a reference the document can hold, and §3.1's `AssetTable` does
not exist. The tempting shortcut is a numeric asset id per entry; §3.1 already refuses it:
`AssetRef(ContentHash)` and "la identidad del recurso sigue siendo su hash, de modo que relocalizarlo
no cambia ningún nodo ni clip" (§3.1, line 288). ARQ-145's table says the same from the cache side —
the frame key uses "el hash del contenido del archivo, **no** el id del recurso en el proyecto" — and
`ProviderAssetId` is explicitly "the id of an asset inside one provider"
(`crates/gafira-ports/src/assets.rs:29-31`): a runtime handle for opening a file, not a document
identity.

Options: **(a)** `AssetRef(ContentHash)` alone, no table, no referential check; **(b)** `AssetRef`
plus a minimal `AssetTable` **keyed by** `AssetRef`; **(c)** an `AssetTable` keyed by a new numeric id
(the design of the first draft of this section).

Decision: **(b)**. `AssetRef(pub ContentHash)` is a newtype whose wire form is the 64-character hex
hash (the same spelling `assets/<blake3-hex>` uses, §7.1), and `AssetTable` is a `BTreeMap<AssetRef,
AssetEntry>` with the two fields the document needs to locate the bytes: `name` and `storage:
AssetStorage::{Embedded | Linked}`. Validation then has something to check — every `Media` and `Audio`
clip names an entry of the table — which is what (a) gives up, and (c) buys the same check by
inventing a second identity that §3.1 forbids. The metadata of the import (duration, rate, legal code
class) comes with the import feature, which is where it is produced.

**What a clip means over a resource that changes.** Because the reference is the hash, the answer is
the content-addressed one, and it is worth stating because the acceptance of FV-02 and FV-21 depends
on it: replacing a file on disk gives a new hash, so it is a *new* asset and every existing clip keeps
playing the old content, untouched. Relinking a *linked* asset rewrites `AssetStorage::Linked`'s path
for the same `AssetRef`; no clip, node or timing changes. A linked file that is gone leaves the
`AssetRef` and its clips intact and is reported as a missing link (§13's `LinksMissing`), not as a
document that fails to load.

Reason: one identity, the one the spec fixed, and no migration when `Project` and `gafira-format`
arrive: the table moves into `Project.assets` with the same keys.

Cost: the table is one more persisted structure in `TL3`, and `ContentHash` gains a serde form through
`AssetRef` (the type itself stays plain bytes, `content_hash.rs:1-11`; its doc comment "the wire form
arrives with `AssetTable`", line 8, is what `TL2` replaces with the newtype).

Reversible: yes, while schema 1 is open (Q1).

### P10. The edge breaks ARQ-136's acceptance criterion, and `TL1` rewrites the test

ARQ-136 says the domain "no nombra ningún tipo de tiempo en este corte" and that the `gafira-doc →
gafira-time` edge does not exist. Its acceptance is executable: the test
`arq_136_the_measure_context_carries_no_time` in `crates/gafira-doc/tests/visit_contract.rs` reads
every source file of the crate (`include_str!`, `DOMAIN_SOURCES`) and fails if the text names
`Flicks`, `FrameRate`, `Instant`, `Duration` or `SystemTime` — and it also fails when the crate
declares a module that its source list does not cover (`visit_contract.rs:221-241`). A `timeline`
module naming `Flicks` therefore cannot coexist with the test as written, in any delivery order.

Decision: `TL1` rewrites that test to keep what the row is really about and drop what the edge
invalidates. The `MeasureCx` field assertion stays exact (`page` and `dpi`, no time field,
`visit_contract.rs:202-217`); the crate-wide name scan is removed, and its replacement is a narrower
one: no type of time may appear in `visit.rs`'s `MeasureCx` and no time field may be added to
`PageView`. The module-coverage loop stays.

This is a **spec change**, not a test-only change: ARQ-136's command clause ("el bloque `pub struct
MeasureCx` ... declara exactamente `page` y `dpi`") survives, its crate-wide clause does not.
`specifier` rewrites the row and `docs/adr/0007`'s pairing is the precedent for doing it in the same
change that brings the edge. Q-54's options (a) and (c) both end here; (b) avoids it.

Reason: keeping the crate-wide scan would force the timeline out of `gafira-doc` (P1's option (c)),
which the crate table of §2.1 does not have.

Cost: one test rewritten, one ARQ row rewritten by `specifier`, and the guard that "the domain never
names a time type" becomes a review rule instead of a grep. That is the honest price of Q-54's (a).

Reversible: only by removing the edge (P2), which is not reversible.

### P11. One clip per track at an instant: no overlap on any lane, and stacking is the order of `tracks`

Options: **(a)** no track allows overlap, and clips are ordered and disjoint within a track; **(b)**
`Video`, `Audio` and `Caption` tracks are disjoint and `Overlay` tracks may overlap; **(c)** overlap is
legal and the last clip in the vector wins.

Decision: **(a)**. FV-01 draws five lanes and a lane holds one strip at an instant; §3.1 gives every
`Track` a single `clips` vector with no z field, so (c) would make the result depend on insertion
order and would give the resolver two answers for one instant. (b) is the rule an NLE would need if a
cross-dissolve were an overlap, but it is not one here: 02 §4.13 and D-50 put the transition on the
cut, as a `TransitionInstance` whose `align` is `Centered | Start | End` "respecto al corte", and
FV-06 says that when the handle frames are missing "se repite el fotograma de borde y se avisa" —
which is only possible while the two clips keep their cut and do not overlap. Stacking belongs
*between* tracks, which is why `tracks` is ordered: index 0 is the bottom, later tracks paint over
earlier ones, the same convention `Page.roots` uses ("back to front", `page.rs:47-49`). The five
lanes of the interface are a mapping from `TrackKind` (plus the label of the track for `Texto`), never
a second ordering.

Consequence for the resolver (P7): at most one clip of a track covers an instant, so
`Sequence::active_clips(t)` answers at most one clip per track and the answer is a function of the
document alone.

Cost: a cross-dissolve that needs frames no longer in either clip is a trim problem, not an overlap
problem; the commands change owns it.

Reversible: yes, while schema 1 is open; an overlap rule later would be a new validation rule and a
new resolver order.

## Critical code

The code below is the critical code of each delivery; the coordinator writes the hand-mutant lists
from it, rule by rule, with the wrong behaviour and no code.

### `TL1` — the edge and `Page.duration`

- The dependency: `crates/gafira-doc/Cargo.toml` gains `gafira-time.workspace = true`, the crate doc
  that says "depends only on `gafira-geom` and `gafira-color`" and its `Document -> Page -> Node`
  line (`lib.rs:1-4`) are corrected, and the nine V5 rows that must gain `gafira-time` list it in the
  same commit (ADR 0007): `gafira-doc`,
  `gafira-edit`, `gafira-testkit`, `gafira-pathops`, `gafira-layout`, `gafira-text`, `gafira-raster`,
  `gafira-segment`, `gafira-render-cpu`. A row without the crate, or the crate without the rows, fails
  the V5 step. The row of `gafira-render-cpu` is owned by the change in flight today, so `TL1`
  sequences behind it; the root `Cargo.toml` needs no edit.
- The conversion of the duration: a page duration is a `Flicks` value ≥ 0, the wire form writes it
  always and reads it with a default (`None`), and it is inserted where §3.1 puts it (after `grid`,
  before `locked`) so the canonical bytes stay in field order.
- The ARQ-136 guard (P10): `arq_136_the_measure_context_carries_no_time` keeps the exact
  `MeasureCx` assertion and the module-coverage loop, and drops the crate-wide name scan. The name
  `Flicks` in a `gafira-doc` source is expected from `TL1` on, so a test that still greps for it is
  a test that cannot pass.
- The goldens: all seven move, because the field is always written — `tests/golden/minimal.json`,
  `full.json`, `extension.json`, `extension_noncanonical.json` and `pre_layout/{minimal,full,
  extension}.json` — and the old bytes stay as an input-only fixture that must load and re-save in the
  new canonical form.

### `TL2` — the model and its laws

- The frame grid: the boundary test of a tick against the sequence's rate, the conversion of a frame
  index to a tick and back, and the 23.976 case (24 000/1 001, 29 429 400 flicks per frame). A tick
  that is not a boundary is reported, never rounded.
- The bounds of a clip: the start and the end on the grid, the duration of at least one frame, the
  source offset ≥ 0, and `TimeRange`'s half-open end, so the last frame of a clip is the frame whose
  start is `end − frame`.
- The order and the overlap rule of P11: the clips of a track are sorted by start and disjoint, in
  every lane; a track's index is its stacking order, index 0 the bottom.
- The cap of the audio tracks: sixteen, and the seventeenth is refused with a message.
- The relation between `speed` and the duration: the place of the clip is the source duration over
  the speed, rounded to the grid, and the source offset absorbs the remainder.
- The retiming rule as a **pure** function: `Sequence::retimed(&self, new_rate) -> Sequence` moves
  every cut to the nearest frame of the new grid and loses no clip (FV-01's acceptance (c)). It
  returns a new sequence and mutates nothing, because only a command may write (ARQ-30) and the
  commands are the following change (P6). The nearest-frame tie is broken towards the later frame, so
  the function is total and deterministic. A clip that would fall below one frame keeps one frame and
  pushes the rest of its track right — "sin perder clips" is the acceptance's own words, and the
  alternative (a collision that leaves the document invalid) loses a clip.
- The resolver query (P7): the binary search of `Track::clip_at` over the clips a track keeps sorted,
  the half-open interval at both ends (an instant at `place.end` belongs to the next clip, not to this
  one), `Clip::source_time` with the `i128` arithmetic and its floor, the exact identities at `speed`
  1/1, an instant outside every clip (no entry), an empty track, and a sequence whose tracks cover
  different instants.
- The `DocMut` accessors: `sequence_mut`, `track_mut`, `clip_mut` and the three id allocators, so the
  timeline is mutable only with the `EditToken` (ARQ-30) and the commands change has one path to it.

### `TL3` — the persistence

- The wire form of the sequences and of the assets: the ids as plain numbers, the enums with their
  tags, every field written, the defaults on read; `AssetRef` as the 64-character lowercase hex hash.
- The wire of the root: `DocumentWire` (`json.rs:38-45`) gains `sequences` with `#[serde(default)]`,
  so a document that predates the field reads as empty (ADR 0006 rule 1), and `Document` writes it
  always; the error path of `Document::from_json` is unchanged.
  The doc comments that name the document's contents ("a version, one id counter, a DPI and ordered
  pages", `document.rs:10-12`; "nodes and pages" on `next_id`, `document.rs:35`) are corrected with it.
- The validation: the ids of the sequences, tracks and clips are unique in the whole document and
  below `next_id` (the rule P2 of the existing validation, extended: `check_ids`'s iterator gains the
  three new id spaces, `validate.rs:43-67`), every `Scene` names an existing page, and every `Media`
  or `Audio` clip names an entry of the asset table (P9). The `Scene` rule has a consequence worth
  naming: `RemovePage` on a page a clip shows fails with `InvalidDocument` today, which is the safe
  half-state; FV-05 (c) ("borrar la escena elimina su clip y es reversible") is the commands change
  deleting both before it validates.
- The goldens: the canonical bytes of a document with a timeline, including a clip of each source
  kind, and the seven old bytes of `TL1` still re-saving byte for byte.

## What this change cannot prove

- **The video editor.** The five lanes, the magnet, the ripple as the user feels it, the keyboard
  navigation, the fps picker and the 16-audio-tracks message are layer 4 (planned, `gafira-ui`); this
  change
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
- **The relink of a linked asset.** P9 says what it means (the same `AssetRef`, a new path, no clip
  touched), but the command that performs it and the `LinksMissing` report are FV-21's change; this
  change only persists the storage entry the relink rewrites.
- **The cross-dissolve.** P11 rules overlap out and points at `TransitionInstance` on the cut
  (02 §4.13, D-50); the transition's own frames are not modelled here.
- **The ARQ-136 guard by grep.** After P10 the rule "the domain never names a type of time" is a
  review rule about `MeasureCx`, not a search over the crate. What stays proven is the context's
  field list.
- **Deleting a scene.** FV-05 (c) says deleting a scene deletes its clip, and that is a command; the
  model only refuses the half-state where a clip shows a page that is gone (`InvalidDocument`).
- **The magnet.** FV-01's attraction to the playhead, cuts, markers and frames needs a zoom (pixels per
  flick) and a threshold in pixels, which are view data; the model offers the cut positions and the
  frame grid it attracts to, and nothing else.

## Open questions (owner)

**Q1. The schema: may the timeline join schema 1, and does it land before the first `.gafira` reaches a user?**

Recommendation: **yes to schema 1, and yes to the order.** ADR 0006's rule 1 holds today, and the
evidence is mechanical: `gafira-format` does not exist, `gafira-cli` prints only `--version` and
`--help` (`apps/gafira-cli/src/main.rs:41-57`), and nothing outside the tests and `gafira-testkit`
calls `Document::to_json`. So no build has written a `.gafira`, the timeline's fields are additive,
and `Document::SCHEMA_VERSION` stays 1 with no migration. The rule closes when the first build that
writes a `.gafira` reaches a user; the check belongs to the moment `gafira-format` is planned
(ADR 0006's own trigger). If the owner prefers to have the container first, the timeline's fields
become schema 2 with a `migrate_1_to_2` and its golden, and this change grows by the migration and
by the fixtures it needs.

**Q2. The `gafira-doc → gafira-time` edge: is the V5 churn acceptable?**

Recommendation: **yes.** ADR 0007 already decided the edge arrives with `Page.duration` and that it
touches those rows as one commit; V-2 fixes the internal time to the flick, so the domain that holds
the timeline holds flicks; Q-54's recommendation (a) says the same. The alternative — the domain
keeping its own integer unit — duplicates a type that already exists and puts a conversion on every
field of the model. The cost is one commit with one manifest edit and nine allow-list rows in
`ci.yml`, which is why `TL1` exists as a delivery of its own; and it forces Q6.

**Q3. The clip's animated volume: a bare `f32` now, or wait for `gafira-anim`?**

Recommendation: **a bare `f32` now** (P3). `ElementBase` is behind §3.2 today (`element.rs:12-32`
has `opacity: f32`, not `Animatable<f32>`), and the animation of the timeline is not in the acceptance
of any row this change closes. Waiting for `gafira-anim` would make the change wait for a second edge
and a second layer-2 crate. The deviation goes to the delta specs, not to the notes of `TL2`.

**Q4. The commands and the resolver: one change or three?**

Recommendation: **three.** The model is the foundation; the commands need the model's laws and two
extensions in `gafira-edit` (`Property`, `MergeKey`); the resolver needs the model and adds nothing
to the ports. Folding them in would grow this change by roughly 1 400 authored lines (×1.4: about
2 000) and would put the commands before the laws they rely on are verified against a persisted
document. The owner may prefer to pay the size once; the order of the deliveries would then be
`TL1`, `TL2`, `TL3`, then the commands, then the resolver, with the commands' delivery sized
against the model that exists by then.

**Q5. The asset table: the minimal table now, keyed by the hash, or nothing until `Project.assets`?**

Recommendation: **the minimum now, keyed by `AssetRef`** (P9): the content hash, the name and the
storage. It is what lets validation refuse a `Media` or `Audio` clip whose entry is gone, and it moves
into `Project.assets` unchanged when `Project` lands. The alternatives are no table (no referential
check for media clips until `Project`) and a table keyed by a numeric id, which §3.1's
`AssetRef(ContentHash)` and ARQ-145's cache key both contradict. The owner may prefer the whole table,
which would move the import's metadata into a change that does not import anything.

**Q6. ARQ-136: who rewrites the row, and does the owner accept a narrower guard?**

Recommendation: **`specifier` rewrites it in this change, and the owner accepts the narrower guard**
(P10). What the row is about is the measurement context: `MeasureCx` carries `page` and `dpi` and no
field of time (`visit_contract.rs:202-217`), and that assertion stays exact. What the timeline
contradicts is the crate-wide clause ("el crate `gafira-doc` no nombra ningún tipo de tiempo") and its
grep over every source file (`visit_contract.rs:221-241`). If the owner prefers to keep the crate-wide
clause, the only way is Q-54's option (b) — the domain keeps its own integer unit and converts at the
border — and P2, P4 and P10 of this design change shape; that is a bigger cost than one spec row,
which is why the recommendation is the rewrite. This is the one question that blocks `TL1` itself:
the delivery cannot pass its gate while the row and its test still forbid the name.

## Notes for the coordinator

- The gates of this change run with the **workspace** scope, because `gafira-doc` is not a leaf crate
  (ARQ-01) and because `TL1` touches `ci.yml`. ADR 0017's `-p <crate>` scope applies to the leaf crates
  only.
- `TL1` is the only delivery that touches `.github/workflows/ci.yml`; it touches no `Cargo.toml` of the
  workspace root, only `crates/gafira-doc/Cargo.toml`. The `gafira-render-cpu` change owns the V5 row
  of `gafira-render-cpu` today, so `TL1` sequences behind it, not blind.
- `TL1` waits on Q6: the ARQ-136 row and its test are part of the same delivery.
- The hand-mutant lists of `TL2` and `TL3` name the laws above; each law is one mutant, and the
  mutants are killed by the unit tests of the law, not by the golden.
- The verify depth of this change is V2 (code, with the laws). The change touches the document model
  and the wire form, so its golden is part of the evidence.

## Owner decisions taken (2026-11-05)

All six open questions are settled. `Q3` was decided by the coordinator as a reversible reading and is recorded with the rest; the others are the owner's.

| # | Decision | Reason |
|---|---|---|
| `Q1` | **Schema 1, no migration.** The timeline's fields join the schema as additive ones. | No build has ever written a `.gafira`: `gafira-format` does not exist, `gafira-cli` only has `--version` and `v0.1.0-beta.1` attaches no binaries. ADR 0006's rule still holds, and it closes when the first build that writes a `.gafira` reaches a user. |
| `Q2` | **The `gafira-doc -> gafira-time` edge goes ahead**, with the V5 rows of the nine crates it reaches, in one commit. | ADR 0007 already decided it arrives with `Page.duration`; V-2 fixes internal time to the flick. The alternative duplicates a type and puts a conversion on every field of the model. |
| `Q3` | **Bare values now** for the clip's animated volume and the base's animated transform. Option (a). | Reversible and recorded as a deviation of `TL2`. Waiting for `gafira-anim` would make the change wait for a second edge and a second layer-2 crate, and the animation of the timeline is not in the acceptance of any row this change closes. |
| `Q4` | **Three deliveries** (`TL1`, `TL2`, `TL3`); the commands and the resolver go to their own changes. | The model is the foundation; the commands need the laws verified against a persisted document, and folding them in would grow this change by about 1 200 authored lines. |
| `Q5` | **The minimum asset table now**: id, name, content hash and storage. | The import's metadata arrives with the change that imports, which is where it is produced. |
| `Q6` | **ARQ-136 is rewritten with the narrow guard**, and `specifier` does it in this change. | Half 1 of the row — `MeasureCx` carries no time field — is exact and stays. Half 2 — a crate-wide ban on naming a time type — described the crate before the timeline existed and is obsolete rather than violated. Keeping it would force Q-54's option (b) and reshape `P2`, `P4` and `P10` at a cost far above one spec row. |

The verify depth of this change is **V1**, one round per group of deliveries, by the owner's decision of the same date: the rigor of `O4`, `O8` and `O10` applies here too, with hand mutants on the critical code, gates per commit and no crate-wide T4 at close.
