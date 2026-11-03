# Spike: layout-core

Facts only. Scratch: `/tmp/claude-1000/-home-meridian-Documentos-Proyectos-Personal-gafira/d6202df9-be61-47bb-9426-75348061c106/scratchpad/` (`ws/` = copy of the workspace with `Page` edited; `prec/`, `rs/` = number and rstar checks). Toolchain 1.93.1 (pinned by `rust-toolchain.toml`). The repo was not modified except this file.

## 1. Does adding `Page` fields need a new format version?

Evidence:
- Version gate: `crates/gafira-doc/src/document.rs:28` `SCHEMA_VERSION: u32 = 1`; `validate.rs:29-33` rejects any `schema_version != 1` (`UnsupportedSchemaVersion`). So a bump to 2 makes every existing file and golden fail to load unless a migration is added (02 §7 expects chained `migrate_n_to_n1` with goldens, ARQ-53; 02:1177 "newer schema = error").
- `Page` has no `deny_unknown_fields`; test `page_defaults_required_fields_and_unknown_fields` (`page.rs`) and `tests/serde.rs::unknown_fields_are_ignored_and_dropped` pin that extra keys are ignored and dropped on re-save. 02:391 says deny_unknown_fields is deliberately off.
- Wire format is pinned byte for byte by: `page.rs::page_wire_form_writes_every_field`, `json.rs::minimal_document_has_exact_bytes` and `full_golden_equals_the_built_document`, `tests/golden.rs` (3 canonical goldens re-saved byte for byte + 1 noncanonical), `tests/serde.rs::payload_numbers_and_keys_get_one_canonical_spelling`. Existing optional `Page` fields use `#[serde(default)]` (name, locked, hidden, roots, nodes).

Experiment A (scratch copy, add `bleed: f64` and `notes: String` with plain `#[serde(default)]`, always serialized), `cargo test -p gafira-doc --no-fail-fast`:
```
unittests: FAILED. 98 passed; 3 failed   (minimal_document_has_exact_bytes, full_golden_equals_the_built_document, page_wire_form_writes_every_field)
tests/golden.rs: FAILED. 1 passed; 2 failed  (canonical_goldens_round_trip_byte_for_byte, noncanonical_golden_normalizes_to_the_canonical_one)
tests/serde.rs: FAILED. 16 passed; 2 failed  (payload_numbers_and_keys_get_one_canonical_spelling, unknown_fields_are_ignored_and_dropped)
edit/properties/others: ok.   cargo check --workspace --all-targets: ok (nothing else builds a Page literal; only Page::new)
```
Failures are byte-comparison only. An added test loading the three OLD goldens (`minimal`, `full`, `extension`) with the new build: `test old_goldens_load ... ok` (new fields take defaults, schema_version stays 1).

Experiment B (same, but `skip_serializing_if` default value: `is_zero` / `String::is_empty`): all suites ok (101 + 12 + 3 + 6 + 18 + 1 + 10 passed), goldens unchanged byte for byte.

Facts:
- Old documents load with serde defaults and `schema_version` 1: confirmed (A).
- A new document read by a build without the fields silently drops them on re-save (unknown fields ignored). No read-only protection, because the version is the same. A consequence of not bumping, not a defect today.
- With always-serialize, 7 existing tests and 3 goldens must be updated (list above). With skip-if-default, none change, but then the wire form is not "every field written" and default-valued fields are absent from the canonical form.
- Verdict: a new `schema_version` is NOT technically required (confirmed). Whether to bump is a decision (D1). If bumped, the coordinator floor `coordinator.md:25` "new serialized format version" => full.

## 2. Types and units

