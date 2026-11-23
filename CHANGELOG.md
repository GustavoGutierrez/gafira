# Changelog

All notable changes to Gafira are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-beta.4] - 2026-11-23

A new Inicio with templates, and brand kits.

### Added

- A new Inicio: search design types and templates, shortcuts to each category of «Crear un diseño», recent projects and templates by type.
- Plantillas: browse every template by category and type, with search in Spanish or English; a template opens as a new design, fully editable.
- Plantillas in the editor: the templates that suit the page's proportions; one replaces the page's content, adds its other pages after it, and undoes in one step.
- 75 templates in Spanish and English, each with its own palette: Instagram posts (square and 4:5) and other social media, invitations, presentations, trifold brochures, business cards, certificates and diplomas, resumes, flyers, web and video. Their photos are frames you can drop a picture on; they carry the guides of their size, the folds of a brochure among them, and printed ones a 3 mm bleed.

- Template details: a click on a template shows it large with its size, «Customise this template» and similar templates.

- Marca: brand kits with logos, colour palettes (HEX and CMYK), text styles, uploaded fonts, photos, graphics and icons, managed on their own screen and used from the Marca panel of the editor and in the colour pickers; a design can be saved as a brand template, or a .gaf uploaded as one.

### Changed

- System file dialogs open in front of Gafira's window.
- The top bar shows the Gafira logo and icons for Inicio, Plantillas and Ajustes; Proyectos, not yet available, is gone.
- Ajustes: sections with icons, the language as cards and each theme with a small picture of the interface; Privacidad says when Gafira goes online.
- «Crear un diseño» opens from the shortcuts of Inicio, on the chosen category, and leads back to Inicio and to Plantillas.

## [0.1.0-beta.3] - 2026-11-22

The first version with the desktop application and with binaries for Linux and Windows.

### Added

- The editor: pages, shapes, lines, texts, images, frames, lights, freehand drawing, guides, rulers, layers, versions and undo, in Spanish and English.
- Projects in `.gaf` files, with their own icon and opened by a double click.
- Export to PNG, JPEG, WebP, AVIF, PDF, SVG, PowerPoint and Excel (tables).
- Import of images (PNG, JPEG, WebP, AVIF), SVG, PDF, Excel and CSV.
- Tables, charts and QR codes; photos and illustrations from open catalogues.
- Image editing that keeps the original: adjustments, filters, shadows, crop with shapes, background removal, resolution improvement, magic eraser and watermark removal, with models that run on the computer.
- Text effects and curved texts.
- Packages: `.deb` and AppImage for Linux; installer and portable `.zip` for Windows.

### Fixed

- Duplicating a page now reports the copied elements as changed.

### Security

- `zip` updated to 9.8 and `url` to 2.5.9 for their security advisories; a `.gaf` file with an entry name that is not text is refused.

### Notes

- Pre-release: the Windows installer is not signed, so Windows warns of an unknown publisher.

## [0.1.0-beta.1] - 2026-10-24

### Added

- Cargo workspace and CI, with the layering policy steps (V5, V11, V12, V13) and `cargo deny` license checks.
- `gafira-geom`: geometry primitives.
- `gafira-color`: color model, including `ColorTransform`.
- `gafira-doc`: document model, canonical JSON with validation on load, and `EditToken` / `DocMut` for controlled mutation.
- `gafira-edit`: skeleton with the `Command` contract and `Transaction`, an all-or-nothing composite command.
- `gafira-cli --version`.
- Product specs and the owner decisions D-01 to D-13, including the D-10 audio exception and the D-12 licensing model.
- OpenSpec changes archived: `workspace-scaffold`, `domain-color`, `domain-doc`. The `edit-history` change is in progress (D1 done).

### Notes

- Pre-release: there is no GUI, no installer and no binaries are attached.
- The API is unstable and may change without notice.
- Gafira is proprietary software; see `LICENSE`.
