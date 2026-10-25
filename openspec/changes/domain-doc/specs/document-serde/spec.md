# Document Serde Specification

## Purpose

The JSON wire form of the `gafira-doc` model: deterministic bytes (ARQ-10), forward compatibility (02 §3.5), `Extension` payload survival (ARQ-11, reformulated), a load path that can never yield an invalid `Document` (ARQ-13) and safe handling of hostile or corrupt input. New capability; no existing spec. Container layout, files, compression and I/O belong to `gafira-format` and are out of scope; this spec covers bytes or strings to value and back only. Model types are in `document-model`, invariants in `document-validation`.

Scenario type tags: (unit), (property), (integration), (command), (manual). "Dn": delivery per the design table. D3 lands the wire format with an unchecked `from = "DocumentWire"` conversion; D4 switches it to `try_from` with `validate()`. Requirements marked (D4) depend on that switch. There are seven deliveries (D1, D2a, D2b, D3, D4, D5, D6). In D3 no public way to build a `Document` exists yet (`from_parts` is D4, `DocMut` is D5), so every D3 scenario that needs a document value (exact bytes, arena order, node shapes, `None` as `null`, bit identity, build-order independence, non-finite values, "built in code") is a UNIT test inside the crate using the `#[cfg(test)]` helper `test_document` (added in D2b with its first test) that fills the private fields. The integration tests under `tests/` in D3 use JSON fixtures and goldens loaded WITHOUT validation (through the unchecked D3 conversion). The idempotent round-trip property over generated documents is D6. **Fixture rule**: every JSON fixture in D3 MUST be a valid document, so it keeps passing after D4 (`try_from`) and D5 (P5 and P6): every node in a fixture's `nodes` array is listed in `roots` or in exactly one group's `children`, and no fixture has a cycle or a second parent. Fixtures that omit defaultable fields still list their nodes in `roots`.

## Decisions followed from the design (still open for the holder)

| # | Decision | Design ref |
|---|---|---|
| S1 | ARQ-11 is reformulated: byte for byte for canonical payloads, by value for any other (`serde_json::Value`; `RawValue` rejected). Q3. | A4 |
| S2 | Canonical output is compact JSON (`to_json`), no trailing newline. | A13, Wire Format |
| S3 | Every field is always written; `None` is written as `null`; no `skip_serializing_if`. | Wire Format |
| S4 | Unknown fields are ignored on read and dropped on save; only `Extension.payload` round-trips unknown content. An unknown core `type` is rejected. | A14, Wire Format |
| S5 | The node arena is a JSON array of nodes in ascending id order, not an object keyed by id; a duplicate id inside one array is rejected at parse time as `LoadError::Json`. | A1 |
| S6 | `schema_version != 1` is rejected by validation (`LoadError::Invalid`). Read-only opening of newer schemas is `gafira-format`'s concern. | Wire Format |

## Requirements

### Requirement: Wire shape and key order (D3)

Serialization MUST write keys in declaration order and MUST write every field, using `null` for `None`. Order: document `schema_version`, `next_id`, `dpi`, `pages`; page `id`, `name`, `size`, `locked`, `hidden`, `roots`, `nodes`; `size` `width`, `height` (each `{"value":..,"unit":..}`); node `base`, `kind`, `children`; base `id`, `name`, `placement`, `opacity`, `blend`, `visible`, `locked`; placement `x`, `y`, `w`, `h`, `rotation_deg`, `skew`, `flip` (`{"x":..,"y":..}`). `NodeKind` MUST be internally tagged by `type` with the values `group`, `shape`, `extension`; `shape` has `geometry` (`rect` or `ellipse`) then `fill`; `extension` has `type_id`, `schema_version`, `payload`. `NodeId`, `PageId` and `Dpi` MUST be bare numbers. `nodes` MUST be an array of nodes in ascending id order regardless of insertion order. `Arc<Page>` and `Arc<Node>` MUST serialize as the bare value. Colors MUST use the `gafira-color` flat form (`space` first).

#### Scenario: Exact minimal document (D3, unit, in-crate helper)
- GIVEN a document with `next_id = 3`, DPI 96 and one page `{id 1, "P", 100 x 200 px}` without nodes
- WHEN serialized with `to_json`
- THEN the output is exactly `{"schema_version":1,"next_id":3,"dpi":96,"pages":[{"id":1,"name":"P","size":{"width":{"value":100.0,"unit":"px"},"height":{"value":200.0,"unit":"px"}},"locked":false,"hidden":false,"roots":[],"nodes":[]}]}`

