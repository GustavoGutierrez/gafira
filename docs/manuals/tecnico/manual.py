#!/usr/bin/env python3
"""Gafira technical manual tool (standard library only).

Subcommands:
  list                     Show the chapters in index order.
  new <slug> "<Title>"     Create the next numbered chapter and add it to the index.
  check                    Verify index and chapters, paths and relative links.
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
CHAPTERS = HERE / "chapters"
INDEX = HERE / "index.md"
BUILD = HERE / "build"
PDF_NAME = "Gafira_Technical_Manual.pdf"
MANUAL_TITLE = "Gafira Technical Manual"

ENTRY_RE = re.compile(r"^- \[(?P<title>[^\]]+)\]\((?P<path>chapters/(?P<file>\d{2}-[a-z0-9-]+\.md))\)\s*$")
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
TEMPLATE = """# {title}

## Purpose

State what this chapter covers and who needs it.

## Content

Describe what exists in the repository today. Mark anything specified but not
implemented as **Planned**.
"""


def repo_root() -> Path:
    for parent in [HERE, *HERE.parents]:
        if (parent / "Cargo.toml").is_file():
            return parent
    return HERE


def read_index() -> tuple[list[str], list[tuple[str, str]]]:
    """Return the raw index lines and the (title, file) entries in order."""
    if not INDEX.is_file():
        sys.exit(f"error: missing {INDEX.name}")
    lines = INDEX.read_text(encoding="utf-8").splitlines()
    entries = []
    for line in lines:
        m = ENTRY_RE.match(line)
        if m:
            entries.append((m.group("title"), m.group("file")))
    return lines, entries


def chapter_title(path: Path) -> str | None:
    for line in path.read_text(encoding="utf-8").splitlines():
        if FENCE_RE.match(line):
            return None
        if line.startswith("# "):
            return line[2:].strip()
        if line.strip():
            return None
    return None


def cmd_list(_: argparse.Namespace) -> int:
    _, entries = read_index()
    for title, file in entries:
        print(f"{file[:2]}  {title}  ({file})")
    return 0


def cmd_new(args: argparse.Namespace) -> int:
    if not SLUG_RE.match(args.slug):
        print("error: slug must be lowercase words joined by hyphens", file=sys.stderr)
        return 2
    lines, entries = read_index()
    on_disk = sorted(p.name for p in CHAPTERS.glob("[0-9][0-9]-*.md"))
    numbers = [int(f[:2]) for f in on_disk] + [int(f[:2]) for _, f in entries]
    number = max(numbers, default=0) + 1
    if number > 99:
        print("error: chapter numbers are limited to 99", file=sys.stderr)
        return 2
    file = f"{number:02d}-{args.slug}.md"
    target = CHAPTERS / file
    if target.exists():
        print(f"error: {target.name} already exists", file=sys.stderr)
        return 2
    CHAPTERS.mkdir(exist_ok=True)
    target.write_text(TEMPLATE.format(title=args.title), encoding="utf-8")
    entry = f"- [{args.title}](chapters/{file})"
    last = max((i for i, l in enumerate(lines) if ENTRY_RE.match(l)), default=None)
    if last is None:
        lines += ["", entry]
    else:
        lines.insert(last + 1, entry)
    INDEX.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"created chapters/{file} and added it to {INDEX.name}")
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


def cmd_check(_: argparse.Namespace) -> int:
    errors: list[str] = []
    lines, entries = read_index()
    files = [f for _, f in entries]
    if not files:
        errors.append("index.md lists no chapters")
    if len(set(files)) != len(files):
        errors.append("index.md lists a chapter twice")
    on_disk = sorted(p.name for p in CHAPTERS.glob("*.md"))
    for f in on_disk:
        if not re.match(r"^\d{2}-[a-z0-9-]+\.md$", f):
            errors.append(f"chapters/{f}: name must be NN-slug.md")
        elif f not in files:
            errors.append(f"chapters/{f}: not listed in index.md")
    for f in files:
        if f not in on_disk:
            errors.append(f"index.md: {f} does not exist")
    if files != sorted(files):
        errors.append("index.md: chapters are not in numeric order")
    numbers = [f[:2] for f in files]
    if len(set(numbers)) != len(numbers):
        errors.append("index.md: two chapters share a number")
    elif [int(n) for n in numbers] != list(range(1, len(numbers) + 1)):
        errors.append("index.md: chapter numbers are not contiguous from 01")
    for title, f in entries:
        path = CHAPTERS / f
        if path.is_file():
            h1 = chapter_title(path)
            if h1 is None:
                errors.append(f"chapters/{f}: first line must be '# Title'")
            elif h1 != title:
                errors.append(f"chapters/{f}: title '{h1}' differs from index '{title}'")
    scanned = [INDEX, HERE / "README.md", Path(__file__).resolve(), *sorted(CHAPTERS.glob("*.md"))]
    for path in scanned:
        if not path.is_file():
            if path.name == "README.md":
                errors.append("README.md is missing")
            continue
        for n, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if path == Path(__file__).resolve():
                continue  # the patterns above would match themselves
            for rx, what in FORBIDDEN_PATHS:
                if rx.search(line):
                    errors.append(f"{path.relative_to(HERE)}:{n}: {what}")
        if path.suffix == ".md":
            check_links(path, errors)
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
    print(f"check ok: {len(files)} chapters, index consistent, no absolute paths, links resolve")
    return 0


def workspace_version() -> str:
    cargo = repo_root() / "Cargo.toml"
    if cargo.is_file():
        m = re.search(r'(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"', cargo.read_text(encoding="utf-8"))
        if m:
            return m.group(1)
    return "unversioned"


def combined_markdown(entries: list[tuple[str, str]]) -> str:
    parts = []
    for _, f in entries:
        parts.append((CHAPTERS / f).read_text(encoding="utf-8").rstrip() + "\n")
    return "\n\\newpage\n\n".join(parts)


def run(cmd: list[str], cwd: Path | None = None) -> None:
    print("+ " + " ".join(cmd))
    subprocess.run(cmd, check=True, cwd=cwd)


CSS = """body{font-family:'DejaVu Sans',Arial,sans-serif;font-size:10.5pt;line-height:1.45;max-width:46em;margin:auto;color:#1a1a1a}
h1{page-break-before:always;border-bottom:2px solid #ccc;padding-bottom:.2em}
table{border-collapse:collapse;width:100%;font-size:9pt;margin:1em 0}
th,td{border:1px solid #bbb;padding:4px 6px;vertical-align:top}th{background:#f0f0f0}
code,pre{font-family:'DejaVu Sans Mono',monospace;font-size:9pt}pre{background:#f6f6f6;padding:8px;overflow-wrap:anywhere;white-space:pre-wrap}
@page{size:A4;margin:20mm}"""


def cmd_build(args: argparse.Namespace) -> int:
    if cmd_check(args) != 0:
        return 1
    if not shutil.which("pandoc"):
        print("error: pandoc is required", file=sys.stderr)
        return 1
    _, entries = read_index()
    BUILD.mkdir(exist_ok=True)
    version = workspace_version()
    today = datetime.date.today().isoformat()
    md = BUILD / "manual.md"
    md.write_text(combined_markdown(entries), encoding="utf-8")
    pdf = BUILD / PDF_NAME
    pdf.unlink(missing_ok=True)
    meta = ["-M", f"title={MANUAL_TITLE}", "-M", f"subtitle=Version {version}", "-M", f"date={today}", "-M", "lang=en"]
    engine = args.engine
    errors = []
    if engine in ("auto", "xelatex") and shutil.which("xelatex"):
        try:
            run(["pandoc", str(md), "-o", str(pdf), "--from", "markdown+pipe_tables", "--pdf-engine=xelatex",
                 "--toc", "--toc-depth=2", "--number-sections", "-V", "geometry:margin=2.2cm",
                 "-V", "fontsize=10pt", "-V", "colorlinks=true", *meta])
        except subprocess.CalledProcessError as e:
            errors.append(f"xelatex failed: {e}")
    elif engine == "xelatex":
        errors.append("xelatex not found")
    if not pdf.exists() and engine in ("auto", "chrome", "wkhtmltopdf"):
        html = BUILD / "manual.html"
        css = BUILD / "manual.css"
        css.write_text(CSS, encoding="utf-8")
        run(["pandoc", str(md), "-s", "-o", str(html), "--from", "markdown+pipe_tables", "--toc", "--toc-depth=2",
             "--number-sections", "--css", css.name, "--metadata", "pagetitle=" + MANUAL_TITLE, *meta])
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
    sub.add_parser("list", help="show chapters in order").set_defaults(func=cmd_list)
    p_new = sub.add_parser("new", help="create the next numbered chapter")
    p_new.add_argument("slug", help="lowercase-hyphenated file slug")
    p_new.add_argument("title", help="chapter title")
    p_new.set_defaults(func=cmd_new)
    sub.add_parser("check", help="verify index, chapters, paths and links").set_defaults(func=cmd_check)
    p_build = sub.add_parser("build", help="render the PDF into build/ (runs check first)")
    p_build.add_argument("--engine", choices=["auto", "xelatex", "chrome", "wkhtmltopdf"], default="auto")
    p_build.set_defaults(func=cmd_build)
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
