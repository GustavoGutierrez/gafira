# Document Validation Specification

## Purpose

`Document::validate()` and `ValidationError`: the structural part of ARQ-13 for `gafira-doc`. It runs on every deserialization (see `document-serde`, validating load) and in `Document::from_parts`, and is the check that C3 runs in `debug_assert` after each command. New capability; no existing spec. Not covered: ordered keyframes and `TimeRange` (no such types yet), the ARQ-13 fuzzer in `gafira-format`, and `gafira-cli validate` (deferred to the `gafira-format` change, design A15, holder confirmation pending).

Scenario type tags: (unit), (property), (integration), (command), (manual). "Dn": delivery per the design table (D4a header, ids, pages and the error type; D4b node values and validating load; D5a parents and reachability; D6a1 generator; D6a2 corruptions; D6b properties).

## Decisions followed from the design (still open for the holder)

| # | Decision | Design ref |
|---|---|---|
| V1 | Out-of-range or non-finite values are REJECTED, never clamped (Q2). | A9 |
| V2 | RGB channels of a fill need only be finite (extended range legal, as in `gafira-color`); fill alpha MUST be in `[0,1]`. | A9 |
| V3 | `w`, `h` `>= 0`; `-90 < skew < 90` (exclusive); `rotation_deg` any finite value; `Extension.type_id` not empty; `-0.0` accepted wherever `0.0` is. | A9 |
| V3b | An extension payload nested deeper than 64 levels is rejected (`ExtensionTooDeep`). Depth counts nested arrays and objects; a scalar has depth 0, `[]` has depth 1. Reason: a document that saves must reopen, and `serde_json` cannot reload a full document whose payload reaches about 122 levels. | Corrected design |
| V4 | `validate()` returns the FIRST error in a fixed pass order P1 to P6; it does not return a list. | A10 |

## Requirements

### Requirement: validate and ValidationError (D4a; node rows D4b)

`Document::validate(&self) -> Result<(), ValidationError>` MUST return `Ok(())` for a document satisfying every rule below and the first error otherwise. It MUST be pure and deterministic, MUST NOT panic and MUST NOT recurse (deep chains, cycles, large counts). `ValidationError` MUST be hand-written (no `thiserror`), `#[non_exhaustive]`, `Clone + Debug + PartialEq` (no `Eq`), implement `Display` and `Error`, and have one distinct variant per rule: `UnsupportedSchemaVersion`, `InvalidDpi`, `DuplicateId`, `ArenaKeyMismatch`, `NextIdNotAboveMax`, `PageSize`, `DanglingRoot`, `DuplicateRoot`, `NonFinite`, `OutOfRange`, `EmptyExtensionTypeId`, `ExtensionTooDeep`, `ChildrenOnLeaf`, `DanglingChild`, `MultipleParents`, `RootHasParent`, `Orphan`, `Cycle` (18 variants). `Display` MUST name the rule and the offending ids or field. For a document with exactly one violation the variant MUST be the one stated per rule; the table-driven test MUST have one row per variant, all distinct.

#### Scenario: Valid documents (unit)
- GIVEN `Document::default()` and a document with a page, a group with two children (a rectangle and an extension)
- WHEN `validate()` runs
- THEN both return `Ok(())`

#### Scenario: Each rule has its own variant (unit, table)
- GIVEN one minimal in-crate fixture per variant above (18 rows; rows 1 to 8 in D4a, rows 9 to 14 in D4b, rows 15 to 18 added in D5a)
- WHEN `validate()` runs on each
- THEN each returns `Err` with its own variant, and no two rows give the same variant

#### Scenario: Error contract (unit)
- GIVEN any `ValidationError`
- WHEN used as `&dyn std::error::Error` and formatted
- THEN it compiles and the text names the rule and the id or field; `cargo tree -p gafira-doc` lists no `thiserror`

#### Scenario: Determinism (D6b, property)
- GIVEN a corrupted document
- WHEN `validate()` runs twice
- THEN both results are equal

### Requirement: Pass order, first error wins (D4a; P4 D4b)

