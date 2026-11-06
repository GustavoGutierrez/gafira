#!/usr/bin/env python3
"""Gafira technical manual tool (standard library only).

English is the source language (chapters/, index.md). The Spanish edition
mirrors it (chapters-es/, index-es.md) with the same numbers and file names.

Subcommands (all accept --lang en|es, default en):
  list                     Show the chapters in index order.
  new <slug> "<Title>"     Create the next numbered chapter in BOTH languages.
  check                    Verify the selected tree, the English/Spanish mirror,
                           paths and relative links.
  build [--engine E]       Concatenate the chapters and render a PDF into build/.

Run it from anywhere:  python3 docs/manuals/tecnico/manual.py check
"""
from __future__ import annotations

import argparse
import datetime
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
BUILD = HERE / "build"

LANGS = {
    "en": {
        "chapters": HERE / "chapters",
        "index": HERE / "index.md",
        "pdf": "Gafira_Technical_Manual.pdf",
        "title": "Gafira Technical Manual",
        "version": "Version",
        "toc": "Contents",
        "kicker": "TECHNICAL MANUAL",
        "blurb": "How Gafira is built: layers, crates and ports, the document format, the quality gates and the development method.",
        "author": "Ing. Gustavo Gutiérrez",
        "author_role": "Colombia",
        "edition": "English edition",
        "legal": "Gafira is proprietary software. This manual documents what the repository implements today and marks as Planned what the specifications name and the code does not have yet. The product intent lives in specs/; when the two disagree, the specification and the owner decisions win.",
        "template": "# {title}\n\n## Purpose\n\nState what this chapter covers and who needs it.\n\n## Content\n\n"
        "Describe what exists in the repository today. Mark anything specified but not\nimplemented as **Planned**.\n",
    },
    "es": {
        "chapters": HERE / "chapters-es",
        "index": HERE / "index-es.md",
        "pdf": "Gafira_Manual_Tecnico.pdf",
        "title": "Manual técnico de Gafira",
        "version": "Versión",
        "toc": "Contenido",
        "kicker": "MANUAL TÉCNICO",
        "blurb": "Cómo está construido Gafira: capas, crates y puertos, el formato de documento, las puertas de calidad y el método de desarrollo.",
        "author": "Ing. Gustavo Gutiérrez",
        "author_role": "Colombia",
        "edition": "Edición en español",
        "legal": "Gafira es software propietario. Este manual documenta lo que el repositorio implementa hoy y marca como Planificado lo que las especificaciones nombran y el código todavía no tiene. La intención de producto vive en specs/; cuando ambas cosas discrepan, ganan la especificación y las decisiones del titular.",
        "template": "# {title}\n\n<!-- TRANSLATE: replace this stub with the Spanish translation of the English chapter -->\n\n"
        "## Propósito\n\nIndique qué cubre este capítulo y quién lo necesita.\n\n## Contenido\n\n"
        "Describa lo que existe hoy en el repositorio. Marque como **Planificado** lo que está\nespecificado pero no implementado.\n",
    },
}
PENDING_MARK = "<!-- TRANSLATE"