Evidence (`gafira-geom`, `gafira-doc`):
- `gafira_geom::Length { value: f64, unit: Unit }` (`length.rs`), `Unit = Px|Mm|Cm|In|Pt`, private fields, `Length::new` const, no validation, `convert(to, dpi: f64)` and `to_px(dpi: f64)` return `Result<_, ConversionError { InvalidDpi, NonFiniteValue, NonFiniteResult }>`. Serde form `{"value":..,"unit":"px"}`. A `Length` as in `GridSettings { step: Length }` exists.
- `gafira_geom` re-exports kurbo `Affine, Point, Rect, Vec2` (f64) plus `Size` (f64 width/height, private fields), `Transform2D`. Rect exists (kurbo); `Affine::transform_rect_bbox` exists in kurbo 0.13.1.
- `gafira_doc::PageSize { width: Length, height: Length }` is the page dimension type (pub fields). `PageSize::MIN_PX = 1.0`, `MAX_PX = 16_384.0` (inclusive, checked in px at the doc DPI, `check(dpi)`); `to_px(dpi) -> Size`.
- `Dpi(u32)`, `MIN 1`, `MAX 10_000`, default 96, stored on `Document`, not on `Page` (so `Page`-level functions need the DPI as input to convert units).
- `Placement { x, y, w, h, rotation_deg, skew: f64, flip }` is f64 in px ("logical px"); `ElementBase.opacity` is f32 (not relevant here).
- Everything length-like is f64. No f32 length exists.

Experiment (`prec/`, `cargo run`):
```
dpi 96:    16384px = 4334.93 mm      dpi 10000: 16384px = 41.61536 mm
dpi 1:     16384px = 416153.6 mm (16384 in)  (a page of 416153.6 mm is valid at dpi 1)
f64 ulp at 16384 = 3.6e-12; at 416153.6 = 5.8e-11
f32 ulp at 16384 = 0.00195; at 416153.6 = 0.03125
```
- Verdict: with f64 (what the code uses) 0.01-unit precision is satisfiable at every size the code allows (ulp ~1e-11 << 0.01). With f32 it would fail for mm at low DPI (ulp 0.03 > 0.01) and be marginal at 16384 px (ulp 0.002). Confirmed: use f64.
- The spec says "precisión de 0,01 unidades" (FD-03a) and "+-0,01" (FD-11a,b) without saying whether the stored value is rounded to 0.01 or only compared with tolerance 0.01, nor in which unit (px, project unit, or page unit). Decision D2.
- 02 §3.1 leaves types unnamed for `bleed`, `margins`, `background`; `duration: Option<Flicks>`. `Flicks` is in `gafira-time` (`flicks.rs`, `Flicks(i64)`); `gafira-doc` does not depend on `gafira-time` (see item 5).

## 3. Non-finite and degenerate input convention

Evidence:
- `Length` (`length.rs` docs): "Construction does not validate"; non-finite cannot be converted (`Err(NonFiniteValue)`) nor persisted (JSON `null` fails to load). Same for `Size`, `Placement`, `Dpi` (stored unvalidated, validated later).
- `gafira-doc/validate.rs:220-226`: per-node numbers are checked finite first (`ValidationError::NonFinite { node, field }`), then range (`OutOfRange`); first failure wins. Test `json.rs::non_finite_numbers_serialize_as_null_and_do_not_load`. `PageSize::check` returns `Conversion{..}` for non-finite sides, `OutOfRange` otherwise.
- `gafira-pathops`: `Path::validate_finite()` runs before any geometry (`path.rs:136-141`), typed `PathOpError` carrying the kind (`NonFiniteValue::{Nan,..}`); non-positive/non-finite tolerance and stroke width are errors.
- `gafira-anim/track.rs:124`: a key with non-finite value is rejected by the track invariant.
- `gafira-geom/transform.rs:60-67`: `inverse` returns `Err(SingularTransform)` for det 0 or non-finite; "Never returns NaN coefficients". `Placement::to_transform` is "total: non-finite fields give non-finite coefficients" (test `placement.rs:148`).
- Verdict: convention exists and is consistent: constructors and plain data do not validate; pure functions/validators check finiteness first and return a typed `Err` (no panic, no NaN output); degenerate geometry is an `Err`, not a silent value. The convention does not say what a layout function does with a non-finite page rect or node bounds (return `Err`, skip the node, or ignore). D3.

## 4. Node bounds in page coordinates