Passes MUST run in this order, pages in `pages` order and nodes in ascending id order: P1 header (schema version, DPI); P2 ids (uniqueness, arena key, `next_id`); P3 pages (page size, roots exist, no root listed twice); P4 nodes (finiteness, ranges, extension `type_id`, then extension payload depth (`ExtensionTooDeep`), then children only on groups, children exist in the same arena). Variant positions follow the design in pass order: 1 `UnsupportedSchemaVersion`, 2 `InvalidDpi` (P1); 3 `DuplicateId`, 4 `ArenaKeyMismatch`, 5 `NextIdNotAboveMax` (P2); 6 `PageSize`, 7 `DanglingRoot`, 8 `DuplicateRoot` (P3); 9 `NonFinite`, 10 `OutOfRange`, 11 `EmptyExtensionTypeId`, 12 `ExtensionTooDeep`, 13 `ChildrenOnLeaf`, 14 `DanglingChild` (P4); 15 `MultipleParents`, 16 `RootHasParent`, 17 `Orphan` (P5); 18 `Cycle` (P6). Delivery: the 18 variants and P1 to P3 land in D4a, P4 in D4b; P5 and P6 land in D5a, so after D4a `validate()` runs P1 to P3 only and between D4b and D5a it runs P1 to P4 only and still accepts cycles and multiple parents; P5 parents (parent edge counts); P6 reachability (iterative depth-first search from the roots). When a document has several defects, `validate()` MUST report the one from the earliest pass, and within a pass the first in the stated order. In P1 to P3 each check runs over all pages before the next check (all duplicate ids, then all key mismatches, then `next_id`; all page sizes, then all dangling roots, then all repeated roots), and in P5 each check (`MultipleParents`, then `RootHasParent`, then `Orphan`) runs over all pages before the next check, so a later check on page 1 never hides an earlier check on page 2. P4 is ordered node by node: nodes in ascending id and, for each node, its checks in the listed order (9 to 14) before the next node is examined, so a defect of a lower-id node wins over any defect of a higher-id node, and for one node `NonFinite` wins over `OutOfRange`, and so on down the list.

#### Scenario: Earlier pass wins (D4a for the first two cases, D4b for the third, unit, table)
- GIVEN a document with `schema_version = 2` and a duplicate id, one with a duplicate id and a page size out of range, and one with an out-of-range opacity and a dangling child in a node with a higher id (both are P4 defects; the lower id is reported first)
- WHEN validated
- THEN the results are `UnsupportedSchemaVersion`, `DuplicateId` and `OutOfRange` (opacity) respectively

#### Scenario: Earlier rule wins inside a pass (D4a, unit, table)
- GIVEN `schema_version = 2` with DPI 0; a duplicate id with `next_id` 0; page 1 with a dangling root and page 2 with a page size out of range
- WHEN validated
- THEN the results are `UnsupportedSchemaVersion`, `DuplicateId` and `PageSize` on page 2 respectively

#### Scenario: Earlier pass wins over parents and cycles (D5a, unit, table)
- GIVEN a document with an out-of-range opacity and an orphan, one with an orphan and a cycle elsewhere, and one with a multiple-parent node and a cycle
- WHEN validated
- THEN the results are `OutOfRange`, `Orphan` and `MultipleParents` respectively

#### Scenario: D4b build accepts cycles (D4b, unit)
- GIVEN the state after D4b (P1 to P4 only) and a document with a self-parenting group
- WHEN validated
- THEN it still passes; the D5a tests close this gap and the scenario is inverted when P5 and P6 land: a self-parenting group that is not a root then fails with `Cycle` (as a root it would fail with `RootHasParent`)

#### Scenario: Ascending node order within a pass (D4b, unit)
- GIVEN nodes 3 and 8 each with a NaN `x`
- WHEN validated
- THEN the error names node 3

### Requirement: Header: schema version and DPI (D4a)

P1 MUST reject `schema_version != 1` with `UnsupportedSchemaVersion { found }` and a `Dpi` outside `1..=10 000` with `InvalidDpi { dpi }`.

#### Scenario: Versions (unit)
- GIVEN `schema_version` 0, 2 and `u32::MAX`, and 1
- WHEN validated
- THEN the first three fail carrying the found value, and 1 passes

