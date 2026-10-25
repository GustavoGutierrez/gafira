# Gafira technical manual

English technical documentation of what is implemented in the repository today. Anything specified but not built is marked **Planned**. The product intent stays in `docs/specs/`; if the two disagree, the spec and the owner decisions win.

```
docs/manuals/tecnico/
  index.md        ordered list of chapters (the table of contents)
  chapters/       NN-slug.md, one file per chapter
  manual.py       list, new, check, build (standard library only)
  build/          generated Markdown, HTML and PDF (gitignored, never committed)
```

## Requirements

- Python 3.9 or newer (no packages).
- For the PDF: `pandoc` plus one engine. `build` tries, in order: `xelatex`, then `google-chrome` or `chromium` (headless print), then `wkhtmltopdf`. Force one with `--engine xelatex|chrome|wkhtmltopdf`.

## Write or update a chapter

1. Edit the file in `chapters/`. Keep the first line as `# Title`; the title must match the index entry.
2. Base every claim on code or specs that exist. Name the source (crate, file, requirement ID). Mark unbuilt parts **Planned**.
3. Write in neutral, professional English. Use tables for facts and short paragraphs for explanation. No absolute or user paths, no AI attribution.
4. Run `python3 docs/manuals/tecnico/manual.py check`.

Update the matching chapter in the same pull request as any change to a public API, a layering rule, a CI gate or the process.

## Add a chapter

```bash
python3 docs/manuals/tecnico/manual.py new testing-strategy "Testing strategy"
```

This creates the next numbered file from a template and appends it to `index.md`. To reorder chapters, rename the files so the numbers stay contiguous from `01`, and reorder the index to match.

## Commands

| Command | What it does |
|---|---|
| `manual.py list` | Prints the chapters in index order |
| `manual.py new <slug> "<Title>"` | Creates the next numbered chapter and indexes it |
| `manual.py check` | Index and files agree, names are `NN-slug.md`, numbers are contiguous, titles match, no absolute or user paths, relative links resolve, `build/` is gitignored |
| `manual.py build [--engine E]` | Runs `check`, joins the chapters into `build/manual.md` and renders `build/Gafira_Technical_Manual.pdf` |

## Generate the PDF

```bash
python3 docs/manuals/tecnico/manual.py build
```

The PDF has a title page with the workspace version and the date, a table of contents and numbered sections. The `build/` folder is gitignored: do not commit the PDF. Publish it as a release or CI artifact when needed.
