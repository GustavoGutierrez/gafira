---
name: product-specs
description: >
  Manage and write the product spec documents in docs/specs/ and turn them into
  OpenSpec changes. Trigger: creating, editing, promoting or archiving a product
  document (00-06), updating the docs/specs/README.md status table, adding or
  renumbering requirement IDs (ARQ-, RF-, RNF-, CAL-, DIS-, FD-, FV-, D-),
  fixing a product spec row on a reported ambiguity, or deriving a change from
  a product document (fast-lane Slice card by default; proposal and delta specs
  only on owner request or in full mode).
---

# Product specs: lifecycle, writing standard, OpenSpec derivation

Product documents describe WHAT and WHY in Spanish; OpenSpec changes describe the slice being built, per delivery. The flow after the spec is in `AGENTS.md` (lite SDD); do not repeat it here. Read `docs/specs/README.md` first: its status table is the live state.

## 1. Layout and states

| Place | Meaning |
|---|---|
| `docs/specs/<NN>-*.md` | In progress. One at a time. |
| `docs/specs/backlog/` | Waiting. |
| `docs/specs/archive/` | Completed (same file name). |
| `docs/specs/00-decisiones.md` | Owner decision record (D-nn). Overrides 01-06. Never promoted or archived. Not reopened without the owner. |

Exception to "one at a time": a change that needs a backlog document (e.g. UI needs 03) promotes it at that moment.

## 2. Procedures (each in its own commit)

Commands live in `docs/specs/README.md` ("Promover", "Archivar"); follow them. Always:
1. `git mv` (never copy), so history follows the file. Cross-references like "01 §x" stay valid because file names do not change.
2. Fix path references: `git grep "backlog/<NN>"` (promote) or `git grep "<NN-doc>"` (archive); also check `AGENTS.md`, `.agents/`, `openspec/`.
3. Update the status table in the same commit as the move.
4. Archive only when the criterion holds (section 5); then promote the next backlog document, as a separate commit.

Commit types: `docs(specs): promote <NN-doc>`, `docs(specs): archive <NN-doc>`, `docs(specs): update status table`.

## 3. Writing standard for a product document

Spanish, neutral, short sentences. Check the real documents before inventing anything; these are the conventions they use.
- Title `# NN — Name`, then a header table: `Documento | Estado | Alcance | Fuentes/Depende de | Convenciones` (and `Lo usan`, numbering notes when needed). See `references/product-doc-template.md`.
- Numbered sections `## N.` and `### N.M`. Cross-references by number: `02 §3.5`, `01 §9`.
- Stable IDs, one prefix per document: `ARQ-` and `ADR-` (02), `RF-`, `RNF-`, `CAL-` (01), `DIS-` and `PD-` for proposals (03), `FD-` (04), `FV-` (05), `D-` (00). IDs are never renumbered or reused; retired numbers become documented reserved gaps in the header. New IDs take the next free number of the document.
- One testable statement per ID, in a table row: `| ID | Requisito | Criterio de aceptación |` (02), plus `PRD | Fase | ARQ | Detalle` (01 RF) or `Cifra | Hardware | Método | Trazo` (01 CAL). A requirement with no verifiable acceptance criterion is not an ID yet.
- Phase/priority: `MVP`, `Fase 2`, `Fase 3` (01 §5).
- Markers, exactly as defined in each header: **FIJADA** (figure fixed by the owner in 00), **PROPUESTA** (not in the sources, not yet decided), **VERIFICAR** (fact to confirm before selling). Do not invent others.
- Owner decisions live only in 00, as `D-nn`. Other documents cite them (`D-03`), never restate them differently. If a document contradicts 00, 00 wins; fix the document.
- Implementation detail only as far as 02 fixes it (crates, traits, ports). 01/03/04/05 name 02 sections instead of redefining them. Names in the UI come from the glossary in 03 §2.
- Trace both ways: each RF cites its PRD item and ARQ; 01 §14 keeps the reverse PRD to RF table. Update it when IDs change.

## 4. From product document to a change