Evidence:
- `Placement::to_transform() -> Affine` (`placement.rs:64`) maps the local box `[0,w]x[0,h]` to page space (mirror, shear, rotate, translate). Doc says `x`,`y` are "page-space" top-left of the box.
- `NodeKind` has only `Group`, `Shape(rect|ellipse)`, `Extension` (`node.rs:64-71`). There is NO text node and NO image node in code (grep: no `TextNode`/`ImageNode`). `Group` has `children: Vec<NodeId>` and its own `Placement`.
- No function returns bounds. `grep bounds|bbox|transform_rect_bbox` in `gafira-doc`, `gafira-edit`, `gafira-anim`, `gafira-pathops`: none. 02:430 declares `NodeBehavior::local_bounds(&self, cx: &MeasureCx, t: Flicks) -> Rect` and 02:264 `layout::overflow(bounds, page)`, but neither trait nor `MeasureCx` exists.
- Nothing composes a child transform with its group parent. Whether a child's `Placement` is page-space or relative to the group is not stated in code or in 04 (the `Placement` doc says page space for all nodes).
- Missing for align/distribute/overflow/snap: (1) a per-node page-space AABB (needs `kurbo::Affine::transform_rect_bbox(rect)` over `Rect::new(0,0,w,h)`, which exists, but no code calls it); (2) the group/child space rule; (3) bounds for text/image (kinds absent; Extension bounds need an unbuilt registry); (4) a rule for stroke/effects (shape has no stroke yet). A pure `layout` function can take `Rect` inputs and avoid all of this; building them from a `Page` is the open part. D4.

## 5. Crate location of Guide/GuideSet, V5 table, deps

Evidence:
- FD-03 (`04:62-68`) "Tecnica: `gafira-layout` (`Guide`, `GuideSet`)" and "se guardan en el documento (`Page.guides`)". 02 §3.1 (`02:192`) puts `guides` in `Page` (crate `gafira-doc`). 02 §2.2 (`02:136`): `layout -> geom, doc`. `gafira-doc` cannot depend on `gafira-layout` (cycle; V5 would also reject it). Conflict CONFIRMED: the type stored in `Page.guides` must be defined in `gafira-doc` (or `gafira-geom`); `gafira-layout` can only use it. This changes FD-03's crate attribution (D5).
- V5 table in `.github/workflows/ci.yml` (step "Domain crates respect the layer table (V5)") rows: geom, color, time, doc (geom color), edit, testkit, pathops (geom), anim. No `gafira-layout` row. The same step fails with `"$name has clippy.toml but no V5 row"` for any crate directory that has a `clippy.toml` (V12 requires a byte-identical copy for every V5 crate). So the new crate needs: a V5 row (`gafira-layout gafira-geom gafira-doc`; the edge is stated in 02:136) and `crates/gafira-layout/clippy.toml` copy. Floor `coordinator.md:25`: "New V5 layer-table row ... => full". Line 42 says a new crate that 02 describes is standard "full only on a full floor"; the V5 row is that floor. Hit: full-mode floor.
- `duration: Option<Flicks>` needs `gafira-doc -> gafira-time`. The V5 row for `gafira-doc` is `gafira-geom gafira-color` only. 02:130-136 draws time -> anim -> doc, so the edge is arguably stated in 02, but the CI row for `gafira-doc` would change. Either way, a V5 row edit => full-mode floor (D6). (`gafira-anim` also would be in the tree only if doc depends on it; not needed for `duration`.)
- `deny.toml`: licences allowlist MIT, Apache-2.0, ...; `bans` list names (no rstar); `multiple-versions = "warn"`. Experiment: scratch crate with `rstar = "0.12"` and the repo `deny.toml`: `cargo deny check licenses bans sources` => `bans ok, licenses ok, sources ok`. Tree: rstar 0.12.2 -> heapless 0.8.0 (hash32, byteorder, stable_deref_trait), num-traits (libm), smallvec. None starts with the V5 forbidden regex (`egui|eframe|winit|wgpu|tokio|mio|socket2|reqwest|ureq`). `rstar` is not in `Cargo.lock` or `[workspace.dependencies]` today; adding it is a workspace manifest change. A registry-resolved `rstar` 0.13.0 is also in the local cargo cache (the "última" of 02:1325).
- Inconsistency: 02:1325 lists rstar as "VERIFICADA (cargo-deny)"; FD-06 `04:90` says "rstar, VERIFICAR licencia". The experiment confirms the licence passes.
- Not a new edge: rstar is external, so no V5 gafira-* edge. Only the new crate row and (if D6 chosen) the doc row.

