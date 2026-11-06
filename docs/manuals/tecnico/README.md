# Gafira technical manual

Technical documentation of what is implemented in the repository today, in English (source) and Spanish. Anything specified but not built is marked **Planned** (**Planificado** in Spanish). The product intent stays in `specs/`; if the two disagree, the spec and the owner decisions win.

```
docs/manuals/tecnico/
  index.md, chapters/        English edition (source): ordered list and NN-slug.md files
  index-es.md, chapters-es/  Spanish edition: same numbers and file names
  cover.tex, backcover.tex   Title page and back cover templates (filled at build time)
  manual.py                  list, new, check, build (standard library only)
  build/                     generated Markdown, HTML and PDF (gitignored, never committed)
```

Outputs: `build/Gafira_Technical_Manual.pdf` (English) and `build/Gafira_Manual_Tecnico.pdf` (Spanish).

## Requirements

- Python 3.9 or newer (no packages).
- For the PDF: `pandoc` plus one engine. `build` tries, in order: `xelatex`, then `google-chrome` or `chromium` (headless print), then `wkhtmltopdf`. Force one with `--engine xelatex|chrome|wkhtmltopdf`.

## Write or update a chapter

1. Edit the English file in `chapters/`, then the same file name in `chapters-es/`. Keep the first line as `# Title`; the title must match its index entry (`index.md` or `index-es.md`).
2. Base every claim on code or specs that exist. Name the source (crate, file, requirement ID). Mark unbuilt parts **Planned**.
3. Write in neutral, professional language. Use tables for facts and short paragraphs for explanation. No absolute or user paths, no AI attribution.
4. Translate prose only. Identifiers, code, commands, paths, crate names and requirement IDs stay as they are. Keep the same heading levels and the same number of code blocks as the English file: `check` compares them.
5. Run `python3 docs/manuals/tecnico/manual.py check`.

Update the matching chapters, in both languages, in the same pull request as any change to a public API, a layering rule, a CI gate or the process.

## Add a chapter

```bash
python3 docs/manuals/tecnico/manual.py new testing-strategy "Testing strategy" --title-es "Estrategia de pruebas"
```

This creates the next numbered file in both languages and appends it to both indexes. The Spanish file is a stub containing a `<!-- TRANSLATE` marker; `check` fails until the marker is replaced by the real translation. To reorder chapters, rename the files in both trees so numbers stay contiguous from `01`, and reorder both indexes.

## Commands

All commands accept `--lang en|es` (default `en`).

| Command | What it does |
|---|---|
| `manual.py list` | Prints the chapters of the selected edition in index order |
| `manual.py new <slug> "<Title>" [--title-es T]` | Creates the next numbered chapter in both languages and indexes it |
| `manual.py check` | For the selected edition: index and files agree, names are `NN-slug.md`, numbers are contiguous, titles match. Always, for both editions: the Spanish tree mirrors the English one (same chapters, same heading structure and code block count, no stubs), no absolute or user paths, relative links resolve, and `build/` is gitignored |
| `manual.py build [--engine E]` | Runs `check`, joins the chapters into `build/manual.md` (`build/manual-es.md` for Spanish) and renders the PDF |

## Generate the PDFs

```bash
python3 docs/manuals/tecnico/manual.py build
python3 docs/manuals/tecnico/manual.py build --lang es
```

Each PDF opens with a cover and closes with a back cover, follows with a table of contents, numbered sections, and every chapter starting on a new page (the first chapter does not share a page with the table of contents). Layout is handled by the build: with `xelatex`, inline code gets break points after separators (`/`, `.`, `_`, `-`), code blocks wrap, and tables use small type with wrapping cells; the HTML engines get equivalent CSS. The `build/` folder is gitignored: do not commit the PDFs. Publish them as release or CI artifacts when needed.

## Cover and back cover

`cover.tex` and `backcover.tex` are the two page designs. `manual.py` fills their `{{...}}` placeholders from the `LANGS` table (title, kicker, blurb, author, edition, legal notice, version and date) and rejects a template with an unfilled placeholder. The pages use the brand tokens — ink `#18243B`, coral `#FF6B57`, canvas `#FAFAF7`, mint `#3FBFA9`, mist `#E7EAF2` — from `docs/manuals/marca/manual-gafira.md`, the primary logo on light and the dark-background logo on ink, and the display families of the brand manual (`Manrope` for titles, `Inter` for text) taken from `assets/fonts/`. Both pages belong to the `xelatex` path only; the HTML fallbacks render the document without them.
