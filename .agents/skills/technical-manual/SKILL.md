---
name: technical-manual
description: >
  Update the Gafira technical manual in docs/manuals/tecnico/ (English source,
  Spanish mirror, PDF build). Trigger: edit, add, review or translate a manual
  chapter, find which chapter documents a crate, API, gate or spec, mark a part
  Implemented or Planned, run manual.py check or build, or generate the manual PDFs.
---

# Technical manual: update, find, build

The manual documents what is implemented in the repository today. English is the source (`chapters/`, `index.md`); Spanish mirrors it (`chapters-es/`, `index-es.md`) with the same numbers and file names. The tool is `docs/manuals/tecnico/manual.py` (standard library only); its `README.md` is the short user guide. Product intent lives in `specs/`; if the manual and the spec disagree, the spec and the owner decisions win.

## Use it when

Any change to a public API, a layering rule, a CI gate or the process needs the matching chapters updated in both languages, in the same pull request. Also use it to add a chapter, fix a chapter, or produce the PDFs.

## Three workflows (read only what you need)

| Need | Read |
|---|---|
| Write or edit a chapter (structure, tone, EN then ES, Implemented/Planned, evidence, `check` parity, page rules) | `references/writing.md` |
| Locate the chapter or section to change (numbering map, `list`, grep recipes, code-to-chapter table, EN/ES pairing) | `references/finding.md` |
| Produce the PDFs (`check`, `build --lang`, engines, output paths, verification, failures) | `references/building-pdfs.md` |

The reference files sit in this skill's folder. Subagents cannot call the Skill tool: whoever launches `manual-writer` resolves the skill name with the `skill-registry` skill and passes the path; the agent reads the files it needs.

## Invariants

- English first, then the Spanish mirror; never leave a stub (`<!-- TRANSLATE`) behind.
- Every claim comes from code or specs that exist. Name the source (crate, file, requirement ID). Unbuilt parts are marked **Planned** (**Planificado**).
- No absolute or user paths, no AI attribution, no invented claims.
- Same heading levels and the same number of code fences in both languages; the first line is `# Title` and matches the index entry.
- `docs/manuals/tecnico/build/` is generated and gitignored: never commit PDFs or `build/` files.
- Do not change `manual.py` while updating content; report tooling gaps instead.

## Update checklist

1. Find: locate the chapter and section (`references/finding.md`); confirm the EN/ES pair.
2. Edit the English chapter (`references/writing.md`); update `index.md` only if a title changes.
3. Mirror the Spanish chapter (and the `index-es.md` title); translate prose only.
4. Check: `python3 docs/manuals/tecnico/manual.py check` (also runs the EN/ES mirror checks).
5. Build when asked or before a release: `build`, then `build --lang es` (`references/building-pdfs.md`).
6. Commit with a conventional message: `docs(manual): <what changed>` (one work unit per commit, both languages together).