ENTRY_RE = re.compile(r"^- \[(?P<title>[^\]]+)\]\((?P<path>[a-z-]+/(?P<file>\d{2}-[a-z0-9-]+\.md))\)\s*$")
SLUG_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
LINK_RE = re.compile(r"!?\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
FENCE_RE = re.compile(r"^\s*(```|~~~)")
# Absolute or user-specific paths must never appear in tracked files.
FORBIDDEN_PATHS = [
    (re.compile(r"/home/[A-Za-z0-9._-]+"), "Linux home path"),
    (re.compile(r"/Users/[A-Za-z0-9._-]+"), "macOS home path"),
    (re.compile(r"\b[A-Za-z]:\\[A-Za-z0-9._\\ -]+"), "Windows absolute path"),
    (re.compile(r"(?<![\w.])~/[A-Za-z0-9._-]+"), "home-relative path"),
    (re.compile(r"file://"), "file:// URL"),
]

# --- layout fixes for the PDF engines -------------------------------------

# Pandoc Lua filter: inline code becomes \texttt with break opportunities
# after path and identifier separators, so long paths wrap inside the margins.
LUA_FILTER = r"""
function Code(el)
  if FORMAT:match("latex") then
    local t = el.text
    local out = {}
    for ch in t:gmatch(utf8.charpattern) do
      if ch == "\\" then out[#out+1] = "\\textbackslash{}"
      elseif ch:match("^[&%%$#_{}]$") then out[#out+1] = "\\" .. ch .. "\\allowbreak{}"
      elseif ch == "~" then out[#out+1] = "\\textasciitilde{}"
      elseif ch == "^" then out[#out+1] = "\\textasciicircum{}"
      elseif ch:match("^[/%.,:;=%-%(%)%[%]<>|]$") then out[#out+1] = ch .. "\\allowbreak{}"
      else out[#out+1] = ch end
    end
    return pandoc.RawInline("latex", "\\texttt{" .. table.concat(out) .. "}")
  end
end
"""

LATEX_HEADER = r"""\usepackage{fvextra}
\usepackage{etoolbox}
\usepackage{sectsty}
\usepackage{tikz}
\usepackage{graphicx}
\usetikzlibrary{calc}
\definecolor{gafiraink}{HTML}{18243B}
\definecolor{gafiracoral}{HTML}{FF6B57}
\definecolor{gafiracanvas}{HTML}{FAFAF7}
\definecolor{gafiramint}{HTML}{3FBFA9}
\definecolor{gafiramist}{HTML}{E7EAF2}
\newfontfamily\GafiraDisplay{Inter}[UprightFont={[TESTFONTDIR/Inter-test.ttf]},
  BoldFont={[MANUALFONTDIR/Inter-SemiBold.ttf]}]
\newfontfamily\GafiraHeavy{Manrope}[UprightFont={[MANUALFONTDIR/Manrope-ExtraBold.ttf]}]
\setmainfont{Inter}[UprightFont={[TESTFONTDIR/Inter-test.ttf]},
  BoldFont={[MANUALFONTDIR/Inter-SemiBold.ttf]}]
\newfontfamily\GafiraHeading{Manrope}[UprightFont={[MANUALFONTDIR/Manrope-ExtraBold.ttf]}]
\allsectionsfont{\GafiraHeading}
% Inter's figures are wider than Latin Modern's: the class's default number box in the
% table of contents is too narrow for a four-character number such as 10.12.
\makeatletter
\renewcommand{\l@subsection}{\@dottedtocline{2}{1.5em}{2.9em}}
\makeatother
\renewcommand{\maketitle}{}
\RecustomVerbatimEnvironment{verbatim}{Verbatim}{breaklines,breakanywhere,fontsize=\footnotesize}
\setlength{\emergencystretch}{4em}
\sloppy
\pretocmd{\section}{\clearpage}{}{}
\setlength{\LTpre}{6pt}\setlength{\LTpost}{6pt}
\AtBeginEnvironment{longtable}{\footnotesize}
\setlength{\tabcolsep}{4pt}
"""

CSS = """body{font-family:'DejaVu Sans',Arial,sans-serif;font-size:10.5pt;line-height:1.45;max-width:46em;margin:auto;color:#1a1a1a;overflow-wrap:break-word;word-wrap:break-word}
h1{page-break-before:always;break-before:page;border-bottom:2px solid #ccc;padding-bottom:.2em}
table{border-collapse:collapse;width:100%;font-size:9pt;margin:1em 0;table-layout:fixed}
th,td{border:1px solid #bbb;padding:4px 6px;vertical-align:top;overflow-wrap:anywhere;word-wrap:break-word;word-break:break-word}th{background:#f0f0f0}
code,pre{font-family:'DejaVu Sans Mono',monospace;font-size:9pt;overflow-wrap:anywhere;word-wrap:break-word;word-break:break-all}
h1.title{page-break-before:avoid;break-before:avoid}
pre{background:#f6f6f6;padding:8px;white-space:pre-wrap}
@page{size:A4;margin:20mm}"""


def repo_root() -> Path:
    for parent in [HERE, *HERE.parents]:
        if (parent / "Cargo.toml").is_file():
            return parent
    return HERE


def read_index(lang: str) -> tuple[list[str], list[tuple[str, str]]]:
    """Return the raw index lines and the (title, file) entries in order."""
    index = LANGS[lang]["index"]
    if not index.is_file():
        sys.exit(f"error: missing {index.name}")
    lines = index.read_text(encoding="utf-8").splitlines()
    entries = [(m.group("title"), m.group("file")) for m in map(ENTRY_RE.match, lines) if m]
    return lines, entries


def body_lines(path: Path) -> list[str]:
    """Lines of a Markdown file outside code fences."""
    out, in_fence = [], False
    for line in path.read_text(encoding="utf-8").splitlines():
        if FENCE_RE.match(line):
            in_fence = not in_fence
            continue
        if not in_fence:
            out.append(line)
    return out


def chapter_title(path: Path) -> str | None:
    for line in path.read_text(encoding="utf-8").splitlines():
        if FENCE_RE.match(line):
            return None
        if line.startswith("# "):
            return line[2:].strip()
        if line.strip():
            return None
    return None


def cmd_list(args: argparse.Namespace) -> int:
    _, entries = read_index(args.lang)
    for title, file in entries:
        print(f"{file[:2]}  {title}  ({file})")
    return 0


def add_to_index(lang: str, lines: list[str], title: str, file: str) -> None:
    entry = f"- [{title}]({LANGS[lang]['chapters'].name}/{file})"
    last = max((i for i, l in enumerate(lines) if ENTRY_RE.match(l)), default=None)
    if last is None:
        lines += ["", entry]
    else:
        lines.insert(last + 1, entry)
    LANGS[lang]["index"].write_text("\n".join(lines) + "\n", encoding="utf-8")


def cmd_new(args: argparse.Namespace) -> int:
    if not SLUG_RE.match(args.slug):
        print("error: slug must be lowercase words joined by hyphens", file=sys.stderr)
        return 2
    indexes = {lang: read_index(lang) for lang in LANGS}
    numbers = [0]
    for lang, (_, entries) in indexes.items():
        numbers += [int(f[:2]) for _, f in entries]
        numbers += [int(p.name[:2]) for p in LANGS[lang]["chapters"].glob("[0-9][0-9]-*.md")]
    number = max(numbers) + 1
    if number > 99:
        print("error: chapter numbers are limited to 99", file=sys.stderr)
        return 2
    file = f"{number:02d}-{args.slug}.md"
    titles = {"en": args.title, "es": args.title_es or args.title}
    for lang in LANGS:
        LANGS[lang]["chapters"].mkdir(exist_ok=True)
        (LANGS[lang]["chapters"] / file).write_text(LANGS[lang]["template"].format(title=titles[lang]), encoding="utf-8")
        add_to_index(lang, indexes[lang][0], titles[lang], file)
        print(f"created {LANGS[lang]['chapters'].name}/{file} and added it to {LANGS[lang]['index'].name}")
    if not args.title_es:
        print("note: the Spanish chapter is a stub; translate it and its index title (check flags it until then)")
    return 0


def check_links(path: Path, errors: list[str]) -> None:
    in_fence = False
    for n, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if FENCE_RE.match(line):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        for target in LINK_RE.findall(re.sub(r"`[^`]*`", "", line)):
            if re.match(r"^[a-z][a-z0-9+.-]*:", target) or target.startswith("#"):
                continue
            ref = target.split("#", 1)[0]
            if ref and not (path.parent / ref).resolve().exists():
                errors.append(f"{path.relative_to(HERE)}:{n}: broken link {target}")


def check_structure(lang: str, errors: list[str]) -> None:
    cfg = LANGS[lang]
    chapters = cfg["chapters"]
    _, entries = read_index(lang)
    files = [f for _, f in entries]
    tag = f"[{lang}] "
    if not files:
        errors.append(tag + f"{cfg['index'].name} lists no chapters")
    if len(set(files)) != len(files):
        errors.append(tag + "index lists a chapter twice")
    on_disk = sorted(p.name for p in chapters.glob("*.md")) if chapters.is_dir() else []
    for f in on_disk:
        if not re.match(r"^\d{2}-[a-z0-9-]+\.md$", f):
            errors.append(tag + f"{chapters.name}/{f}: name must be NN-slug.md")
        elif f not in files:
            errors.append(tag + f"{chapters.name}/{f}: not listed in {cfg['index'].name}")
    for f in files:
        if f not in on_disk:
            errors.append(tag + f"{cfg['index'].name}: {f} does not exist")
    if files != sorted(files):
        errors.append(tag + "chapters are not in numeric order")
    numbers = [f[:2] for f in files]
    if len(set(numbers)) != len(numbers):
        errors.append(tag + "two chapters share a number")
    elif [int(n) for n in numbers] != list(range(1, len(numbers) + 1)):
        errors.append(tag + "chapter numbers are not contiguous from 01")
    for title, f in entries:
        path = chapters / f
        if path.is_file():
            h1 = chapter_title(path)
            if h1 is None:
                errors.append(tag + f"{chapters.name}/{f}: first line must be '# Title'")
            elif h1 != title:
                errors.append(tag + f"{chapters.name}/{f}: title '{h1}' differs from index '{title}'")


def scan_files(lang: str, errors: list[str]) -> None:
    cfg = LANGS[lang]
    paths = [cfg["index"], *sorted(cfg["chapters"].glob("*.md"))]
    for path in paths:
        if not path.is_file():
            continue
        for n, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            for rx, what in FORBIDDEN_PATHS:
                if rx.search(line):
                    errors.append(f"{path.relative_to(HERE)}:{n}: {what}")
        check_links(path, errors)


def check_mirror(errors: list[str]) -> None:
    """The Spanish tree must mirror the English one."""
    _, en = read_index("en")
    _, es = read_index("es")
    en_files = {f for _, f in en}
    es_files = {f for _, f in es}
    for f in sorted(en_files - es_files):
        errors.append(f"missing Spanish translation: {LANGS['es']['chapters'].name}/{f}")
    for f in sorted(es_files - en_files):
        errors.append(f"Spanish chapter without English source: {LANGS['es']['chapters'].name}/{f}")
    for f in sorted(en_files & es_files):
        a, b = LANGS["en"]["chapters"] / f, LANGS["es"]["chapters"] / f
        if not (a.is_file() and b.is_file()):
            continue
        if PENDING_MARK in b.read_text(encoding="utf-8"):
            errors.append(f"pending translation (stub): {b.relative_to(HERE)}")
            continue
        heads = lambda p: [len(l) - len(l.lstrip("#")) for l in body_lines(p) if re.match(r"^#{1,6} ", l)]
        fences = lambda p: sum(1 for l in p.read_text(encoding="utf-8").splitlines() if FENCE_RE.match(l))
        ha, hb = heads(a), heads(b)
        if ha != hb:
            errors.append(f"{b.relative_to(HERE)}: heading structure differs ({len(hb)} vs {len(ha)} in English)")
        if fences(a) != fences(b):
            errors.append(f"{b.relative_to(HERE)}: code block count differs from English")


def cmd_check(args: argparse.Namespace) -> int:
    errors: list[str] = []
    check_structure(args.lang, errors)
    for lang in LANGS:
        scan_files(lang, errors)
    check_mirror(errors)
    if not (HERE / "README.md").is_file():
        errors.append("README.md is missing")
    else:
        for n, line in enumerate((HERE / "README.md").read_text(encoding="utf-8").splitlines(), 1):
            for rx, what in FORBIDDEN_PATHS:
                if rx.search(line):
                    errors.append(f"README.md:{n}: {what}")
    gitignore = repo_root() / ".gitignore"
    ignored = gitignore.is_file() and any(
        l.strip() in {"docs/manuals/tecnico/build/", "/docs/manuals/tecnico/build/"}
        for l in gitignore.read_text(encoding="utf-8").splitlines()
    )
    if not ignored:
        errors.append(".gitignore must list docs/manuals/tecnico/build/")
    if errors:
        print("\n".join(errors), file=sys.stderr)
        print(f"check failed: {len(errors)} problem(s)", file=sys.stderr)
        return 1
    _, entries = read_index(args.lang)
    print(f"check ok [{args.lang}]: {len(entries)} chapters, English and Spanish mirror each other, "
          "no absolute paths, links resolve")
    return 0


def workspace_version() -> str:
    cargo = repo_root() / "Cargo.toml"
    if cargo.is_file():
        m = re.search(r'(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"', cargo.read_text(encoding="utf-8"))
        if m:
            return m.group(1)
    return "unversioned"


def run(cmd: list[str]) -> None:
    print("+ " + " ".join(cmd))
    subprocess.run(cmd, check=True)


def render_template(name: str, values: dict[str, str]) -> str:
    """Fill a tracked .tex template; every placeholder must be provided."""
    text = (HERE / name).read_text(encoding="utf-8")
    for key, value in values.items():
        text = text.replace("{{" + key + "}}", value)
    missing = sorted(set(re.findall(r"\{\{([A-Z_]+)\}\}", text)))
    if missing:
        sys.exit(f"error: {name} has unfilled placeholders: {', '.join(missing)}")
    return text


def cmd_build(args: argparse.Namespace) -> int:
    if cmd_check(args) != 0:
        return 1
    if not shutil.which("pandoc"):
        print("error: pandoc is required", file=sys.stderr)
        return 1
    cfg = LANGS[args.lang]
    _, entries = read_index(args.lang)
    BUILD.mkdir(exist_ok=True)
    stem = "manual" if args.lang == "en" else "manual-es"
    md = BUILD / f"{stem}.md"
    md.write_text("\n\n".join((cfg["chapters"] / f).read_text(encoding="utf-8").rstrip() + "\n"
                                         for _, f in entries), encoding="utf-8")
    (BUILD / "code-break.lua").write_text(LUA_FILTER, encoding="utf-8")
    assets = (HERE.parent / "marca" / "assets").resolve()
    (BUILD / "header.tex").write_text(
        LATEX_HEADER.replace("TESTFONTDIR", str((repo_root() / "assets" / "fonts" / "test").resolve()))
        .replace("MANUALFONTDIR", str((repo_root() / "assets" / "fonts" / "manual").resolve()))
        + f"\\graphicspath{{{{{assets}/}}}}\n", encoding="utf-8")
    today = datetime.date.today().isoformat()
    values = {
        "TITLE": cfg["title"], "KICKER": cfg["kicker"], "BLURB": cfg["blurb"],
        "VERSION": f"{cfg['version']} {workspace_version()}", "AUTHOR": cfg["author"],
        "AUTHOR_ROLE": cfg["author_role"], "LEGAL": cfg["legal"],
        "EDITION": cfg["edition"], "DATE": today,
    }
    (BUILD / "cover.tex").write_text(render_template("cover.tex", values), encoding="utf-8")
    (BUILD / "backcover.tex").write_text(render_template("backcover.tex", values), encoding="utf-8")
    css = BUILD / "manual.css"
    css.write_text(CSS, encoding="utf-8")
    pdf = BUILD / cfg["pdf"]
    pdf.unlink(missing_ok=True)
    meta = ["-M", f"title={cfg['title']}", "-M", f"subtitle={cfg['version']} {workspace_version()}",
            "-M", f"author={cfg['author']}",
            "-M", f"date={datetime.date.today().isoformat()}", "-M", f"lang={args.lang}", "-M", f"toc-title={cfg['toc']}"]
    base = ["--from", "markdown+pipe_tables", "--toc", "--toc-depth=2", "--number-sections"]
    engine, errors = args.engine, []
    if engine in ("auto", "xelatex") and shutil.which("xelatex"):
        try:
            run(["pandoc", str(md), "-o", str(pdf), *base, "--pdf-engine=xelatex", "--syntax-highlighting=none",
                 "--lua-filter", str(BUILD / "code-break.lua"), "-H", str(BUILD / "header.tex"),
                 "--include-before-body", str(BUILD / "cover.tex"),
                 "--include-after-body", str(BUILD / "backcover.tex"),
                 "-V", "papersize=a4", "-V", "geometry:margin=2.2cm", "-V", "fontsize=10pt", "-V", "colorlinks=true", *meta])
        except subprocess.CalledProcessError as e:
            errors.append(f"xelatex failed: {e}")
    elif engine == "xelatex":
        errors.append("xelatex not found")
    if not pdf.exists() and engine in ("auto", "chrome", "wkhtmltopdf"):
        html = BUILD / f"{stem}.html"
        run(["pandoc", str(md), "-s", "-o", str(html), *base, "--css", css.name,
             "--metadata", "pagetitle=" + cfg["title"], *meta])
        chrome = next((c for c in ("google-chrome", "chromium", "chromium-browser") if shutil.which(c)), None)
        if chrome and engine in ("auto", "chrome"):
            with tempfile.TemporaryDirectory() as profile:
                try:
                    run([chrome, "--headless", "--no-sandbox", "--disable-gpu", f"--user-data-dir={profile}",
                         "--no-pdf-header-footer", f"--print-to-pdf={pdf}", html.as_uri()])
                except subprocess.CalledProcessError as e:
                    errors.append(f"chrome failed: {e}")
        if not pdf.exists() and shutil.which("wkhtmltopdf") and engine in ("auto", "wkhtmltopdf"):
            try:
                run(["wkhtmltopdf", "--enable-local-file-access", str(html), str(pdf)])
            except subprocess.CalledProcessError as e:
                errors.append(f"wkhtmltopdf failed: {e}")
    if not pdf.exists() or pdf.stat().st_size == 0:
        print("error: no PDF produced. " + "; ".join(errors), file=sys.stderr)
        return 1
    print(f"built {pdf.relative_to(repo_root())} ({pdf.stat().st_size} bytes)")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)

    def add(name: str, help: str, func) -> argparse.ArgumentParser:
        p = sub.add_parser(name, help=help)
        p.add_argument("--lang", choices=sorted(LANGS), default="en", help="language edition (default en)")
        p.set_defaults(func=func)
        return p

    add("list", "show chapters in order", cmd_list)
    p_new = add("new", "create the next numbered chapter in both languages", cmd_new)
    p_new.add_argument("slug", help="lowercase-hyphenated file slug")
    p_new.add_argument("title", help="English chapter title")
    p_new.add_argument("--title-es", help="Spanish title (default: same as English; the chapter stays a flagged stub)")
    add("check", "verify tree, mirror, paths and links", cmd_check)
    p_build = add("build", "render the PDF into build/ (runs check first)", cmd_build)
    p_build.add_argument("--engine", choices=["auto", "xelatex", "chrome", "wkhtmltopdf"], default="auto")
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