#### Scenario: DPI edges (unit)
- GIVEN DPI 0, 1, 10 000, 10 001 and `u32::MAX`
- WHEN validated
- THEN 0, 10 001 and `u32::MAX` fail with `InvalidDpi` carrying the value, and 1 and 10 000 pass

### Requirement: Unique ids and next_id (D4a)

P2 MUST require all page ids and all node ids to be distinct across the whole document (`DuplicateId { id }`: two pages with the same id, the same node id in two pages, or a page id equal to a node id), the arena key of every entry to equal its `base.id` (`ArenaKeyMismatch { page, key, id }`), and `next_id` to be strictly greater than the maximum raw id (`NextIdNotAboveMax { next_id, max_id }`). A document with no ids MUST accept any `next_id`. A node or page with id `u64::MAX` MUST be rejected, because no `next_id` can exceed it. A duplicate id inside one page cannot reach `validate()` from JSON (parse time rejection in `document-serde`); `validate()` does not need to handle it.

#### Scenario: Duplicate ids (unit, table)
- GIVEN two pages with id 1; two pages whose arenas both hold id 9; page id 4 with a node id 4
- WHEN validated
- THEN each fails with `DuplicateId` naming 1, 9 and 4

#### Scenario: Key differs from base id (unit)
- GIVEN an in-memory arena entry with key 5 whose node has `base.id` 6
- WHEN validated
- THEN it fails with `ArenaKeyMismatch` naming the page, 5 and 6

#### Scenario: next_id bound (unit)
- GIVEN max id 10 with `next_id` 11, 10, 5 and 0, and an empty document with `next_id` 0
- WHEN validated
- THEN 11 and the empty document pass, and 10, 5 and 0 fail with `NextIdNotAboveMax` carrying `next_id` and `max_id`

#### Scenario: Id u64::MAX (unit)
- GIVEN a node with id `u64::MAX` and `next_id = u64::MAX`
- WHEN validated
- THEN it fails with `NextIdNotAboveMax`

### Requirement: Page size and roots (D4a)

P3 MUST call `PageSize::check(dpi)` for each page (`PageSize { page, error }` carrying the `PageSizeError`, so width is reported before height), MUST require each id in `roots` to exist in that page's own arena (`DanglingRoot`) and MUST reject an id listed twice in `roots` (`DuplicateRoot`). The size limits are those of `document-model`: each axis in unrounded px at the document DPI inside `[1.0, 16384.0]`, inclusive.

#### Scenario: Size edges (unit)
- GIVEN pages of `1.0 px`, `16384.0 px`, `next_down(1.0) px` and `next_up(16384.0) px`
- WHEN validated
- THEN the first two pass and the last two fail with `PageSize` carrying `OutOfRange`

#### Scenario: Limit follows the document DPI (unit)
- GIVEN an A0 portrait page (841 x 1189 mm)
- WHEN validated at DPI 300, 350 and 351
- THEN 300 and 350 pass and 351 fails with `PageSize` on `Height`

#### Scenario: Changing the DPI can invalidate a page (D5b, unit)
- GIVEN a valid document with an A0 page at DPI 300, then `set_dpi(351)` through the capability
- WHEN validated
- THEN it fails with `PageSize`

#### Scenario: Non-finite or zero size (unit)
- GIVEN a width of NaN, `+inf`, `0.0` and `-5.0`
- WHEN validated
- THEN each fails with `PageSize` (`Conversion` for non-finite, `OutOfRange` otherwise)

#### Scenario: Dangling and duplicate roots (unit)
- GIVEN `roots = [42]` with no node 42, `roots = [3, 3]` with node 3 present, and a root whose node exists only in another page
- WHEN validated
- THEN the results are `DanglingRoot` naming 42, `DuplicateRoot` naming 3 and `DanglingRoot`

### Requirement: Node values (D4b)

