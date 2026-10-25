# Finding the chapter or section to update

Self-contained guide. Paths are relative to the repository root.

## List the chapters

```bash
python3 docs/manuals/tecnico/manual.py list            # English, index order
python3 docs/manuals/tecnico/manual.py list --lang es  # Spanish
```

Output is `NN  Title  (NN-slug.md)`. The index files (`docs/manuals/tecnico/index.md`, `index-es.md`) define the order; `chapters/` and `chapters-es/` share the same `NN-slug.md` names. The table below can go stale when chapters are added: trust `list`.

## Code area to chapter

| Code area, spec or change | Chapter (file in `chapters/`) |
|---|---|
| Repository map, current-state table, reading order | `01-overview.md` |
| Layering, crate list, allowed dependencies (V5), enforced rules, planned crates | `02-architecture.md` |
| `crates/gafira-geom` (lengths, units, transforms); spec `geometry-primitives` | `03-crate-gafira-geom.md` |
| `crates/gafira-color` (color spaces, conversions, wire form); specs `color-model`, `color-conversions` | `04-crate-gafira-color.md` |
| `crates/gafira-doc` public API, model types, invariants, placement geometry; spec `document-model` | `05-crate-gafira-doc.md` |
| Canonical JSON, reading rules, limits, golden files; spec `document-serde` | `06-document-model-and-json.md` |
| Validation passes and where validation runs; spec `document-validation` | `07-validation.md` |
| `crates/gafira-edit` (Command, Transaction, History, commands), `crates/gafira-testkit`; spec `edit-capability`; change `edit-history` | `08-edit-command-history.md` |
| CI, local gates, lints, release profile, license and dependency policy, mutation testing; specs `workspace-tooling`, `cli-entrypoint` | `09-tooling-and-quality.md` |
| Lite SDD, TDD, commits, branches, PRs, `AGENTS.md`, the `openspec/` flow | `10-methodology.md` |
| Terms and acronyms | `11-glossary.md` |
| A crate with no row (new crate) | Add a chapter or a section, and update `02-architecture.md` and the state table in `01-overview.md` |
| Product intent in `docs/specs/` (00 decisions, 02 architecture, backlog 01 and 03 to 06) | Intent is not documented; only the built part appears in the matching chapter, the rest as **Planned** |

Spec names are folders in `openspec/specs/`; open changes are folders in `openspec/changes/`. Refresh the picture with `ls crates openspec/specs openspec/changes`.

## Grep recipes

Run from the repository root. Search both trees so the pair is found together.

```bash
M=docs/manuals/tecnico
# by heading
grep -rn '^#\{1,3\} .*History' $M/chapters $M/chapters-es
# by crate or API name (identifiers are not translated, so one pattern hits both)
grep -rn 'gafira-edit\|Transaction' $M/chapters $M/chapters-es
# by requirement ID, CI gate or change name
grep -rln 'V13\|edit-history' $M/chapters
# all headings of one chapter
grep -n '^#' $M/chapters/05-crate-gafira-doc.md
# what is still Planned
grep -rn 'Planned' $M/chapters
grep -rn 'Planificado' $M/chapters-es
# where a symbol lives in code, to decide which chapter owns it
grep -rn 'pub struct Transaction\|pub trait Command' crates
```

Search order: (1) the table above, (2) `list`, (3) grep the symbol or heading, (4) read the chapter's `## Status` or `## Purpose` to confirm ownership.

## Confirm the EN/ES pair

1. The same file name exists in both trees:
   ```bash
   ls docs/manuals/tecnico/chapters/NN-slug.md docs/manuals/tecnico/chapters-es/NN-slug.md
   ```
2. Quick skeleton comparison (heading lines and fence lines; counts include fenced lines, fine for a rough check):
   ```bash
   grep -c '^#' docs/manuals/tecnico/chapters/NN-slug.md docs/manuals/tecnico/chapters-es/NN-slug.md
   grep -c '^```' docs/manuals/tecnico/chapters/NN-slug.md docs/manuals/tecnico/chapters-es/NN-slug.md
   ```
3. Both index titles equal the first line of their chapter.
4. `python3 docs/manuals/tecnico/manual.py check` is the authority: it verifies all of this for both editions.

## Add versus edit

Edit when the chapter's purpose already covers the topic. Add a chapter only for a new crate or a topic with its own audience; the procedure is in `references/writing.md`.
