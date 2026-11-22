# Third-party notices

Notices for third-party material bundled with Gafira. Each file is also registered, with its hash and origin, in `assets/LICENSES.toml` (ARQ-75).

## Phosphor Icons

- Version: `@phosphor-icons/core` 2.1.1 (npm), <https://github.com/phosphor-icons/core>.
- Files: `assets/icons/phosphor/regular/` and `assets/icons/phosphor/fill/`, unmodified; converted to geometry at build time (spec 037, D-87).
- License: MIT, copied in `assets/icons/phosphor/LICENSE`.

```text
MIT License

Copyright (c) 2023 Phosphor Icons

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Font library (spec 062)

Thirty-three families from [google/fonts](https://github.com/google/fonts), commit `cc5661d43cd4f9aad5567033a8e15ca28dc35c86`, each with its licence text next to its files in `assets/fonts/library/<family>/`: Open Sans, Montserrat, Poppins, DM Sans, Roboto, Lato, Inter, Manrope, EB Garamond, Source Serif 4, Anton, Pacifico, Arimo, League Spartan, Archivo Black, Questrial, Bebas Neue, Alice, Bree Serif, Shrikhand, Chau Philomene One, Belleza, Cormorant Garamond, Chewy, DM Serif Display, Kaushan Script, Satisfy, Alex Brush, Dancing Script, Allura, Great Vibes, Merienda and Courgette. Chewy and Satisfy are under the Apache License 2.0 (their texts in `assets/fonts/library/chewy/LICENSE.txt` and `assets/fonts/library/satisfy/LICENSE.txt`); the others under the SIL Open Font License 1.1. The static files of the earlier variable families are instances at weights 400 and 700 (fontTools); Dancing Script and Merienda are their unmodified variable files, renamed `-Regular.ttf` and used at their default weight, 400; the others are unmodified. `assets/LICENSES.toml` records the origin, authorship and hash of every file.

## Open Peeps and Open Doodles (spec 072)

- Illustrations by Pablo Stanley: 93 characters of Open Peeps (<https://www.openpeeps.com>) in `assets/illustrations/openpeeps/` and 33 drawings of Open Doodles (<https://www.opendoodles.com>) in `assets/illustrations/opendoodles/`, as published on their sites on 2026-11-21, unmodified (Open Doodles file names in lower case).
- License: CC0 1.0 Universal, copied in each folder as `LICENSE.txt`. No attribution is required; it is given here as a courtesy.

## GridCraft

- Source: GridCraft (MIT OR Apache-2.0; used under MIT).
- Use: the column letters, the stored column widths, the `_xHHHH_` escapes and the interning of styles of `crates/gafira-io-xlsx`, and the rules of the clipboard and of numbers written as text of `crates/gafira-table`, are adapted from it (spec 081). No file, asset or mark of it is bundled.
- License: MIT.

```text
MIT License

Copyright (c) 2026 ArtCraft Team and the GridCraft contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
