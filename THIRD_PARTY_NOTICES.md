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

Fifteen more families from the same repository, commit `5026800714d07231937d92eb4cceabf06e05d74e`, all under the SIL Open Font License 1.1 with their `OFL.txt` next to their files and all unmodified: Londrina Outline, Rubik Bubbles, Sniglet, Cabin Sketch, Michroma, Rye, Mr Dafoe, Damion, Mrs Saint Delafield, Herr Von Muellerhoff, Monsieur La Doulaise, Meddon, UnifrakturMaguntia, UnifrakturCook and Pirata One. UnifrakturCook ships only its bold file, used as the family's single face.

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

## Image models (spec 084)

- U²-Net (`assets/models/u2netp.onnx`, unmodified): Copyright the U²-Net authors (<https://github.com/xuebinqin/U-2-Net>), Apache License 2.0, copied in `assets/models/LICENSE-u2net.txt`.
- Real-ESRGAN (`assets/models/realesr-general-x4v3.onnx`, the official weights exported to ONNX with `tools/export_realesr.py`): <https://github.com/xinntao/Real-ESRGAN>, BSD 3-Clause.
- Downloaded only when the person asks, and not distributed with Gafira: U²-Net and U²-Net Human (Apache-2.0), IS-Net (Apache-2.0, <https://github.com/xuebinqin/DIS>), Real-ESRGAN x4plus (BSD-3-Clause) and ONNX Runtime (MIT, <https://github.com/microsoft/onnxruntime>).

```text
BSD 3-Clause License

Copyright (c) 2021, Xintao Wang
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its
   contributors may be used to endorse or promote products derived from
   this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```