Default: fast lane (ADR 0002). The product document IS the acceptance source; it is not rewritten.
- The coordinator writes a Slice card at the top of `openspec/changes/<change>/apply-progress.md` (fields in `.agents/agents/coordinator.md`: product IDs, scope, non-goals, deliveries if more than one, plan, Flow line), committed as `docs(<change>): slice card`. No `proposal.md`, `design.md` or delta specs.
- Pick a coherent slice and name the change as in steps 1 and 2 below (the Change Chain row goes in the status table "Detalle" at archive instead of a proposal).
- Each ID in the card must have a testable "Criterio de aceptación". If one does not, or `tdd-apply`/`verify` report an ambiguity, fix the product document row here (Spanish, same ID, or the next free ID), in its own `docs(specs)` commit; never write a delta spec for it unless the owner asks.
- Tests name the ID they prove (test name or a one-line comment), so `git grep ARQ-nn` finds them.

Full OpenSpec derivation (only when the owner asks, or the change runs in full mode and needs a proposal or delta specs). Template files: `references/change-template.md`.
1. Pick a coherent slice of the in-progress document (a layer or a crate group, not a section count). Risky claims (limits, CI behavior, library capabilities, cross-document numbers) go to `explorer` first; fix refuted ones in the product document before specs rely on them.
2. Name it in kebab-case after the capability (`domain-doc`, `edit-history`). Add a row to the Change Chain table (order `C<n>`, change, content) in the proposal; the first change's proposal holds the table, later ones extend it.
3. `openspec/changes/<change>/proposal.md`: intent, scope, out of scope, approach, risks, rollback plan, deliveries. Each delivery is at most ~700 changed lines, with a forecast and the note that measured size runs x1.4 to x1.9 over forecast (mostly tests). Re-cut when the ratio is above x1.3 after the first delivery. Review budget in `openspec/config.yaml` is 800.
4. `design.md` only for a real open decision, only about that decision.
5. Delta specs `specs/<capability>/spec.md` with `## ADDED|MODIFIED|REMOVED|RENAMED Requirements` (REMOVED needs Reason and Migration). RFC 2119 keywords. Heading shape used in the repo: `### Requirement: <name> (<product IDs>) (<deliveries>)`, for example `(ARQ-10) (D3, D6b)`. Scenarios: `#### Scenario: <name> (<delivery>, <type>)` with GIVEN/WHEN/THEN, type one of unit, property, integration, command, manual.
6. Every requirement cites the product IDs it implements (ARQ, RF, CAL...). A product ID that the change deliberately changes (a reformulation, a deviation) is listed as an amendment in the proposal; the product document is edited in its own commit, not silently.
7. Spec files follow the language already used in that change; code, comments and commits are English.

## 5. Traceability and completion

- Coverage: fast lane, `git grep -n "ARQ-10" crates/ apps/` finds the tests that prove it and the status table names the change; older or full changes, `git grep -n "ARQ-10" openspec/` finds the requirement and the change. Keep IDs verbatim (test names or comments, requirement headings) so the grep works.
- Each change's `verify-report.md` lists the product IDs it covers and any it deliberately leaves out, with the reason.
- When a change is archived, update the status table "Detalle" in the same commit or the next one: `Hecho: C<n> <change> (IDs)`, and name what follows. For fast-lane changes `archive` does it; there is no delta merge and `openspec/specs/` is not edited.
- Archive criterion for a product document: every requirement ID in it is either covered by an archived change (status table, tests naming the ID, or older delta specs) or explicitly marked as not applicable or moved; check with `git grep` per ID range, then say so in the table. Reserved gaps do not need coverage.

## 6. Hand-off

product spec + Slice card -> `explorer` (risky claims) -> `tdd-apply` -> `verify` -> `mutants` T4 per touched crate at close (per the Flow line's `mutation`) -> `archive`, run by the `coordinator`; `specifier` only on a reported ambiguity; see `AGENTS.md` for gates, delivery sizes and integration rules (ADR 0003). The coordinator keeps the status table current and promotes the next document when one completes. Related skills: `cognitive-doc-design` (prose), `work-unit-commits`, `git-workflow-and-versioning`.
