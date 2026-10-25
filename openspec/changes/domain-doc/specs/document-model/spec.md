# Document Model Specification

## Purpose

`gafira-doc` (domain layer): the pure, in-memory `Document -> Page -> Node` model of 02 §3.1 and §3.2, first product slice. New capability; no existing spec. The wire format is in `document-serde`, invariants in `document-validation`, the mutation capability and id allocation in `edit-capability`. Out of scope: sequences, tracks, clips, assets, styles, identities, masks, effects, `timing`, `Animatable`, text/image/frame nodes, `Path`, stroke, `grid`, `notes`, `bleed`, guides, `gafira-cli validate` (deferred to the `gafira-format` change, design A15, holder confirmation pending).

Scenario type tags: (unit), (property), (integration), (command), (manual). "Dn" after a requirement names the delivery per the design's table: D1 skeleton, ids, `Dpi`, `PageSize`, `BlendMode`; D2a `Placement` and `ElementBase`; D2b `NodeKind`, `Node`, `Page`, arena and `Document` (eleven sub-deliveries (D1, D2a, D2b, D3a, D3b, D4a, D4b, D5a, D5b, D6a, D6b; D1 also shipped as PR 1a and 1b) in all). `Document::from_parts` arrives in D4b and `DocMut` in D5b, so D2b tests that need a `Document` build it with a `#[cfg(test)]` helper inside the crate (private fields are visible there); scenarios that need `from_parts` or `DocMut` carry the later delivery in their title.

## Decisions followed from the design (still open for the holder)

| # | Decision | Design ref |
|---|---|---|
| M1 | The root type is `Document` (not `Project`), with private fields; 02 §3.1 amendment 4. | A7, Q4 |
| M2 | `Dpi` is a `u32` in `1..=10 000`, stored once per document. | A6, Q1 |
| M3 | Page limits apply per axis to unrounded px at the document DPI. | A6, Q1 |
| M4 | `Page::locked` and `ElementBase::locked` are independent flags; the data never cascades. | A16, Q4 |
| M5 | `skew` is one angle in degrees (x shear); `flip` is `{x, y}` booleans; only `w`, `h` must be `>= 0`. | A9, A17 |
| M6 | `ElementBase.name` is `Option<String>`; `Page.name` is `String`. | 02 §3.2 |

## Requirements

### Requirement: Purity and dependencies (D1)