## 6. FD-04(a) consistency

Text: "12 columnas con espacio 24 px en 1080 px genera 11 guias de separacion y 2 de margen". Formula in FD-04: `columnas x ancho + (columnas-1) x espacio` must fit in "area util". Inputs listed: columnas, filas, espacio, margen (no column width).

Computation (`prec/` output + arithmetic), 12 columns, gap 24, page 1080:
- margin 0: column width = (1080 - 11*24)/12 = 68 (exact). margin 24: 64. margin 40: 61.3333333 (needs rounding to 0.01, rule absent).
- Guide counts by reading:
  - R1, one guide per gap (centre line) + 2 margin: 11 + 2 = 13. Matches the sentence literally.
  - R2, each gap bounded by two column-edge guides: 11 gaps x 2 = 22 separation guides + 2 margin = 24. This is the usual column model (12 columns x 2 edges = 24 edges, of which the outer 2 coincide with the margins).
  - R3, column edges only: 24 total (22 inner + 2 outer), with the outer 2 being the margin guides. "11 de separacion" does not appear.
  - R4, guides = boundaries between consecutive gaps and columns counted once: 11 inner gaps give 22 inner boundaries; no reading yields exactly 11 except R1.
- The sentence is consistent only with R1 (gap = a single guide). It contradicts the formula's model (gap = space between columns, i.e. two edges) unless "espacio" is represented by one guide. So the criterion is NOT well defined: refuted as written (needs D7).
- Zero margin: the "2 de margen" guides would sit at x=0 and x=1080 (the page edges). The spec does not say whether they are created, omitted, or merged with the page-edge snap targets. Undefined.
- Fit formula: since width is derived from the other inputs, `cols*width + (cols-1)*gap = usable` holds by construction; the only failing case is width <= 0, i.e. `(cols-1)*gap >= usable` (also `usable <= 0` when `2*margin >= page`). The spec does not state the minimum column width (>0? >= some value?), nor whether margins come from the input "margen" or from `Page.margins`, nor which axis the rows use (assumed height, silent), nor what "area util" is (page minus margins? minus safe zone?).
- Presets: "12 columnas", "3 columnas", "Rejilla 3x3" have no default gap/margin values in the spec. "Rejilla 3x3" count of guides is silent too.

## 7. Testability of the FD rows at the `gafira-layout` pure-function level