#### Scenario: Arena order is ascending id (D3, unit, in-crate helper)
- GIVEN nodes inserted with ids 10, 2, 7
- WHEN serialized
- THEN `nodes` is an array whose `base.id` values are 2, 7, 10

#### Scenario: Node shapes per kind (D3, unit, in-crate helper)
- GIVEN one group, one ellipse shape with an sRGB fill and one extension
- WHEN serialized
- THEN the `kind` objects are `{"type":"group"}`, `{"type":"shape","geometry":"ellipse","fill":{"space":"srgb",...}}` and `{"type":"extension","type_id":...,"schema_version":...,"payload":...}`, each node ends with `"children":[...]`, and a shape without fill writes `"fill":null`

#### Scenario: None is written as null (D3, unit, in-crate helper)
- GIVEN an `ElementBase` with `name = None`
- WHEN serialized
- THEN `base` contains `"name":null`

### Requirement: Deterministic round trip (ARQ-10) (D3, D6)

Equal documents MUST serialize to identical bytes on every platform. `to_json(from_json(to_json(d)))` MUST equal `to_json(d)` byte for byte. For valid documents, `from_json(to_json(d))` MUST equal `d` field by field with every float compared by `to_bits()`, including `-0.0`, subnormals, `f32::MAX` and `f64::MAX`. The workspace `serde_json` MUST keep `float_roundtrip`.

#### Scenario: Round trip idempotent (property)
- GIVEN generated valid documents (`ProptestConfig::with_cases(10_000)`)
- WHEN serialized, loaded and serialized again
- THEN the two byte strings are identical and the loaded document is bit-identical to the original

#### Scenario: Bit identity of numbers (D3, unit, in-crate helper)
- GIVEN `opacity = 1e-45_f32` (subnormal), `x = -0.0`, `w = f64::MAX`, `rotation_deg = 5e-324`
- WHEN round-tripped
- THEN every value is bit-identical and `x.is_sign_negative()` holds

#### Scenario: Same bytes from a different build order (D3, unit, in-crate helper)
- GIVEN two documents with the same nodes inserted in different orders
- WHEN serialized
- THEN the bytes are identical

#### Scenario: Arena order mutation is caught (D3, unit, in-crate helper)
- GIVEN a build whose nodes were inserted out of id order
- WHEN the serializer is mutated to emit the nodes in reverse output order (a `BTreeMap` has no insertion order to mutate)
- THEN the exact-order test fails (recorded as a property mutation)

### Requirement: Defaults, normalization and forward compatibility (D3)

Required on read: `schema_version`, `next_id`, `pages` (document); `id`, `size` (page); `base`, `kind` (node); `id`, `placement` (base); `x`, `y`, `w`, `h` (placement); `type` (kind); `type_id`, `schema_version`, `payload` (extension); `geometry` (shape); `space` (color). Only the fields listed next have a default, and they MUST default on read: `dpi` 96, `name` empty or `None`, `locked`, `hidden` false, `roots`, `nodes`, `children` empty, `opacity` 1.0, `blend` `normal`, `visible` true, `rotation_deg` 0, `skew` 0, `flip` false (and each of `flip.x`, `flip.y` false when missing), `fill` `None`; a re-save MUST write them explicitly. Any other field of any type, such as `Length` `value` and `unit` or the color channels, has no default and MUST be present. `deny_unknown_fields` MUST NOT be used at any level; unknown fields are ignored and dropped on save. An integer for an `f64` field (`"x":3`), nodes out of id order and non-canonical payloads MUST be accepted and normalized on save (`3.0`, ascending ids, canonical payload). A missing required field MUST be an error. An unknown `type`, `geometry`, `blend`, `unit` or `space`, or a wrong letter case (`"Group"`), MUST be an error.

#### Scenario: Defaults applied (unit)
- GIVEN a node JSON containing only `base.id`, `base.placement{x,y,w,h}` and `kind.type = "group"`, inside a page and document with only their required fields, where the page's `roots` is `[<that node id>]` (otherwise the node would be an orphan, which D5 rejects; `roots` is defaultable, so it is given explicitly here)
- WHEN loaded
- THEN the defaults above are applied exactly and re-saving writes them all

#### Scenario: Unknown fields ignored and dropped (unit)
- GIVEN a document with an extra field at document, page, node, base, placement and kind level
- WHEN loaded and re-saved
- THEN loading succeeds, equals the document without extras, and the output contains none of the extras

