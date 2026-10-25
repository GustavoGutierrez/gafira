# Validation passes

## Contract

`Document::validate(&self) -> Result<(), ValidationError>` returns the first rule the document breaks, or `Ok(())`. Nothing is altered or clamped. The result is deterministic: the same document always yields the same error.

Passes run in a fixed order, P1 to P6. An earlier pass always wins over a later one, and inside a pass the checks run in the order of the `ValidationError` variants. In P1 to P3 each check runs over all pages (in page order) before the next check; P4 goes node by node in ascending id order.

`ValidationError` is `#[non_exhaustive]`, so a downstream `match` needs a wildcard arm (a compile-fail doctest guards this). Each variant carries the ids needed to locate the fault.

## Passes

| Pass | Checks, in order | Errors |
|---|---|---|
| P1 Header | Schema version equals `Document::SCHEMA_VERSION`; DPI within `Dpi::MIN..=Dpi::MAX` | `UnsupportedSchemaVersion`, `InvalidDpi` |
| P2 Ids | Page and node ids are unique across the whole document; each arena key equals the id of the node under it; `next_id` is above the largest id in use | `DuplicateId`, `ArenaKeyMismatch`, `NextIdNotAboveMax` |
| P3 Pages | Each page side is within the pixel limits at the document DPI (width first); every root exists in its page; no root is repeated | `PageSize`, `DanglingRoot`, `DuplicateRoot` |
| P4 Nodes | Numeric fields are finite, then in range; extension `type_id` is not empty; extension payload depth is at most 64; only groups have children; every child exists in the page | `NonFinite`, `OutOfRange`, `EmptyExtensionTypeId`, `ExtensionTooDeep`, `ChildrenOnLeaf`, `DanglingChild` |
| P5 Parents | No node is the child of more than one parent; no root is also a child; no node is neither a root nor a child | `MultipleParents`, `RootHasParent`, `Orphan` |
| P6 Reachability | Depth-first search from the roots; the lowest unreached id is reported as in or below a cycle | `Cycle` |

## Numeric ranges in P4

| Field | Rule |
|---|---|
| `placement.x`, `placement.y`, `placement.rotation_deg` | Any finite value |
| `placement.w`, `placement.h` | At least 0 |
| `placement.skew` | Strictly between -90 and 90 degrees |
| `opacity` | 0 to 1, inclusive |
| Shape fill red, green and blue | Any finite value (wide gamut and HDR values pass) |
| Shape fill alpha | 0 to 1, inclusive |

`NonFinite` is reported for any field before any `OutOfRange`. The field is named by `NodeField` (`X`, `Y`, `W`, `H`, `RotationDeg`, `Skew`, `Opacity`, `FillChannel`, `FillAlpha`).

## Robustness

- P6 and the payload depth measure are iterative, so a deep tree or a hostile payload cannot overflow the stack.
- The pixel limit follows the document DPI: changing the DPI can make a previously valid page invalid, which is why commands that change the DPI must re-validate.
- A cycle is representable in memory (the model does not forbid it) but never survives `validate`, and therefore never survives load.

## Where validation runs

| Place | State |
|---|---|
| `Document::from_parts` and `Document::from_json` / `Deserialize` | Implemented: invalid documents cannot be built through these paths |
| After every command (`debug_assert!(doc.validate().is_ok())`, ARQ-13) | Implemented in `History` (debug builds); commands check their own inputs in all builds |

## Property tests

`crates/gafira-doc/tests/properties.rs` generates valid documents (all must validate), applies targeted corruptions (each must be detected as the exact expected variant) and checks that corruption survives a JSON round trip. Coverage guards ensure the generators reach every variant.
