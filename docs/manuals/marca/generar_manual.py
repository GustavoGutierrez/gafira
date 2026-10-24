#!/usr/bin/env python3
"""Gafira: genera un PDF maquetado desde el manual Markdown y sus SVG.

Reimplementación reproducible con WeasyPrint 68.0; el script original no
quedó guardado. Su salida no pretende ser idéntica píxel por píxel al PDF v2.

Uso:
  python generar_manual.py
  python generar_manual.py --input manual-gafira.md --output Gafira.pdf
  python generar_manual.py --html gafira-manual-de-marca-v2.html --output Gafira_v1.pdf
"""
from __future__ import annotations

import argparse
import html
import re
from pathlib import Path

import mistune
from weasyprint import HTML, CSS

ROOT = Path(__file__).resolve().parent


def get_cover_and_content(markdown: str) -> tuple[str, str]:
    """Separa encabezado editorial del cuerpo de capítulos."""
    match = re.search(r"(?m)^##\s+01\.", markdown)
    if not match:
        return "", markdown
    return markdown[:match.start()], markdown[match.start():]


def markdown_to_html(markdown: str) -> str:
    converter = mistune.create_markdown(
        escape=False, plugins=["table", "strikethrough", "url"]
    )
    return converter(markdown)


def make_html(markdown: str) -> str:
    meta_md, chapter_md = get_cover_and_content(markdown)
    meta_text = re.sub(r"!\[[^\]]*\]\([^)]*\)", "", meta_md)
    subtitle_match = re.search(r"\*\*([^*]+)\*\*", meta_text)
    subtitle = html.escape(
        subtitle_match.group(1) if subtitle_match else "Manual de marca y guía de estilo"
    )
    content = markdown_to_html(chapter_md)
    meta_extra = markdown_to_html(meta_text)
    # Todas las imágenes y SVG se resuelven contra base_url de la carpeta del .md
    return f'''<!doctype html>
<html lang="es">
<head>
  <meta charset="utf-8" />
  <title>Gafira — Manual de marca y guía de estilo</title>
</head>
<body>
  <section class="cover">
    <header><span>BRAND GUIDELINES / V2.0</span><span>gafira.com</span></header>
    <div class="cover-main">
      <div class="cover-figure">
        <img alt="Gafira" src="assets/gafira-logo-primary.svg" />
      </div>
      <div class="cover-kicker">IDENTIDAD · DISEÑO · PRODUCTO</div>
      <h1>Haz visibles<br />tus ideas<span class="accent">.</span></h1>
      <p>{subtitle}</p>
    </div>
    <footer><span>EDICIÓN 2.0</span><span>OCTUBRE 2026</span></footer>
  </section>
  <section class="preface">
    <p class="eyebrow">DOCUMENTO MAESTRO / INTRODUCCIÓN</p>
    <h1>Un lienzo donde<br/><span class="accent-text">las ideas toman forma.</span></h1>
    <p class="lead">Un manual para mantener una voz clara, una identidad consistente y una experiencia visual que pertenezca a Gafira.</p>
    <div class="brand-sample"><img src="assets/gafira-logo-primary.svg" alt="Gafira" /></div>
    <div class="source-meta">{meta_extra}</div>
  </section>
  <main class="manual">{content}</main>
</body>
</html>'''


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", default="manual-gafira.md", help="Markdown de entrada")
    parser.add_argument("--output", default="Gafira_Manual_de_Marca.pdf", help="PDF destino")
    parser.add_argument("--css", default="manual.css", help="Hoja de estilos de impresión")
    parser.add_argument("--html", help="Opcional: HTML existente (salta la conversión Markdown)")
    args = parser.parse_args()

    output = (ROOT / args.output).resolve()
    css_path = (ROOT / args.css).resolve()
    if args.html:
        input_path = (ROOT / args.html).resolve()
        html_document = input_path.read_text(encoding="utf-8")
    else:
        input_path = (ROOT / args.input).resolve()
        html_document = make_html(input_path.read_text(encoding="utf-8"))

    output.parent.mkdir(parents=True, exist_ok=True)
    HTML(string=html_document, base_url=input_path.parent.as_uri() + "/").write_pdf(
        target=str(output),
        stylesheets=[CSS(filename=str(css_path))],
        presentational_hints=True,
    )
    print(f"PDF generado: {output}")


if __name__ == "__main__":
    main()