#### Scenario: Normalization on save (unit)
- GIVEN `"x":3`, nodes listed 7 then 2 with `roots` `[7,2]` (both are roots, so the fixture stays valid in D5), and an unsorted payload
- WHEN loaded and re-saved
- THEN the output has `"x":3.0`, nodes 2 then 7 and a canonical payload

#### Scenario: Missing required field (unit, table)
- GIVEN documents lacking, one per row, each required field of the list above
- WHEN loaded
- THEN each fails with a JSON error

#### Scenario: Unknown vocabulary (unit, table)
- GIVEN `"type":"text"`, `"type":"Group"`, `"geometry":"star"`, `"blend":"Multiply"`, `"unit":"parsec"`, `"space":"rec2020"`
- WHEN loaded
- THEN each fails with a JSON error (no silent drop, no panic)

### Requirement: Animatable compatibility of bare values (D3)

`opacity`, `placement` and `fill` (a `Color`) MUST stay bare on the wire. `Placement` MUST NOT have a container-level `#[serde(default)]` and MUST require `x`, `y`, `w`, `h`; `Color` requires `space`. A keyed object such as `{"keys":[]}` MUST fail to deserialize as `Placement`, as `Color` and as `f32`, so a future untagged `Animatable` falls through to its keyed form. A bare `0.5` MUST deserialize as an `f32`.

#### Scenario: Keyed object rejected (unit)
- GIVEN `{"keys":[]}`
- WHEN deserialized as `Placement`, as `Color` and as `f32`
- THEN all three fail

#### Scenario: Bare values (unit)
- GIVEN a node with `opacity = 0.5` and a shape fill
- WHEN serialized
- THEN `opacity` is the bare number `0.5`, and the placement and fill are flat objects with no tag or wrapper and no `keys` field

#### Scenario: Container default mutation is caught (unit)
- GIVEN a mutation adding `#[serde(default)]` to `Placement`
- WHEN the keyed-object test runs
- THEN it fails (recorded as a mutation)

### Requirement: Extension payload survival (ARQ-11 reformulated) (D3)

An `Extension` node MUST keep `type_id`, `schema_version` and `payload` through load and save; every extension is "unknown" in this change (no registry exists). The payload is a `serde_json::Value`. A canonical payload is exactly the output of `serde_json::to_string(&Value)` (sorted keys, no whitespace, shortest round-trip numbers, serde_json escaping). A canonical payload MUST come back byte for byte inside the document; any other payload MUST load as the same value and be written in canonical form, after which a second round trip changes nothing. The `preserve_order` and `arbitrary_precision` features of `serde_json` MUST NOT be enabled anywhere in the workspace. Goldens MUST be written in canonical form. From D4 a payload nested deeper than 64 levels is rejected by validation (`ExtensionTooDeep`), because `serde_json` serializes without a limit but cannot reload a document whose payload reaches about 122 levels.

#### Scenario: Canonical golden survives (integration)
- GIVEN the golden `extension.json` with a canonical payload (nested objects, arrays, integers, floats, strings, booleans, `null`)
- WHEN loaded with `from_json` and saved with `to_json`
- THEN the output bytes equal the golden bytes

#### Scenario: Non-canonical golden normalizes (integration)
- GIVEN the golden `extension_noncanonical.json` (unsorted keys, whitespace, `1E2`, an unescaped `é`)
- WHEN loaded and saved
- THEN the output equals `extension.json` byte for byte, and a second round trip is identical

#### Scenario: Key order and number spelling (unit)
- GIVEN the payloads `{"b":1,"a":{"z":0,"y":1}}`, `1.10`, `1`, `1.0`, `10E-1`
- WHEN loaded and saved
- THEN the first becomes `{"a":{"y":1,"z":0},"b":1}`, `1.10` becomes `1.1`, `1` stays an integer, and `1.0` and `10E-1` each get one canonical float spelling

#### Scenario: Integer beyond u64 (unit)
- GIVEN the payload numbers `18446744073709551616` and `18446744073709551615`
- WHEN loaded and saved
- THEN the first is stored as a float and written in float spelling (documented lossy normalization), the second stays an exact integer

#### Scenario: Duplicate keys in a payload (unit)
- GIVEN the payload `{"a":1,"a":2}`
- WHEN loaded and saved
- THEN the output is `{"a":2}` (last wins, documented)

#### Scenario: Unrepresentable payload number (unit)
- GIVEN the payload number `1e400`
- WHEN loaded
- THEN it fails with a JSON error and does not panic

