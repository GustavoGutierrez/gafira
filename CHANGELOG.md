# Changelog

All notable changes to Gafira are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- 3D objects (spec 100): import `.glb` and `.obj` models from «Importar archivo…», by dragging them onto the canvas or from Biblioteca, where a new 3D tab shows a rendered preview of each imported model. A model is an element like any other; double-click it or choose «Editar 3D» to rotate, move, scale and orbit it with gizmos on the canvas, an orientation cube, quick views (front, sides, top, bottom, back, isometric) and Soft, Studio and Dramatic lighting, then apply it as one undo step or cancel. The «Objeto 3D» panel has the same controls as numbers. Models are drawn by a CPU renderer (D-130), kept inside the `.gaf` and exported as the view their camera sees, with a transparent background.

### Changed

- Gafira is now free and open source software under the GNU General Public License, version 3 or later (`LICENSE`). The name, logo and app icon stay reserved (`TRADEMARKS.md`), and contributors have a guide (`CONTRIBUTING.md`).

## [0.2.0] - 2026-11-24

One way to choose colours everywhere, a new Layers panel, and quotes and speech bubbles.

### Added

- An eyedropper beside every colour picker of the editor: it shows a loupe of the pixels under the pointer with their code, and a click takes the colour of anything in the window.
- A colour picker of its own: a round swatch that opens a saturation square, hue and transparency bars, the hexadecimal code and a second eyedropper.
- One colour field for the text colour, a shape's fill and outline and the page background: the picker, the brand and design colours, and the preset colours as round swatches with «Ver todo» for a palette of 133 colours and gradients by group (cool, warm, monochrome, all).
- Capas: each element is a card with a grip, a preview, its name on one line, eye, lock and a «⋮» menu (copy, paste, duplicate, delete, layer order, align to page, rename, lock, hide, image as background); rows are dragged with a floating copy and an insertion line, and the list scrolls.
- Formas: a Quotes section of 30 quote marks (round, heavy, block, soft, slanted, wedge, thin, straight, angle and square, double and single, opening and closing, and round and square badges), and 12 more speech bubbles: clouds with soft puffs, curved tails and thought dots, hand-drawn bubbles, an oval with a curved tail and a double bubble.
- The quote templates use the quote shapes instead of a letter.
- A package can be uninstalled from its panel: «⋮» or a right click on its header, then confirm.

### Changed

- Every slider of the editor, the bottom-bar zoom included, has the style of «Editar imagen».
- Properties of texts, shapes and the rest scroll, and show position and size, rotation, opacity and arrange as an image does.
- The menus of the top bar are lists; Ver ticks what is on. «Tamaño» and «Notas» are hidden until they work.
- Installed packages fold under their header in their panels, closed by default and opened by a search.

### Fixed

- Signs the interface font lacks (arrows, ticks, warnings) showed as «?»; they are icons now.
- The cloud speech bubble looks like a cloud, and no tail sticks out between its puffs.
- Long names in Capas no longer break the panel.

## [0.1.0-beta.4] - 2026-11-23

A new Inicio with templates, and brand kits.

### Added

- A new Inicio: search design types and templates, shortcuts to each category of «Crear un diseño», recent projects and templates by type.
- Plantillas: browse every template by category and type, with search in Spanish or English; a template opens as a new design, fully editable.
- Plantillas in the editor: the templates that suit the page's proportions; one replaces the page's content, adds its other pages after it, and undoes in one step.
- 75 templates in Spanish and English, each with its own palette: Instagram posts (square and 4:5) and other social media, invitations, presentations, trifold brochures, business cards, certificates and diplomas, resumes, flyers, web and video. Their photos are frames you can drop a picture on; they carry the guides of their size, the folds of a brochure among them, and printed ones a 3 mm bleed.

- Template details: a click on a template shows it large with its size, «Customise this template» and similar templates.

- Marca: a brand kit is exported to a .gafkit file with all its colours, styles, fonts, files and templates, and imported in another Gafira (opening a .gafkit imports it); when a kit of that name exists, Gafira asks whether to replace it. The .gafkit type has its own icon on Linux, Windows and macOS.
- Marca: fonts are uploaded (several at once) from the Fuentes tipográficas section or a style's font list, shown in their own face, available in every font list of Gafira, and uninstalled whole by family.
- Marca: a palette can take colours from an image of the kit or of the computer; Gafira finds its main colours and the person chooses which ones to add.
- Marca: brand kits with logos, colour palettes (HEX and CMYK), text styles, uploaded fonts, photos, graphics and icons, managed on their own screen and used from the Marca panel of the editor and in the colour pickers; a design can be saved as a brand template, or a .gaf uploaded as one.

### Changed

- Dibujo: «Seleccionar» uses Gafira's own cursor icon.
- A new text box is as wide as its longest line and follows it while you write or delete; dragging a side fixes its width.
- A curved text always fits inside its box.
- Moving an element shows Gafira's move pointer.
- Asset packages (`.gafpack`): install one from Home → Install package or by opening the file; Gafira shows its thumbnail and details and checks it before installing. Its illustrations, icons, shapes, frames and PowerClips (as frames that take a photo), collages and templates appear in their sections, filterable by package, and Settings → Packages lists and uninstalls them. The `.gafpack` file type has its own icon. Large packages are checked and installed in the background with a progress bar, a few files at a time; `scripts/gafpack.py` builds valid packages from a folder.
- Formas: 173 shapes (from 48), new Abstract shapes and Hang tags sections, rows of five with arrows and «Ver todo», and dashed and dotted lines with filled circle, filled square and bar ends.
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