| Row | Criterion | Pure-function level | Needs UI / export | PROPUESTA / VERIFICAR in the row |
|---|---|---|---|---|
| FD-03 | (a) 0.01 precision | yes (round/parse, depends on D2) | typing in dialog | none |
| | (b) guides survive save/open | yes in `gafira-doc` serde (not `gafira-layout`) | | |
| | (c) snap to guide within threshold | yes (`snap`) | | |
| FD-04 | (a) 12 col/24/1080 counts | yes but ill defined (item 6) | | none |
| | (b) cancel leaves doc unchanged | no: dialog/Command flow | UI | |
| | (c) apply + undo restores exact set | needs `SetGuides` `Command` in `gafira-edit`, not layout | | |
| FD-05 | (a) PDF TrimBox/BleedBox | no | PDF export (`gafira-io-pdf`) | "5 % por lado, PROPUESTA"; "valores VERIFICAR por plataforma" (video) |
| | (b) warning appears/disappears | yes: `overflow(bounds, safe_rect)` returns a boolean/excess | message/UI display | bleed default "3 mm en Impresion" tied to a preset, 0-10 mm range stated |
| | (c) margins respected on Adapt | yes only with `reflow` (FD-54) | | |
| FD-06 | (a) 5 px from guide snaps exactly | yes | | threshold 6 px screen, 2-16 px "PROPUESTA"; rstar "VERIFICAR licencia" (verified, item 5) |
| | (b) Alt disables | caller flag only (pure fn gets `enabled`) | key handling | |
| | (c) hidden/locked don't attract | yes (input filter) | | |
| | (d) property: shift <= threshold | yes (proptest) | | |
| FD-07 | (a) step respected in any unit | yes (convert via Length+DPI) | | step 1-500, default 10 (stated range; unit of range not stated) |
| | (b) toggle does not change exported doc | export-level (ARQ-15 golden) | export | |
| | (c) <= 2000 lines at 3200 % | yes if `grid` takes viewport and zoom | | "<25 % shows major grid only" |
| FD-09 | (a) PNG has no pixels outside the page; (b) moving back shows it whole; (c) saved doc keeps the element intact | (a),(b) render/export only; (c) serde round trip (doc crate); `overflow(bounds, page)` amounts (the "+24 px" label) are pure | export, canvas dimming, Capas icon | none |
| FD-11 | (a) left align within +-0.01 | yes | | none |
| | (b) equal gaps +-0.01 | yes | | |
| | (c) one undo | `Command` in `gafira-edit` | | |
| FD-53 | (a) each table size created with exact dims | `assets/sizes.toml` data + page creation; pure test on data/unit conversion possible, creation is a Command | | table values: "VERIFICAR contra la plataforma vigente"; budget 500 ms PROPUESTA |
| | (b) preview within 1/255 | no | render (`gafira-render`) | |
| | (c) custom size honours units/ratio | yes (size calc, lock ratio) | | |
| FD-54 | (a) 20-template set no text outside safe zone | needs text nodes (absent) + test corpus | | budget 1 s PROPUESTA |
| | (b) reading order kept | needs reflow + node order | | |
| | (c) original hash unchanged | Command-level | | |
| | (d) one undo | Command-level | | |

(FD-09 acceptance at `04:116` read and confirmed: `layout::overflow` is its only pure-function part.)

Silent in the specs (checked in 04, 03 backlog `backlog/03-*.md:1090,133`, 00, 02):
- Snap threshold: 6 px of SCREEN, configurable 2-16 px, "PROPUESTA" (04:90; 03:1090 repeats 6 px PROPUESTA). Silent: the conversion from screen px to page units (the function needs a zoom or scale input), and whether the 2-16 range lives in settings (not in `Page`).
- Snap default on/off: 03 only says the Imán "atrae" and has the shortcut `Ctrl+Shift+'`; no default state. 04/03 do not give a snap default for `GridSettings.snap` either.
- Tie rules: when two candidates are equally near (guide vs edge vs grid vs margin), no priority order, no rule between x and y axes, no rule for multiple matches at the same distance. Silent.
- Which node edges snap (bounding box of rotated nodes? only AABB?), and exclusion of the dragged selection itself, and of children of the dragged group. Silent.
- Distribute: 3+ elements; equal space between elements OR between centres (04 lists both: "o entre centros") but not how the user selects the mode, nor sort order (by position on that axis), nor which two stay fixed (assumed the extremes; silent), nor behaviour when total width exceeds span, nor the effect of locked elements in the sequence (locked "no se mueven" but do they count as anchors?). Silent.
- Align: reference page (1 element) vs selection (2+); with locked elements and 1 unlocked element in the selection not stated.
- Rounding rule for results (0.01): silent (D2). Negative sizes, zero-size elements, NaN: silent (item 3).
- `GridSettings`: style (lines|dots) is in FD-07 but not in 02 `GridSettings { visible, step, subdivisions, snap }`; step range 1-500 "unidades" (which unit?); subdivisions u8 1-10.
- `Page.guides`: orientation, position unit, locked flag, bounds of a position (outside page?) are not in 02. FD-03 lets guides be locked and deleted by dragging out.
- 02:209 says `grid` and `notes` already exist in 02 §3.1; the code lacks all of `bleed, margins, guides, grid, notes, background, duration`; the types of `bleed`, `margins` (per-side, linked flag stored or only UI), `background` (Color? Paint?) are not given.
- 00-decisiones has no entries on guides, snap, grid or tolerance (grep: no relevant match).

## Verdicts (counts)