#### Scenario: f64 field in an internally tagged node (unit)
- GIVEN an extension node and a shape node with `f64` values near the extremes
- WHEN round-tripped
- THEN every `f64` keeps its bits (the `arbitrary_precision` guard)

#### Scenario: Extension fields kept (unit)
- GIVEN `type_id` `"com.example.sticker"`, `schema_version` 2 and an object payload
- WHEN round-tripped
- THEN all three are unchanged and the extension's `schema_version` is not confused with the document's

#### Scenario: Feature guards (command)
- GIVEN the workspace lockfile
- WHEN `cargo tree -p gafira-doc -e features -i serde_json` runs
- THEN neither `preserve_order` nor `arbitrary_precision` appears, and `cargo tree -p gafira-doc -e features -i serde` shows `rc`

### Requirement: Canonical goldens survive every checkout (D3)

Golden files MUST be single-line compact JSON with no trailing newline and no byte-order mark, read with `include_str!` (no `std::fs`, banned in tests too). `crates/gafira-doc/tests/golden/.gitattributes` MUST mark them `*.json -text` so that a Windows checkout cannot convert LF to CRLF and change the bytes. The goldens MUST be `minimal`, `full`, `extension` and `extension_noncanonical`.

#### Scenario: Round-trip goldens (integration)
- GIVEN `minimal`, `full` and `extension`
- WHEN each is loaded and saved on Windows, Linux and macOS
- THEN the output equals the golden bytes (integration test over fixtures loaded without validation in D3); in a separate in-crate unit test, `full` equals the serialization of the same document built with the `#[cfg(test)]` helper

#### Scenario: Text conversion is off (command)
- GIVEN the repository
- WHEN `git check-attr text -- crates/gafira-doc/tests/golden/minimal.json` runs
- THEN the attribute is `unset`

#### Scenario: Golden shape (command)
- GIVEN each golden file
- WHEN checked for newline and BOM bytes
- THEN it contains no `\n`, no `\r` and no byte-order mark

### Requirement: Non-finite and out-of-range numbers (D3, D4)

Serialization MUST NOT panic for NaN or infinite numbers; `serde_json` writes them as `null`, and a document containing such a `null` in a number field MUST fail to load. Finite out-of-range values (for example `opacity = 1.5`) MUST serialize unchanged by serde alone; from D4 they are rejected by the validating load. Tests that claim identity MUST use `to_bits()`.

#### Scenario: Non-finite not persistable (D3, unit, in-crate helper)
- GIVEN documents built with NaN `opacity`, `+inf` `x`, `-inf` `rotation_deg`, a NaN color channel and a NaN page width
- WHEN serialized and the output loaded
- THEN serialization does not panic, the field is `null`, and loading fails with a JSON error

#### Scenario: Finite out-of-range survives serde, not load (unit)
- GIVEN a document with `opacity = 1.5` built without validation
- WHEN its nodes are serialized and deserialized as plain `Node` values
- THEN the value is bit-identical, and a full-document `from_json` (D4) fails with `LoadError::Invalid`

### Requirement: Validating load path (D4)

`Document` MUST implement `Deserialize` through `#[serde(try_from = "DocumentWire")]`, and the conversion MUST run `validate()`, so no invalid `Document` can come out of any deserialization, including `serde_json::from_str::<Document>`. `Document::from_json(&[u8]) -> Result<Document, LoadError>` MUST return `LoadError::Json(serde_json::Error)` for syntax, shape and encoding errors, and `LoadError::Invalid(ValidationError)` carrying the same variant `validate()` returns for a validation failure. `LoadError` MUST be `#[non_exhaustive]`, implement `Display` and `Error` with `source()`. `Document::to_json(&self) -> Result<String, serde_json::Error>` MUST write the compact canonical form. `DocumentWire` MUST NOT be public. Before D4 (D3), the unchecked conversion applies and tests of validation through load are not yet required.

#### Scenario: Plain serde cannot build an invalid document (unit)
- GIVEN valid JSON whose `next_id` is 1 while a node has id 5
- WHEN loaded with `serde_json::from_str::<Document>` and with `from_json`
- THEN both fail, and `from_json` returns `LoadError::Invalid(ValidationError::NextIdNotAboveMax { .. })`

#### Scenario: Shape versus validation (unit)
- GIVEN truncated JSON, and well-formed JSON with `"schema_version":2`
- WHEN loaded with `from_json`
- THEN the first is `LoadError::Json` and the second `LoadError::Invalid(UnsupportedSchemaVersion { found: 2 })`

