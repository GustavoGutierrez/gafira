# Crate gafira-doc

## Purpose

Layer-1 document model: `Document -> Page -> Node`. Pure domain code: no I/O, no `unsafe`. It depends only on `gafira-geom` and `gafira-color`; geometry types are always imported through `gafira_geom`. Externally it uses `serde` (feature `rc`) and `serde_json`.

Spec 02 describes a larger model (`Project`, sequences, tracks, clips, styles, assets, animation). Only the subset below exists; the rest is **Planned**.

## Public API

| Item | Description |
|---|---|
| `Document` | Root: `SCHEMA_VERSION` (1), `schema_version`, `next_id`, `dpi`, `pages`, `page(id)`, `from_parts`, `validate`, `from_json`, `to_json`, `edit`; `Default` is an empty document with `next_id` 1 and 96 DPI |
| `Page` | `id`, `name`, `size`, `locked`, `hidden`, `roots`, `nodes` (a `BTreeMap<NodeId, Arc<Node>>`); `Page::new`, `node(id)` |
| `Node` | `base: ElementBase`, `kind: NodeKind`, `children: Vec<NodeId>`; `Node::new` |
| `NodeKind` | Closed enum: `Group(GroupNode)`, `Shape(ShapeNode)`, `Extension(ExtensionNode)` |
| `ShapeNode`, `ShapeGeometry` | Shape with `geometry` (`Rect` or `Ellipse`) and an optional `fill: Option<Color>` |
| `ExtensionNode` | Opaque data of a plugin or future feature: `type_id`, `schema_version`, `payload` (`serde_json::Value`); `MAX_EXTENSION_DEPTH` is 64 |
| `ElementBase` | Shared header: `id`, `name`, `placement`, `opacity`, `blend`, `visible`, `locked`; `ElementBase::new` |
| `Placement`, `Flip` | Position and orientation in logical px: `x`, `y`, `w`, `h`, `rotation_deg`, `skew`, `flip`; `to_transform()` returns an `Affine` |
| `BlendMode` | The 16 W3C compositing modes; `BlendMode::ALL` |
| `Dpi` | Pixels per inch: `DEFAULT` 96, `MIN` 1, `MAX` 10 000; `new`, `get`, `is_valid` |
| `PageSize`, `Axis`, `PageSizeError` | Page dimensions as two `Length` values; `MIN_PX` 1.0, `MAX_PX` 16 384.0; `to_px(dpi)`, `check(dpi)` |
| `NodeId`, `PageId` | Distinct `u64` newtypes with `new`, `get`, `Display` |
| `IdSpaceExhausted` | Error when the id counter would pass `u64::MAX` |
| `ValidationError`, `NodeField` | Result of `validate` (see the validation chapter) |
| `LoadError` | `Json(serde_json::Error)` or `Invalid(ValidationError)` (non-exhaustive) |
| `EditToken`, `DocMut` | The edit capability (below) |
| `msg::MessageKey`, `msg::CommandLabel` | Neutral message key `MessageKey(&'static str)`; `CommandLabel` is an alias. No language and no translated text; never serialized into a document |

## Structure and sharing

A `Document` holds its pages as `Arc<Page>`, and each page holds its nodes as `Arc<Node>` in a `BTreeMap`. Cloning a document copies no page or node. A mutation copies on write only along the path it touches (`Arc::make_mut` on the page and on the node). This is the structural sharing that history snapshots rely on.

Ids come from one counter, `next_id`, shared by pages and nodes. Ids in `roots` and in `children` resolve only inside their own page's arena.

## Invariants of the model types

- Construction never validates. Out-of-range values (for example an opacity of 2.0 or a DPI of 0) are representable; `Document::validate` is the check, and `Document::from_parts` and loading call it.
- Equality is IEEE and structural: a NaN never equals itself.
- Page flags (`locked`, `hidden`) do not cascade into node flags.
- `NodeKind` is closed on purpose; adding a kind makes the compiler flag every `match`.
- Iteration order is deterministic: the arena is a `BTreeMap`, and `HashMap` is banned by `clippy.toml`.

## Placement geometry

The local box of an element is `[0,w] x [0,h]`; `w` and `h` size the box and never scale it. `Placement::to_transform` maps local coordinates to page space, applying right to left: translate by minus half the size, mirror, shear x by `tan(skew)`, rotate clockwise (y-down page), translate to the box center. The function is total: non-finite fields give non-finite coefficients.

## Edit capability

`Document` has private fields and exposes no `&mut` access. Mutation needs an `EditToken` and goes through `Document::edit(&token)`, which returns a `DocMut`:

| `DocMut` method | Effect |
|---|---|
| `alloc_node_id`, `alloc_page_id` | Take an id from the shared counter; `IdSpaceExhausted` at `u64::MAX`, with the counter unchanged |
| `restore_next_id` | Lower the counter back to a value read before the allocations being undone; a value at or above the counter is ignored, so the counter never moves up and ids stay unique in every state |
| `set_dpi` | Set the DPI (not validated) |
| `pages_mut` | The page list, to add, remove or reorder pages |
| `page_mut`, `node_mut` | Copy-on-write access along one path |
| `as_doc` | Read access while editing |

`EditToken` has a private field and implements none of `Clone`, `Copy`, `Default` or `Debug`, so it cannot be duplicated or built by a literal. Its constructor is `#[doc(hidden)]`. Only the editing crates may name it; CI step V13 enforces that. Nothing is validated implicitly: after any sequence of `DocMut` operations the document may be invalid. Replacing a whole value (`*doc = other`) by whoever owns the document is possible and is documented, not prevented. The compile-fail doctests in `edit.rs` are regression aids; the `trybuild` compile-fail fixtures for ARQ-30 live in `gafira-edit` (see the edit chapter).

## Tests

Integration tests are in `crates/gafira-doc/tests/`: `serde.rs`, `golden.rs`, `properties.rs`, `edit.rs`, with the shared generators taken from `gafira-testkit` (a dev-dependency). Golden JSON files are in `tests/golden/`.