P4 MUST reject, per node and with the node and field named (`NodeField`: `X`, `Y`, `W`, `H`, `RotationDeg`, `Skew`, `Opacity`, `FillChannel`, `FillAlpha`): a non-finite value as `NonFinite { node, field }`; and as `OutOfRange { node, field, value }`: `w < 0`, `h < 0`, `skew <= -90` or `skew >= 90`, `opacity` outside `[0,1]`, fill alpha outside `[0,1]`. RGB channels outside `[0,1]` MUST be accepted if finite. `rotation_deg` MUST accept any finite value. An empty `Extension.type_id` MUST fail with `EmptyExtensionTypeId`. An `Extension.payload` nested deeper than 64 levels MUST fail with `ExtensionTooDeep { node, depth }`; exactly 64 MUST pass. The depth walk MUST be iterative. `-0.0` and subnormals MUST be accepted where `0.0` is. A non-empty `children` on a `Shape` or `Extension` MUST fail with `ChildrenOnLeaf`. Every child MUST exist in the same page's arena (`DanglingChild { parent, child }`; a node that lives only in another page is dangling). `validate()` MUST NOT alter or clamp any value (it takes `&self`).

#### Scenario: Opacity edges (unit)
- GIVEN opacity `0.0`, `-0.0`, `1.0`, `1e-45`
- WHEN validated
- THEN all pass

#### Scenario: Opacity out of range (unit)
- GIVEN opacity `-0.0000001`, `1.0000001`, `2.0`, NaN, `+inf`, `-inf`
- WHEN validated
- THEN the first three fail with `OutOfRange` on `Opacity` and the last three with `NonFinite` on `Opacity`

#### Scenario: Placement values (unit, table)
- GIVEN `x` NaN, `y` `+inf`, `rotation_deg` `-inf`, `skew` NaN, `w` `-1.0`, `h` `-1e-300`, `skew` `90.0`, `skew` `-90.0`
- WHEN validated
- THEN the results are `NonFinite` (X, Y, RotationDeg, Skew), `OutOfRange` (W, H) and `OutOfRange` (Skew) twice

#### Scenario: Placement values accepted (unit)
- GIVEN `w = -0.0`, `w = 0.0`, `rotation_deg = 720.0`, `skew = 89.999` and `skew = -89.999`
- WHEN validated
- THEN all pass

#### Scenario: Fill values (unit)
- GIVEN a fill with a NaN red channel, one with alpha NaN, one with alpha 1.5, and one with `r = -0.5`, `g = 1.75`, alpha 0.5
- WHEN validated
- THEN the first fails with `NonFinite` (`FillChannel`), the second with `NonFinite` (`FillAlpha`), the third with `OutOfRange` (`FillAlpha`), and the last passes

#### Scenario: Extension type_id (unit)
- GIVEN an `Extension` with `type_id = ""` and another with `type_id = "x"`
- WHEN validated
- THEN the first fails with `EmptyExtensionTypeId`, the second passes

#### Scenario: Payload depth at the limit (unit)
- GIVEN extension payloads of 64 nested arrays, 65 nested arrays, 65 nested objects, a scalar, and `[]`
- WHEN validated
- THEN 64, the scalar and `[]` pass and 65 (arrays or objects) fails with `ExtensionTooDeep` naming the node

#### Scenario: Very deep payload does not overflow (unit)
- GIVEN a payload built 1 000 levels deep in code (not parsed; kept modest because dropping a `Value` is recursive)
- WHEN validated
- THEN validation returns `ExtensionTooDeep` and the depth walk is iterative (a mutation to a recursive walk is recorded, not exercised at extreme depth)

#### Scenario: Children on a leaf and dangling child (unit)
- GIVEN a `Shape` and an `Extension`, each with `children = [2]`; and a group listing child 99 absent from the arena; and a group in page A listing a node that lives only in page B
- WHEN validated
- THEN the results are `ChildrenOnLeaf` (twice), `DanglingChild` naming parent and 99, and `DanglingChild`

#### Scenario: Not clamped (unit)
- GIVEN a document with `opacity = 1.5`
- WHEN validated and then read
- THEN it failed and `opacity` is still bit-identical to `1.5`

### Requirement: Parents and reachability (D5a)

