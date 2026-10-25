# Document model and canonical JSON

## Entry points

| Call | Behavior |
|---|---|
| `Document::from_json(&[u8]) -> Result<Document, LoadError>` | Reads UTF-8 JSON, then validates. `LoadError::Json` for syntax, shape or encoding errors; `LoadError::Invalid` for well-formed JSON that breaks a document rule |
| `Document::to_json(&self) -> Result<String, serde_json::Error>` | Writes the compact canonical form, without a trailing newline |

Deserialization of `Document` goes through an unchecked wire type and `validate()`, so plain `serde` cannot produce an invalid document. This file I/O boundary belongs to the **Planned** `gafira-format` crate; today the document crate only converts bytes and values.

## Canonical form

Output is deterministic: saving the same document always produces the same bytes, whatever order it was built in.

- Keys are written in declaration order and every field is written; `None` is written as `null`.
- The order is: document `schema_version`, `next_id`, `dpi`, `pages`; page `id`, `name`, `size`, `locked`, `hidden`, `roots`, `nodes`; node `base`, `kind`, `children`; base `id`, `name`, `placement`, `opacity`, `blend`, `visible`, `locked`; placement `x`, `y`, `w`, `h`, `rotation_deg`, `skew`, `flip`.
- `NodeKind` is internally tagged by `type` with the values `group`, `shape` and `extension`. A shape writes `geometry` (`rect` or `ellipse`) then `fill`; an extension writes `type_id`, `schema_version` and `payload`.
- `NodeId`, `PageId` and `Dpi` are bare numbers. `Length` is `{"value":..,"unit":..}`. `Color` uses the flat form of `gafira-color`.
- `nodes` is an array in ascending id order, whatever the insertion order.
- Blend modes use the CSS kebab-case names (`color-dodge`).

A minimal document, as stored in `crates/gafira-doc/tests/golden/minimal.json`:

```json
{"schema_version":1,"next_id":3,"dpi":96,"pages":[{"id":1,"name":"P","size":{"width":{"value":100.0,"unit":"px"},"height":{"value":200.0,"unit":"px"}},"locked":false,"hidden":false,"roots":[],"nodes":[]}]}
```

## Reading rules

| Rule | Detail |
|---|---|
| Required fields | `schema_version`, `next_id`, `pages`; page `id`, `size`; node `base`, `kind`; base `id`, `placement`; placement `x`, `y`, `w`, `h`; kind `type`; extension `type_id`, `schema_version`, `payload`; shape `geometry`; color `space`. A missing one is an error |
| Defaults on read | `dpi` 96; `name`, `locked`, `hidden`, `roots`, `nodes`, `children`; `opacity` 1.0; `blend` `normal`; `visible` true; `rotation_deg` 0; `skew` 0; `flip` false; `fill` none. A re-save writes them explicitly |
| Unknown fields | Ignored and dropped on save; `deny_unknown_fields` is not used, for forward compatibility |
| Unknown vocabulary | An unknown `type`, `geometry`, `blend`, `unit` or `space`, or a wrong letter case, is an error |
| Normalization | An integer for an `f64` field is accepted and written as a float; nodes out of id order are accepted and written in ascending order |
| Duplicate ids | Duplicate node ids in one page fail at load; duplicates across pages are caught by validation |

## Extension payloads

An `Extension` node keeps `type_id`, `schema_version` and `payload` through load and save. The payload is a `serde_json::Value`. A canonical payload is exactly the output of `serde_json::to_string(&Value)` (sorted keys, no whitespace, shortest round-trip numbers) and comes back byte for byte. Any other payload loads as the same value and is written canonically; a second round trip changes nothing. Known normalizations: duplicate keys keep the last value, and an integer beyond `u64` becomes a float.

The extension's `schema_version` is its own and is unrelated to the document's. The `preserve_order` and `arbitrary_precision` features of `serde_json` must not be enabled anywhere in the workspace, because they would break key order and `f64` bit identity. The workspace enables `float_roundtrip`.

## Limits

| Limit | Value | Reason |
|---|---|---|
| Extension payload depth | 64 levels (`MAX_EXTENSION_DEPTH`) | `serde_json` cannot reload a document whose payload reaches about 122 levels; a saved document must reopen |
| Page side | 1 to 16 384 px, inclusive, at the document DPI | `PageSize::MIN_PX` and `MAX_PX` |
| DPI | 1 to 10 000 | `Dpi::MIN` and `Dpi::MAX` |
| Schema version | 1 | Load accepts only version 1 |

## Hostile input

Loading returns an error and never panics, overflows the stack or returns an invalid document for empty, truncated or trailing input, invalid UTF-8, wrong types, bad numbers, duplicate fields, deep trees and deep payloads. Property tests corrupt valid documents and check that the corruption is detected.

## Golden files

`crates/gafira-doc/tests/golden/` holds `minimal.json`, `full.json`, `extension.json` and `extension_noncanonical.json`. Canonical goldens must round-trip byte for byte on every checkout, so `crates/gafira-doc/tests/golden/.gitattributes` marks `*.json -text`. That stops a Windows checkout from converting LF to CRLF and changing the bytes.

## Planned

Reading and writing `.gafira` files, atomic saves, linked media, version folders and per-step schema migrations are specified (spec 02, section 7) for `gafira-format` and are not implemented. The `gafira-cli validate` subcommand moves with it (D-13).