#### Scenario: Error contract (unit)
- GIVEN a `LoadError::Invalid(..)` and a `LoadError::Json(..)`
- WHEN formatted and `source()` is called
- THEN both display text and `source()` returns the inner error

#### Scenario: Revert mutation is caught (unit)
- GIVEN a mutation replacing `try_from` with `from`
- WHEN the first scenario runs
- THEN it fails (recorded as a mutation)

### Requirement: Hostile and corrupt input (D3, D4, D6)

Loading MUST return an error, never panic, never overflow the stack and (from D4) never return an invalid document, for every input below. Each row is a table-driven test.

#### Scenario: Empty, truncated, trailing and encoding (unit, table)
- GIVEN the empty input, every prefix of a small golden, a valid document followed by `x`, invalid UTF-8 bytes, a leading byte-order mark and a lone surrogate escape `\ud800` in a string
- WHEN loaded with `from_json`
- THEN every case is `LoadError::Json`

#### Scenario: Wrong types (unit, table)
- GIVEN `null`, `[]`, `"x"`, `"next_id":"7"`, `"pages":{}`, `"locked":"yes"`, `"name":5`
- WHEN loaded
- THEN each fails with a JSON error

#### Scenario: Bad numbers (unit, table)
- GIVEN ids, `next_id` and `dpi` of `-1`, `1.5`, `1e3`, `null`, `18446744073709551616`, and `dpi` `4294967296`, and a float `1e400`
- WHEN loaded
- THEN each fails with a JSON error

#### Scenario: Duplicate struct field (unit)
- GIVEN a document with two `next_id` keys, and a `base` with two `opacity` keys, and a shape `kind` with two `geometry` keys
- WHEN loaded
- THEN all fail with a JSON error (the last row depends on serde derive behavior inside internally tagged variants; confirm in apply, design "Not Verifiable at Design Time")

#### Scenario: Duplicate node id inside one page (unit)
- GIVEN a page whose `nodes` array holds two nodes with `base.id` 5
- WHEN loaded
- THEN it fails with `LoadError::Json` (the arena cannot hold it; `validate()` is never reached)

#### Scenario: Duplicate id across pages and against a page id (integration)
- GIVEN two pages whose arrays both contain id 9, and separately a node id equal to a page id
- WHEN loaded (D4)
- THEN both fail with `LoadError::Invalid(DuplicateId { .. })`

#### Scenario: Nodes out of order are accepted (unit)
- GIVEN a `nodes` array with ids 7 then 2
- WHEN loaded
- THEN it succeeds and the saved array is 2 then 7

#### Scenario: Pure JSON recursion limit (unit)
- GIVEN a JSON value of 127 nested arrays and another of 128, and one of 10 000
- WHEN parsed with plain `serde_json` as a `Value`
- THEN 127 parses, and 128 and 10 000 fail with a recursion-limit error, not a stack overflow

#### Scenario: Payload depth inside a full document (unit)
- GIVEN a full document whose extension payload nests 121 levels, and another with 122
- WHEN loaded with `from_json`
- THEN 122 fails with `LoadError::Json` (recursion limit); 121 parses, and then (D4) fails validation as in the next scenario because 121 exceeds 64

#### Scenario: Deep payload rejected by validation and by load (D4, integration)
- GIVEN a document with an extension payload nested 65 levels, built through the in-crate helper or loaded from JSON
- WHEN `validate()` runs, and the document is serialized and loaded with `from_json`
- THEN validation fails with `ExtensionTooDeep`, and `from_json` fails with `LoadError::Invalid(ExtensionTooDeep { .. })`; a payload of 64 levels passes both and reloads bit-identical (a document that saves must always reopen)

#### Scenario: Deep tree (D5, integration)
- GIVEN a valid chain of 10 000 nested groups (flat arena)
- WHEN loaded with the full validation (P1 to P6, D5)
- THEN it succeeds without stack overflow

#### Scenario: Hostile values (unit)
- GIVEN `"dpi":0` and `"dpi":10001` (well-formed JSON), `"schema_version":0`, and a fill color with an unknown `space`
- WHEN loaded
- THEN the first three fail with `LoadError::Invalid` (D4) and the last with `LoadError::Json`

#### Scenario: Corruption never panics (D6, property Q5)
- GIVEN a golden document with a random single-byte mutation
- WHEN loaded with `from_json`
- THEN the result is `Err`, or an `Ok` document that passes `validate()`, and nothing panics
