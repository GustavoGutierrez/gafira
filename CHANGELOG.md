# Changelog

All notable changes to Gafira are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Document tabs (spec 111): several projects open at once, each in a tab with its file name and a mark when it has unsaved changes. Opening, creating or «Copiar y redimensionar» open a new tab; a file already open shows its tab; Inicio keeps the tabs; closing a tab with changes asks first. Ctrl+Tab and Ctrl+Shift+Tab move between tabs, Ctrl+W closes one; each tab keeps its zoom, scroll and selection.
- «Ayuda» in the editor's top bar: keyboard shortcuts (Ctrl+/), the licence of Gafira and of what it ships, the website, reporting a problem and «Acerca de».
- Nine more lights (spec 110): «Luz de luna», «Foco», «Resplandor», «Estrellas», «Polvo de estrellas», «Estela de cometa», «Aurora boreal», «Fuga de luz» and «Anillo luminoso», customisable like the others.
- Grunge shapes in Formas (spec 109): 26 brush strokes, 13 splatters and 11 claw marks, ten of them drawn by hand, as vector paths you can colour, resize and turn.
- «Redimensionar» in the editor's top bar (spec 108): search the size catalogue, pick a suggestion, a category or a custom size, and resize every page or the one in view with their content scaled and centred to fit, in one undo step, or open a resized copy of the design.
- Eight more font families, now 56: high-contrast display serifs (Abril Fatface, Yeseva One), scripts (Lobster, Pinyon Script), a rounded heavy face (Lilita One), a stencil (Black Ops One), a textured face (Rubik Dirt) and a marker (Permanent Marker).

### Changed

- Fotos: the lists of the filters cut their text and the photos keep two whole columns with Openverse too.
- The side panel of the tools keeps a fixed width (Fotos no longer widens it), and both side panels fold away with a handle on their edge; Ctrl+. hides or shows both for the canvas alone, and Ctrl+2 («Ajustar al ancho», also in Ver and the zoom menu) makes the page as wide as the canvas.
- «Luces y efectos» is now «Efectos de luz», and its list shows each light on a plain card (spec 110).
- «Combinaciones de fuentes» redesigned: forty-two compositions of two or three texts with display, script, brush, blackletter, sketched, stencil and outlined families, letter spacing, line height, curves and effects, some with a full headline, subtitle and paragraph to rewrite. The panel shows them in two columns, drawn as the canvas draws them, and loads more as it scrolls. Added to a page, each keeps the alignment and proportions its card shows.

## [0.5.0] - 2026-11-26

Bulk create from a spreadsheet, styles on chosen words, a right-click menu with a style painter, and fifteen more fonts.

### Added

- Bulk create (spec 104): a «Crear en lote» tab connects a CSV, Excel or pasted table to a design. Its columns become fields written in the texts as `{{field}}`, by dragging a card, with buttons, from the right-click menu or with autocomplete while typing `{{`. The «Revisar» step shows each row as it will look, also enlarged, and «Exportar en lote» writes one design per row in any export format, with its options: one PDF with every row, a PDF per row, images, and an editable `.gaf`. Progress, cancel and the result show in a card like the one of «Exportar». A spreadsheet dropped on the panel loads it, and the table is kept inside the `.gaf`.
- Styles on chosen words (spec 106): choose some letters while writing and change their colour, gradient, font, size, bold or italic from Propiedades and «Fuente», keeping the rest of the box. They are drawn, written and exported in their own style.
- Right-click menu (spec 105): every element on the canvas has a menu with cut, copy, paste, duplicate, delete, order, align, lock, hide, group and ungroup, and «Conectar datos» on texts. «Copiar estilo», «Copiar efectos» and «Copiar propiedades» turn the pointer into a painter: the next click pastes them on another element (Ctrl+Shift+C / Ctrl+Shift+V).
- Fifteen more font families (OFL), now 48: signatures (Mrs Saint Delafield, Herr Von Muellerhoff, Monsieur La Doulaise, Meddon), blackletter for diplomas (UnifrakturMaguntia, UnifrakturCook, Pirata One), brush scripts (Mr Dafoe, Damion) and display faces (Londrina Outline, Rubik Bubbles, Sniglet, Cabin Sketch, Michroma, Rye).
- Ten more font pairings with text effects (neon, gold, retro, sticker, screen, outline, echo, label, laser, night), shown with their effects in the Texto panel.

### Changed

- Font pairings come in as wide as their words, and grow sideways as they are written.
- Menus show their shortcuts and arrows on the right edge, and a long label of the rail takes two lines.
- The data grid of charts is shared with bulk create.

## [0.4.0] - 2026-11-25

Present your designs full screen, see all the pages in a grid, and six lights for 3D objects.

### Added

- Icons (spec 103): Elementos → Iconos now includes about 5,100 Tabler icons (MIT) in line and filled styles, grouped in 40 categories with a search in Spanish and English; an icon is added as vector shapes that scale and take any colour. Icons from installed packs stay under «De tus paquetes».
- Biblioteca: the details of a 3D model show its preview, and the file menu has the list style of the other menus.
- 3D lighting: six presets — Studio (brighter, now the default), Warm, Cool, Dark (formerly Dramatic; older files open with it), Soft and Natural — each shown as a small picture of the model under that light.
- Grid view (spec 102): a button in the bottom bar shows every page as a thumbnail; pages are picked, reordered by dragging (the others make room and a gap shows where it lands), added with a «+» between two pages, duplicated, deleted and hidden; each page has a «⋯» menu with its title, size and these actions plus lock. Closing the grid shows the page chosen.
- Present full screen (spec 101): a button in the bottom bar and Ctrl+Alt+P show the pages one at a time on the whole screen. The arrow keys, Space and a click move between them, Esc leaves, and the bar with the page number and the arrows hides when the pointer leaves the bottom edge. Pages change with a cross-fade, and a laser pointer (L) and drawing tools (pen, marker, highlighter, eraser, six colours) work over the slides.

## [0.3.0] - 2026-11-25

3D objects in your designs, and Gafira becomes free and open source software.

### Added

- 3D objects (spec 100): import `.glb` and `.obj` models from «Importar archivo…», by dragging them onto the canvas or from Biblioteca, where a new 3D tab shows a rendered preview of each imported model. A model is an element like any other; double-click it or choose «Editar 3D» to rotate, move and orbit it with gizmos with volume on the canvas, resize it in the design with its frame handles without distorting it, an orientation cube, quick views (front, sides, top, bottom, back, isometric) and Soft, Studio and Dramatic lighting, then apply it as one undo step or cancel. The «Objeto 3D» panel has the same controls as numbers. Models are drawn by a CPU renderer (D-130), kept inside the `.gaf` and exported as the view their camera sees, with a transparent background.

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