| # | Item | Verdict |
|---|---|---|
| 1 | Adding fields does not require a new schema version; old files load | confirmed (experiment A, B) |
| 1b | Goldens need not change | confirmed only with skip-if-default (B); refuted with always-serialize (A: 7 tests, 3 goldens change) |
| 2 | `Length`, `Rect`, `PageSize` available, all f64; 0.01 precision OK in f64 | confirmed |
| 3 | A non-finite/degenerate convention exists | confirmed (validate at boundary, typed Err); layout policy not stated |
| 4 | Page-space node bounds | refuted: no bounds function, no group-space rule, no text/image nodes |
| 5a | Guide/GuideSet cannot live in `gafira-layout` if `Page.guides` is in `gafira-doc` | confirmed (conflict) |
| 5b | `gafira-layout` has a V5 row | refuted: no row; new row + `clippy.toml` required |
| 5c | `rstar` allowed by `deny.toml` | confirmed (cargo deny ok); not in workspace deps yet |
| 5d | `doc -> time` edge for `duration` already allowed | refuted: V5 row of doc has only geom, color |
| 6 | FD-04(a) well defined | refuted as written (only R1 gives 11+2; zero-margin and fit undefined) |
| 7 | Testability table and silent list | confirmed (read of specs; not executed) |

Confirmed 6, refuted 5 (4, 5b, 5d, 6, 1b-with-A), not verified 0 (item 7 is a document read, listed as confirmed with that note).

## Decisions needed (options, none picked)

- D1 Format: (a) no bump, new fields `#[serde(default)]`, schema stays 1; (b) bump to 2 with a `migrate_1_to_2` and goldens (full floor). And independently: always serialize new fields (update 7 tests + 3 goldens) vs `skip_serializing_if` default (goldens untouched).
- D2 Precision of FD-03(a)/FD-11: store values rounded to 0.01 vs keep full f64 and only compare with a 0.01 tolerance; unit of the 0.01 (px, project unit, page unit).
- D3 Non-finite/degenerate input to layout functions: typed `Err` for the whole call vs skip the offending item vs documented precondition.
- D4 Source of page-space bounds: (a) layout takes caller-supplied `Rect`s only; (b) layout adds `node_bounds(page, id)` from `Placement::to_transform` + `transform_rect_bbox`, with an explicit rule for group children (page-space vs relative); text/image deferred.
- D5 Where `Guide`, `GuideSet` (and `GridSettings`, margins, bleed types) are defined: `gafira-doc` (and FD-03 text amended) vs `gafira-geom`; `gafira-layout` re-exports or only operates on them.
- D6 `duration`/`background` in this change: add `duration: Option<Flicks>` (needs `doc -> time` V5 edge) vs defer `duration` (and `background` type) to the video change.
- D7 FD-04(a): which reading of 11+2 (R1 one guide per gap, R2 two edges per gap = 24, R3 column edges only); behaviour with margin 0; minimum column width; rounding of non-integer widths; default gap and margin of each preset; row axis rule.
- D8 Snap: screen-px threshold conversion input (zoom), tie priority order (guide > margin > grid > page > element or nearest-wins), rotated-node edges, default state of the Imán.
- D9 Distribute: mode choice (equal gap vs equal centres) as an input; ordering; locked elements.
- D10 Whether FD-53 (sizes data, `assets/sizes.toml`) and FD-54 (reflow, needs text nodes that do not exist) are in this change or only `gafira-layout` primitives FD-03..07, 09, 11.

## Full-mode floors hit

- New V5 row for `gafira-layout` (and its `clippy.toml` copy for V12): `coordinator.md:25`. YES.
- `doc -> time` edge (only if `duration` is added now): V5 row change. CONDITIONAL.
- New format version: only if D1(b). CONDITIONAL.
- Cross-cutting design choice the spec leaves open: D5 (type location contradicts FD-03 vs 02 §3.1). YES (`coordinator.md:25`/`:110`).
- Owner decision needed: none found in `00-decisiones.md`.

Next: spec fix / `specifier` after the coordinator decides the floor (full -> `architect`), not `tdd-apply`.
