# 0006. Schema 1 stays open until a build can save, and every field is written

- Status: proposed
- Date: 2026-10-30
- Related: ARQ-10, ARQ-13, ARQ-53, 02 §3.5, 02 §7.1, change `layout-core` (spike items 1 and D1)

## Context

`layout-core` adds fields to `Page`. The spike measured three facts:

- Adding fields does not require a new version: old documents load with serde defaults (`spike.md` item 1, experiment A).
- A new document read by an older build silently drops the new fields on re-save, because unknown fields are ignored (02 §3.5). The version is the same, so the read-only guard of 02 §7.1 does not fire.
- Always writing the new fields changes 7 tests and 3 goldens. Skipping default values changes none.

No document can exist in the wild today. `v0.1.0-beta.1` attaches no binaries and `gafira-cli` only has `--version` (`CHANGELOG.md`). `gafira-format`, which owns `.gafira` and migrations, does not exist yet.

## Decision

1. **Schema 1 stays open until the first build that writes a `.gafira` file reaches a user.** Until then, additive fields join schema 1. `Document::SCHEMA_VERSION` stays 1 and no migration is written.
2. **After that build, any new field that holds user data bumps the schema**, with a `migrate_n_to_n1` and its golden (02 §7.1). The version is the only guard against the silent drop above.
3. **Every field is always written.** No `skip_serializing_if` in `gafira-doc`. `#[serde(default)]` stays on read (02 §3.5).
4. **`GridSettings.style`** follows the same rule: always written, `#[serde(default)]` is `Lines`, and it needs no bump now. `GridStyle` is a closed enum, so an unknown variant fails to load; adding a variant after the first saving build is a bump.

## Consequences

- `page_wire_form_writes_every_field` keeps its meaning, and the wire form stays self-describing.
- A default can change without changing the meaning of saved files. This matters here: the grid defaults are PROPUESTA and may change with measurements (FD-07, D-34).
- The first delivery updates 7 tests and 3 goldens. The old bytes stay as an input-only fixture that must load and re-save in the new canonical form, a stronger test than the old byte equality.
- Every later field addition costs the same golden update. That is the purpose of byte-pinned goldens.
- The silent drop stays possible between two development builds on schema 1. It can only touch test data.
- Rule 2 needs a named trigger: the coordinator checks it when `gafira-format` is planned.

## Alternatives considered

- **Bump to 2 now.** Needs a `migrate_1_to_2` with no owner crate and no file to migrate, and makes `validate` reject every existing golden until that exists.
- **Skip default values.** Zero churn today, but an absent field then means "whatever the default is in the reading build", so changing a PROPUESTA default silently changes old documents. It also drops `-0.0`, because `0.0 == -0.0` in the default check, so a value does not round-trip bit for bit.
- **Rely on `features_required` of the manifest (02 §7.1) instead of bumping.** A useful addition in `gafira-format`, but it is a manifest field of a crate that does not exist. It does not replace the version.