P5 MUST count parent edges over all `children` lists of a page, where a root counts as having none: more than one edge to a node (including the same group listing it twice) fails with `MultipleParents { node }`; a root that appears in any `children` fails with `RootHasParent { node, parent }`; a node that is not a root and has no parent fails with `Orphan { node }`. P6 MUST run an iterative depth-first search from the roots and report any node not reached as `Cycle { node }`, naming the lowest-id such node. After P5 every unreached node has exactly one parent, so it lies in or below a cycle: on the ring itself (including a self-parent) or in a subtree hanging off a ring that no root reaches; `Cycle { node }` MAY therefore name a node outside the ring. Both passes MUST work without recursion.

#### Scenario: Second parent (unit)
- GIVEN node C listed as a child of groups A and B, and separately C listed twice by one group
- WHEN validated
- THEN both fail with `MultipleParents` naming C

#### Scenario: Root with a parent (unit)
- GIVEN node C in `roots` and in A's `children`
- WHEN validated
- THEN it fails with `RootHasParent` naming C and A

#### Scenario: Orphan (unit)
- GIVEN a node in the arena that is not a root and in nobody's `children`
- WHEN validated
- THEN it fails with `Orphan` naming it

#### Scenario: Cycles (unit, table)
- GIVEN a group listing itself (not a root, so it is its own only parent), groups A -> B -> C -> A with none of them a root, and the same ring with an extra group D listed only as a child of C (D hangs below the ring)
- WHEN validated
- THEN the first two fail with `Cycle` naming a node of the ring, and the third with `Cycle` naming the lowest-id unreached node, which may be D

A ring reachable from a root is not a separate row: its entry node would have two parents (`MultipleParents`) or be a root with a parent (`RootHasParent`), so P5 reports it before P6 runs.

#### Scenario: Deep chain without overflow (D5a, unit)
- GIVEN a valid chain of 10 000 nested groups
- WHEN validated
- THEN it passes and the call does not overflow the stack

### Requirement: Validation properties (D6a2 corruption generators and guard; D6b properties)

Generated valid documents MUST validate. A corruption step MUST exist for each of: duplicate id, key mismatch, `next_id <= max id`, page size below or above the limit, dangling root, duplicate root, non-finite field, out-of-range opacity, fill alpha, skew, `w` or `h`, empty `type_id`, extension payload deeper than 64, children on a leaf, dangling child, multiple parents, root with a parent, orphan, cycle, bad DPI, `schema_version = 2`. Applying one to a generated document MUST make `validate()` return exactly the matching variant. For every corruption except `ArenaKeyMismatch` and `NonFinite` (the wire-visible corruptions: `to_json` writes the corrupted value and a reader reads the same value back), `from_json(to_json(d))` MUST return `Err(LoadError::Invalid(same variant))`. `ArenaKeyMismatch` is excluded because the arena is written as the node array only, so the key is not on the wire and a reload derives it from `base.id` and succeeds. `NonFinite` is excluded because `to_json` writes `null` and the reload fails with `LoadError::Json`, never `Invalid(NonFinite)`. Both keep the `validate()` clause. A duplicate id is generated across two pages (or as a page id equal to a node id), because an in-page duplicate cannot exist in a `BTreeMap`. These are properties Q3 (corruptions) and Q2 (valid documents) of the design, named Q so they do not clash with passes P1 to P6.

#### Scenario: Valid generated documents (property)
- GIVEN generated documents (default 256 cases)
- WHEN validated
- THEN all return `Ok(())`

#### Scenario: Corruption is detected exactly (property)
- GIVEN a generated document and one applicable corruption chosen at random
- WHEN validated
- THEN the result is `Err` with exactly the matching variant

#### Scenario: Corruption survives the wire (property)
- GIVEN a corrupted document whose corruption is wire-visible (every corruption except `ArenaKeyMismatch` and `NonFinite`)
- WHEN `from_json(to_json(d))` runs
- THEN it returns `Err(LoadError::Invalid(same variant))`

#### Scenario: Coverage guards and mutations (unit)
- GIVEN the corruption list, the deterministic guard, and the planned mutations (remove P6, count parents across pages instead of per page's arena, clamp opacity on read)
- WHEN the guards and mutated builds run
- THEN every corruption variant is reached by the guard, and each mutation makes the property fail (an undetected mutation is recorded with its reason)