`gafira-doc` MUST have no I/O, MUST use `#![forbid(unsafe_code)]`, MUST import `Affine`, `Size`, `Length`, `Unit`, `ConversionError` (and any other geometry type) from `gafira_geom`, never from `kurbo`, and MUST NOT depend on any `gafira-*` crate other than `gafira-geom` and `gafira-color`. Its normal dependencies MUST be `gafira-geom`, `gafira-color`, `serde` (with the `rc` feature declared in this crate's manifest only) and `serde_json`. `proptest` MAY appear as a dev-dependency only. No new third-party package MAY enter `Cargo.lock`. Every public value type MUST implement `Serialize` and `Deserialize`, except error and helper types (`Axis`, `PageSizeError`) and `Document`, whose `Deserialize` is validating (see `document-serde`). Serde for `Placement`, `Flip`, `ElementBase` and the other D2 types lands in D3 (`design.md` commit plan); until then those types carry no serde derives.

#### Scenario: Normal dependency tree (command)
- GIVEN the crate
- WHEN `cargo tree -p gafira-doc -e normal,build --target all --prefix none -f '{p}'` runs
- THEN the `gafira-*` lines are only `gafira-doc`, `gafira-geom`, `gafira-color`, the direct dependencies are the four above, and no UI, engine or I/O crate appears

#### Scenario: Dev tree bounded (command)
- GIVEN the crate
- WHEN `cargo tree -p gafira-doc -e normal,build,dev` runs
- THEN the additions over the normal tree are `proptest` and its transitive crates only (no `rusty-fork`, no `tempfile`)

#### Scenario: No kurbo and no new package (command)
- GIVEN `crates/gafira-doc` and `git diff Cargo.lock`
- WHEN searched for `kurbo` and for added packages
- THEN `kurbo` is absent from the crate manifest and sources, and the lock gains only the `gafira-doc` package

#### Scenario: I/O and unsafe rejected (integration)
- GIVEN a scratch `std::fs::read` call and, separately, a scratch `unsafe {}` block in `gafira-doc`
- WHEN `cargo clippy -p gafira-doc -- -D warnings` and `cargo check -p gafira-doc` run
- THEN the first fails with `disallowed_methods` and the second with an unsafe-code error

### Requirement: NodeId and PageId (D1)

`NodeId(u64)` and `PageId(u64)` MUST be distinct types (passing one where the other is expected MUST NOT compile). Both MUST be `Copy`, `Eq`, `Ord`, `Hash`, expose `new(u64)` and `get() -> u64`, `Display` as the bare number, and serialize as a bare JSON number. Construction MUST NOT validate; `u64::MAX` MUST be representable and MUST survive a serde round trip (it is rejected later by validation).

#### Scenario: Types are distinct (integration, compile-fail doctest)
- GIVEN a function taking `NodeId`
- WHEN it is called with a `PageId`
- THEN compilation fails (the doctest is annotated `compile_fail`; the error code is documentation only, because stable rustdoc does not verify it)

#### Scenario: Order, accessors and Display (unit)
- GIVEN `NodeId::new(2)`, `NodeId::new(10)` and `PageId::new(7)`
- WHEN compared, read with `get()` and formatted
- THEN `2 < 10`, `get()` returns the inputs, and the text is `2`, `10`, `7`

#### Scenario: Transparent wire form (unit)
- GIVEN `NodeId::new(5)` and `NodeId::new(u64::MAX)`
- WHEN serialized and deserialized
- THEN the JSON is `5` and `18446744073709551615` and both round-trip exactly

### Requirement: The single id counter (D2b)

A `Document` MUST hold one `next_id: u64` shared by nodes and pages. `Document::default()` MUST have `schema_version = 1`, `next_id = 1`, `dpi = 96` and no pages. Ids MUST NOT be derived from content. The allocation rules (return `next_id`, then increment; fail at `u64::MAX`) belong to `DocMut` and are specified in `edit-capability`. `IdSpaceExhausted` (`Copy`, `Eq`, `Display`, `Error`) is public from D2b; until D5b the allocator behind `DocMut` is a crate-private helper gated `#[cfg(test)]`, and D2b unit tests use it to check the increment and the exhaustion rule.

#### Scenario: Private allocator before DocMut (D2b, unit)
- GIVEN the `#[cfg(test)]` allocator with `next_id = 5`, and then with `next_id = u64::MAX`
- WHEN an id is allocated
- THEN the first call returns 5 and leaves `next_id` at 6, and the second returns `Err(IdSpaceExhausted)` leaving `next_id` unchanged

#### Scenario: Default document (unit)
- GIVEN `Document::default()`
- WHEN inspected through its accessors
- THEN `schema_version()` is 1, `next_id()` is 1, `dpi()` is 96 and `pages()` is empty

### Requirement: Dpi (D1)

`Dpi` MUST wrap a `u32` (pixels per inch), be `Copy`, `Eq`, `Ord`, `Hash`, and serialize as a bare integer (`96`). `Dpi::DEFAULT` and `Dpi::default()` MUST be 96. `MIN = 1` and `MAX = 10_000`. `Dpi::new(u32)` MUST NOT validate; `is_valid()` MUST be true exactly for `1..=10_000`. A `Dpi` outside the range MAY be built and carried; `validate()` rejects it (see `document-validation`). Deserialization of `Dpi` alone MUST accept any `u32` and MUST reject anything that is not an integer in `0..=u32::MAX`.

#### Scenario: Default (unit)
- GIVEN `Dpi::default()` and `Dpi::DEFAULT`
- WHEN read
- THEN both are 96 and `is_valid()` is true

#### Scenario: Range edges (unit)
- GIVEN the values 0, 1, 10 000, 10 001 and `u32::MAX`
- WHEN `is_valid()` is evaluated on `Dpi::new(v)`
- THEN the results are false, true, true, false, false, and `get()` returns each input

#### Scenario: Wire form (unit)
- GIVEN `Dpi::new(300)`
- WHEN serialized and deserialized
- THEN the JSON is `300` and it round-trips

#### Scenario: Non-integer wire values rejected (unit)
- GIVEN the JSON `-1`, `96.0`, `1e2`, `4294967296`, `null`, `"96"`
- WHEN deserialized as `Dpi`
- THEN each fails

### Requirement: PageSize and its pixel limits (D1)

`PageSize` MUST hold `width: Length` and `height: Length` (public fields, any `Unit`), `Copy`, with total construction. `PageSize::to_px(dpi) -> Result<gafira_geom::Size, ConversionError>` MUST convert each side with `Length::to_px(f64::from(dpi.get()))`; the crate MUST contain no unit math of its own. `PageSize::check(dpi) -> Result<(), PageSizeError>` MUST succeed only when, for each axis, the unrounded px value is finite and lies in `[PageSize::MIN_PX, PageSize::MAX_PX] = [1.0, 16384.0]`, both bounds inclusive, with no tolerance. `PageSizeError` MUST be `#[non_exhaustive]` with `Conversion { axis, error }` (conversion failed: non-finite value, overflow, invalid DPI) and `OutOfRange { axis, px }`, where `axis` is `Width` or `Height`; width is checked before height. It MUST implement `Display` and `Error`, and `Display` MUST name the axis.

#### Scenario: Bounds are inclusive (unit)
- GIVEN `1.0 px` and `16384.0 px` on both axes
- WHEN checked at DPI 96
- THEN `check` succeeds and `to_px` returns exactly 1.0 and 16384.0

#### Scenario: One step outside the bounds (unit)
- GIVEN `next_down(1.0)` px and `next_up(16384.0)` px as width, with a valid height, and then as height with a valid width
- WHEN checked
- THEN each fails with `OutOfRange` naming the axis used

#### Scenario: Bounds reached through a physical unit (unit)
- GIVEN `1 in` at DPI 1, `0.5 in` at DPI 1, `8192 in` at DPI 2 and `8192 in` at DPI 3
- WHEN checked as width
- THEN the first (1.0 px) and third (16384.0 px) succeed; the second (0.5 px) and fourth (24576 px) fail with `OutOfRange`

#### Scenario: A4 at 300 DPI (unit)
- GIVEN `210 mm` by `297 mm`
- WHEN `to_px` runs at DPI 300
- THEN the rounded values are 2480 and 3508 (FD-01a), the stored lengths are unchanged and the unrounded values are returned

#### Scenario: A0 follows the document DPI (unit)
- GIVEN A0 portrait, 841 mm by 1189 mm
- WHEN checked at DPI 300, 350 and 351
- THEN 300 passes (about 14043 px high), 350 passes (about 16383.86 px) and 351 fails with `OutOfRange` on `Height` (about 16430.7 px)

#### Scenario: Zero, negative and non-finite sides (unit)
- GIVEN widths `0.0 px`, `-0.0 px`, `-5.0 px`, NaN, `+inf`, `-inf`
- WHEN each is checked
- THEN the first three fail with `OutOfRange` and the non-finite ones with `Conversion`

#### Scenario: Overflowing conversion and invalid DPI (unit)
- GIVEN `1e308 in` at DPI 10 000, and `10 mm` at DPI 0
- WHEN checked
- THEN both fail with `Conversion` without panicking

#### Scenario: Unit round trip (property)
- GIVEN a finite value in each of `Unit::ALL` and a DPI in `1..=10 000` whose px result lies in 1 to 16384
- WHEN converted to px and back to the original unit
- THEN the relative error is below `1e-6` (FD-01d)

#### Scenario: Wire form (unit)
- GIVEN a page size of 210 x 297 mm
- WHEN serialized
- THEN the JSON is `{"width":{"value":210.0,"unit":"mm"},"height":{"value":297.0,"unit":"mm"}}`

### Requirement: BlendMode (D1)

`BlendMode` MUST be a closed, exhaustive (not `#[non_exhaustive]`) enum of the 16 W3C compositing modes: `Normal`, `Multiply`, `Screen`, `Overlay`, `Darken`, `Lighten`, `ColorDodge`, `ColorBurn`, `HardLight`, `SoftLight`, `Difference`, `Exclusion`, `Hue`, `Saturation`, `Color`, `Luminosity`. It MUST be `Copy`, `Eq`, `Hash`, default to `Normal`, expose `ALL: [BlendMode; 16]`, and serialize as the kebab-case CSS names (`normal`, `multiply`, ..., `color-dodge`, `color-burn`, `hard-light`, `soft-light`, ..., `luminosity`). Any other spelling MUST fail deserialization.

#### Scenario: Exhaustive set (unit)
- GIVEN a test matching every variant of `ALL` without a wildcard arm
- WHEN the tests compile
- THEN compilation succeeds and `ALL` has 16 distinct members

#### Scenario: Wire names (unit)
- GIVEN each variant of `ALL`
- WHEN serialized
- THEN the strings are exactly the 16 names above, in order, and each deserializes back

#### Scenario: Unknown or mis-cased names (unit)
- GIVEN `"Multiply"`, `"color_dodge"`, `"nope"`, `""`
- WHEN deserialized
- THEN each fails

#### Scenario: Default (unit)
- GIVEN `BlendMode::default()`
- WHEN read
- THEN it is `Normal`

### Requirement: Placement and to_transform (D2a)

`Placement` MUST hold public `x`, `y`, `w`, `h`, `rotation_deg`, `skew` (degrees) in `f64` and `flip: Flip { x: bool, y: bool }`. `Placement::new(x, y, w, h)` MUST set `rotation_deg = 0`, `skew = 0`, `flip` both false. Values MUST be stored unchanged and never normalized (a rotation of `720.0` stays `720.0`). Units are logical px in page space; the local box is `[0,w] x [0,h]`; `w` and `h` size the box and MUST NOT scale it. `to_transform() -> gafira_geom::Affine` MUST be total (never panic; non-finite fields give non-finite coefficients) and MUST equal, right to left, translate to the box center, rotate by `rotation_deg`, shear x by `tan(skew)` per unit of y, mirror per `flip`, translate by minus the half size. Rotation, skew and flip therefore pivot on the box center. Positive rotation turns `+x` toward `+y` (clockwise on a y-down page).

#### Scenario: Plain placement (unit)
- GIVEN `x=10, y=20, w=30, h=40`, no rotation, skew or flip
- WHEN `to_transform()` is applied to local `(0,0)` and `(30,40)`
- THEN the results are `(10,20)` and `(40,60)`

#### Scenario: Quarter turn clockwise (unit)
- GIVEN `x=0, y=0, w=2, h=2, rotation_deg=90`
- WHEN local `(2,1)` is transformed
- THEN the result is `(1,2)` within `1e-12`

#### Scenario: Flip (unit)
- GIVEN `x=10, y=0, w=10, h=10`
- WHEN local `(0,0)` is transformed with `flip.x`, and then with `flip.y`
- THEN the results are `(20,0)` and `(10,10)`

#### Scenario: Skew 45 degrees (unit)
- GIVEN `x=0, y=0, w=2, h=2, skew=45`
- WHEN local `(1,2)` is transformed
- THEN the result is `(2,2)` within `1e-12` (x sheared by the distance to the center line)

#### Scenario: Center is a fixed point (property)
- GIVEN any finite placement with any rotation, skew in `(-90, 90)` and flip
- WHEN the local center `(w/2, h/2)` is transformed
- THEN the result equals `(x + w/2, y + h/2)` within `1e-9` relative error

#### Scenario: Non-finite does not panic (unit)
- GIVEN a placement with NaN `x` and another with `+inf` `rotation_deg`
- WHEN `to_transform()` runs
- THEN it returns without panicking

#### Scenario: Values retained (unit)
- GIVEN `rotation_deg = 720.0` and `x = -0.0`
- WHEN stored and read
- THEN both are bit-identical to the inputs

### Requirement: ElementBase (D2a)

`ElementBase` MUST hold public `id: NodeId`, `name: Option<String>`, `placement: Placement`, `opacity: f32`, `blend: BlendMode`, `visible: bool`, `locked: bool`, all as bare values (no wrapper, no `Animatable`) so that a future untagged `Animatable::Static` stays wire-compatible. `ElementBase::new(id, placement)` MUST set `name = None`, `opacity = 1.0`, `blend = Normal`, `visible = true`, `locked = false`. Colors in the model MUST be `gafira_color::Color` bare values.

#### Scenario: Defaults (unit)
- GIVEN `ElementBase::new(id, placement)`
- WHEN inspected
- THEN `opacity` is bit-identical to `1.0`, `blend` is `Normal`, `visible` is true, `locked` is false, `name` is `None`

### Requirement: NodeKind (D2b)

`NodeKind` MUST be a closed, exhaustive enum with exactly `Group(GroupNode)`, `Shape(ShapeNode)` and `Extension(ExtensionNode)`. `GroupNode` MUST carry no data. `ShapeNode` MUST carry `geometry: ShapeGeometry` (`Rect` or `Ellipse`, exhaustive) and `fill: Option<Color>` (`None` by default; no stroke). `ExtensionNode` MUST carry `type_id: String`, `schema_version: u32` (the extension's own version, unrelated to the document's) and `payload: serde_json::Value`.

#### Scenario: Variant sets (unit)
- GIVEN tests matching on `NodeKind` and on `ShapeGeometry` without wildcard arms
- WHEN the tests compile
- THEN compilation succeeds (adding a variant breaks it deliberately)

#### Scenario: Shape without fill (unit)
- GIVEN a rectangle `ShapeNode` built without a fill
- WHEN read
- THEN its fill is `None`

### Requirement: Node, Page and the arena (D2b)

`Node` MUST hold public `base: ElementBase`, `kind: NodeKind` and `children: Vec<NodeId>` (back to front). `Page` MUST hold public `id: PageId`, `name: String`, `size: PageSize`, `locked: bool`, `hidden: bool`, `roots: Vec<NodeId>` (back to front) and `nodes: BTreeMap<NodeId, Arc<Node>>`, iterating in ascending `NodeId` order (proposed amendment to 02 §3.1, which says `HashMap`). `Page::new(id, name, size)` MUST give `locked = false`, `hidden = false`, no roots and no nodes; `Page::node(id)` MUST return the node of that page only. Ids in `roots` and `children` are resolved only inside the page's own arena. Changing `Page::locked` or `hidden` MUST NOT change any node's own `locked` or `visible`. The model MUST allow representing structural violations (tree rules are enforced by `validate()`).

#### Scenario: Page defaults (unit)
- GIVEN `Page::new(id, name, size)`
- WHEN inspected
- THEN `locked` and `hidden` are false, `roots` is empty and `nodes` is empty

#### Scenario: Deterministic iteration (unit)
- GIVEN nodes inserted with ids 10, 2, 7
- WHEN the arena is iterated
- THEN the order is 2, 7, 10

#### Scenario: Lookup stays inside the page (unit)
- GIVEN two pages, node 9 only in the second
- WHEN `Page::node(9)` is called on each
- THEN the first returns `None` and the second returns the node

#### Scenario: Page flags do not cascade (unit)
- GIVEN a page with a visible, unlocked node, then the page set `locked` and `hidden`
- WHEN the node is read
- THEN `visible` is still true and `locked` is still false

### Requirement: Document root (D2b; `from_parts` D4b)

`Document` MUST have private fields `schema_version: u32` (constant `SCHEMA_VERSION = 1`), `next_id: u64`, `dpi: Dpi` and `pages: Vec<Arc<Page>>` (order is page order). It MUST be `Clone`, `Debug`, `PartialEq`, with read accessors `schema_version()`, `next_id()`, `dpi()`, `pages()` and `page(PageId) -> Option<&Page>`. `Document::from_parts(next_id, dpi, pages) -> Result<Document, ValidationError>` MUST build a document and run `validate()`, so it never returns an invalid document. No public API reachable from `&Document` or `Arc<Document>` MAY return `&mut` to its contents or write a field (see `edit-capability`).

#### Scenario: Accessors and page order (D4b, unit)
- GIVEN a document built with pages 3, 1, 2 through `from_parts`
- WHEN `pages()` is read and `page(PageId::new(1))` is called
- THEN the order is 3, 1, 2 and the lookup returns page 1; an unknown id returns `None`

#### Scenario: from_parts validates (D4b, unit)
- GIVEN `next_id = 1` with a page of id 5
- WHEN `from_parts` runs
- THEN it returns `Err(ValidationError::NextIdNotAboveMax { .. })`

#### Scenario: Clone is independent (D5b, unit)
- GIVEN a document and its clone
- WHEN the original is edited through the capability to add a page
- THEN the clone still has its previous page count

### Requirement: Structural sharing (D2b; the edit scenario D5b)

Pages MUST be shared by `Arc<Page>` and nodes by `Arc<Node>`. Cloning a `Document` MUST NOT copy any `Page` or `Node`; cloning a `Page` MUST NOT copy any `Node`. After an edit that changes one node, every other node and every other page MUST stay pointer-identical (`Arc::ptr_eq`) to the same item in a prior clone.

#### Scenario: Clone shares everything (D2b, unit, in-crate helper)
- GIVEN a document built with the in-crate test helper with 2 pages and 3 nodes in the first, and its clone
- WHEN pages and nodes are compared with `Arc::ptr_eq`
- THEN all are pointer-equal

#### Scenario: Edit touches one path (D5b, unit)
- GIVEN a document, its clone, and one node changed through `DocMut::node_mut`
- WHEN compared with `Arc::ptr_eq`
- THEN the edited node and its page differ, while the untouched nodes of that page and the other page are still pointer-equal

### Requirement: Value policy and equality (D2a, D2b)

Constructors MUST be infallible, MUST NOT clamp, normalize or validate, and `Dpi::new` is included. `PartialEq` on model types MUST be IEEE (`NaN != NaN`, `-0.0 == 0.0`). Tests that claim identity of stored numbers MUST compare `to_bits()`.

#### Scenario: Out-of-range values are representable (unit)
- GIVEN an `ElementBase` with `opacity = 1.5`, a `Placement` with NaN `x`, and `Dpi::new(0)`
- WHEN built
- THEN construction succeeds and the values read back bit-identical

#### Scenario: IEEE equality (unit)
- GIVEN two placements equal except `-0.0` versus `0.0` in `x`, and a node with a NaN placement
- WHEN compared with `==` and by `to_bits()`
- THEN the zeros compare equal while their bits differ, and the NaN node is not equal to itself

### Requirement: Generator and model properties (D6a generator and guards; D6b properties)

A bounded proptest generator of whole documents MUST exist as dev-only test code in `crates/gafira-doc/tests/common`, built through `DocMut` with an `EditToken`. Bounds: 1 to 3 pages (plus a dedicated empty-document case), 0 to 12 nodes per page, tree depth at most 4, only groups with children, ids allocated from the counter in shuffled order with an extra `next_id` gap of 0 to 3, extension payloads of depth at most 3 with at most 4 entries per container and strings of at most 8 characters including non-ASCII, quote, backslash and control characters. It MUST generate valid documents only. Guards using a deterministic runner (20 000 draws) MUST show that it reaches: `-0.0`, subnormals, px exactly 1.0 and 16384.0, opacity 0.0, `-0.0` and 1.0, depth at least 3, an `Extension` node, a group with children, several pages, an empty page, and every corruption variant of `document-validation`.

#### Scenario: Generated documents are valid (property)
- GIVEN generated documents (default 256 cases)
- WHEN `validate()` runs on each
- THEN every one passes

#### Scenario: Generator coverage guard (unit)
- GIVEN the deterministic 20 000-draw guard run
- WHEN it counts reached features
- THEN each feature in the list above is reached at least once
