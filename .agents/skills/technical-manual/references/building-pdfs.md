# Generating the PDFs

Self-contained guide. Run from the repository root; the tool resolves its own paths, so any working directory works.

## Commands

```bash
python3 docs/manuals/tecnico/manual.py check            # must pass first (also checks the ES mirror)
python3 docs/manuals/tecnico/manual.py build            # English
python3 docs/manuals/tecnico/manual.py build --lang es  # Spanish
python3 docs/manuals/tecnico/manual.py build --engine xelatex   # force an engine
```

`build` runs `check` itself and stops if it fails. `--engine` accepts `auto` (default), `xelatex`, `chrome`, `wkhtmltopdf`.

## Requirements and engines

- Python 3.9 or newer, no packages.
- `pandoc` is mandatory (`build` exits with "pandoc is required" otherwise).
- Engine order in `auto`: `xelatex` (through pandoc), then HTML printed by headless `google-chrome` / `chromium` / `chromium-browser`, then `wkhtmltopdf`.
- Fallback: if xelatex is missing or fails, `auto` falls through to the HTML engines. A forced engine that is missing or fails produces an error and no PDF.
- Check what is installed:
  ```bash
  command -v pandoc xelatex google-chrome chromium wkhtmltopdf
  ```
- The xelatex path adds break points in inline code, wraps code blocks and shrinks table type. The HTML paths get equivalent CSS (A4, 20 mm margin).

## Outputs (gitignored)

| Edition | Intermediate | PDF |
|---|---|---|
| English | `docs/manuals/tecnico/build/manual.md` (and `manual.html` on HTML engines) | `docs/manuals/tecnico/build/Gafira_Technical_Manual.pdf` |
| Spanish | `docs/manuals/tecnico/build/manual-es.md` (and `manual-es.html`) | `docs/manuals/tecnico/build/Gafira_Manual_Tecnico.pdf` |

`build/` is ignored through `.gitignore`; `check` fails if that entry is removed. Never `git add` anything from it. The PDF is deleted before each build, so a stale file never passes as new. On success the tool prints `built <path> (<bytes> bytes)`.

## Layout produced

Title page (workspace version from `Cargo.toml` and today's date), table of contents (depth 2), numbered sections, every chapter on a new page.

## Verify the result

```bash
ls -l docs/manuals/tecnico/build/*.pdf
pdfinfo docs/manuals/tecnico/build/Gafira_Technical_Manual.pdf
pdftotext -layout docs/manuals/tecnico/build/Gafira_Technical_Manual.pdf - | head -80
pdftoppm -r 60 -png -f 1 -l 4 docs/manuals/tecnico/build/Gafira_Technical_Manual.pdf <scratch-dir>/preview
```

`pdfinfo`, `pdftotext` and `pdftoppm` come from poppler. Write preview images to a scratch directory outside the repository. Look at: the title page, the table of contents, that each chapter starts on a new page, tables not clipped at the right margin, long inline code and paths wrapped, and no leftover `TRANSLATE` text. Open a few pages as images when layout matters.

## Common failures

| Symptom | Cause and fix |
|---|---|
| `check failed: N problem(s)` | Read the listed lines. Typical: missing ES file, heading or code-fence count differs, index title differs, a stub marker left, an absolute path, a broken relative link |
| `pending translation (stub)` | Translate the Spanish chapter and remove the `<!-- TRANSLATE` marker |
| `heading structure differs` / `code block count differs` | Make the Spanish heading levels and fence count equal the English file's |
| `.gitignore must list docs/manuals/tecnico/build/` | Restore the ignore entry |
| `error: pandoc is required` | Install pandoc |
| `xelatex failed` | Usually a missing LaTeX package or font; read the pandoc output above the error, or retry with `--engine chrome` |
| `no PDF produced` | No engine available or all failed; install one (xelatex, chrome or chromium, wkhtmltopdf) and read the collected messages in the error |
| Chrome engine gives an empty or blocked PDF | Sandbox or profile issue; retry, or use `--engine wkhtmltopdf` |
| Text overflows the margin | An unbreakable token (long identifier or URL) or a wide table; shorten or rephrase the source, do not edit the tool |
| Spanish accents broken | Use the xelatex or chrome engine; wkhtmltopdf may lack fonts |

Do not commit the PDFs. When they must be shared, publish them as release or CI artifacts.
