# Writing a manual chapter

Self-contained guide. Paths are relative to the repository root; the manual lives in `docs/manuals/tecnico/`.

## Edit or add

| Situation | Action |
|---|---|
| A fact in an existing chapter changed (API, gate, rule, status) | Edit that chapter in both languages |
| A new crate, subsystem or process area has no home | Add a chapter: `python3 docs/manuals/tecnico/manual.py new <slug> "<Title>" --title-es "<Titulo>"` |
| A topic is only a section of an existing area | Add a `##` section to the existing chapter, never a new file |
| A planned part becomes built | Flip its status row and rewrite its text in the same pull request as the code |

`new` creates the next number in both trees and both indexes; the Spanish file is a stub with a `<!-- TRANSLATE` marker that `check` rejects until it is replaced. To reorder, rename files in both trees so numbers stay contiguous from `01` and reorder both indexes. Chapter numbers are capped at 99.

## Order of work

1. Read the code or spec you are documenting (crate source, `specs/capabilities/<capability>/`, `specs/`). Quote nothing from memory.
2. Write or edit the English chapter.
3. Mirror it in `chapters-es/` with the same file name.
4. Update both index titles if a chapter title changed.
5. Run `check` (see `building-pdfs.md`).

## Structure of a chapter

- Line 1 is `# Title` (nothing before it). It must equal the index entry text exactly.
- Then `##` sections; `###` only when needed. Do not skip levels. The PDF table of contents uses depth 2.
- Typical shape for a crate: Purpose, Public API, Invariants, Tests (see `03-crate-gafira-geom.md`). A chapter with Planned parts opens with a `## Status` table (see `08-edit-command-history.md`).
- Tables for facts (API items, states, rules); short paragraphs for explanation. A fact that fits a row does not go in prose.
- Link to other chapters with relative links (`02-architecture.md`); `check` verifies they resolve.

## Tone

Neutral, professional, present tense, short sentences. Say what exists and what it guarantees. No marketing, no first person, no opinions. Define a term once (glossary chapter) and reuse it.

## Implemented vs Planned

- Anything not in the repository today is **Planned** (Spanish **Planificado**), in bold, as a status cell, a section title (`## Planned: History`) or a sentence prefix.
- Anything built is stated as fact or marked Implemented (**Implementado**) in status tables.
- A change still open under `specs/changes/<name>` is Planned, and the chapter cites that change. Merged to `main` with code is Implemented.
- Never describe the spec's design as if built. When unsure, read the code; if the symbol is absent, it is Planned.

## Evidence and no invented claims

- Every behavior, limit, error variant or number comes from code, tests, a delta spec requirement or a product spec. Name the source inline: crate, module or file, requirement ID (`RF-`, `RNF-`, `ARQ-`, `D-nn`, CI gate `V13`).
- Verify numbers (limits, versions, counts) by grepping the code or running the command. Do not copy from older text without checking.
- If a claim cannot be verified, omit it or mark it Planned with its source; do not guess.

## Language rules (Spanish mirror)

- Translate prose only. Identifiers, code, commands, paths, crate names, requirement IDs and file names stay as written.
- Register: neutral, professional, impersonal. Status words: Implementado, **Planificado**.
- Keep the same heading levels in the same order and the same number of code fences as the English file. Heading text is translated; the levels and counts are what `check` compares.
- Translate the index title in `index-es.md`; it must equal the Spanish `# Title`.
- Remove the `<!-- TRANSLATE` marker only when the chapter is fully translated.

## Paths and attribution

- No absolute or user paths anywhere (`check` fails on home-directory paths of Linux and macOS, Windows drive paths, `~/x` and `file://`). Use repository-relative paths in backticks.
- No AI attribution in text or commits.
- Skills are referenced by name only.

## Page-break and overflow rules (PDF)

The build starts every chapter on a new page (each `#` heading) and handles wrapping, but the content must cooperate:
- One `#` per file: the chapter title. Sections use `##` or deeper.
- Inline code and paths wrap at `/ . _ -`; a very long token without separators still overflows, so avoid them.
- Tables render in small type with wrapping cells. Keep them to about 4 columns and short cells; move long text out of cells into prose. Pipe tables only (no grid tables, no HTML).
- Code blocks wrap; keep lines under about 90 characters anyway and give every fence a language tag.
- If an image is ever added, use a relative path that `check` can resolve.
- Do not add manual page breaks or raw HTML or LaTeX.

## Before committing

- `check` passes (it validates both languages whichever `--lang` is used).
- `git status` shows changes only under `docs/manuals/tecnico/chapters/`, `chapters-es/` and the indexes; nothing under `build/`.
- Commit `docs(manual): <summary>` with both languages in the same commit.
