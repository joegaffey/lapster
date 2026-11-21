# Third-Party Notices

## track-ed

- Source: https://github.com/joegaffey/track-ed
- Used in: `web/track-ed-src/` (pinned submodule) and the built site under
  `web/track-ed/`
- License: MIT OR Apache-2.0
- Purpose: Browser-first venue editor hosted by Lapster; its Test drive action
  compiles the current export through the browser-side `track-pack` wasm and
  opens the drive page with a locally stored pack.

track-ed bundles these runtime dependencies with compatible licenses:

| Package | Version | License | Source |
| --- | --- | --- | --- |
| `clipper2-ts` | 2.0.1-19 | BSL-1.0 | https://github.com/countertype/clipper2-ts |
| `earcut` | 3.2.4 | ISC | https://github.com/mapbox/earcut |
| `poly2tri` | 1.5.0 | BSD-3-Clause | https://github.com/r3mi/poly2tri.js |
| `polygon-clipping` | 0.15.7 | MIT | https://github.com/mfogel/polygon-clipping |
| `three` | 0.187.0 | MIT | https://github.com/mrdoob/three.js |

The track-ed notices and license files are copied into `dist/web/track-ed/` by
`web/scripts/build-pages.sh`. Full license texts for the bundled packages are
available with their upstream distributions.

## Engine_Sim

- Source: https://github.com/YigitSalihEmecen/Engine_Sim
- Vendored commit: `c3ff61031050f1fc280a864f4bb445f30aeaa0a3`
- Used in: `web/src/audio/engine-sim/`
- License: MIT (declared in the upstream README and `package.json`; upstream did
  not include a license file at vendoring time)

The vendored source is used as a runtime browser audio library. No upstream
sample MP3s, images, or other assets are copied.

MIT License

Copyright (c) Yigit Salih Emecen

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

## webhid-wheel

- Source: https://github.com/joegaffey/webhid-wheel
- Vendored version: `0.5.0` (`1e20fba`)
- Used in: `web/src/vendor/webhid-wheel/`
- License: MIT
- Purpose: WebHID wheel and generic HID input, multi-device mapping, setup profiles,
  constant-force FFB, and the embeddable configuration panel.

MIT License

Copyright (c) Joe Gaffey

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

## Stylized Components

- Source: https://github.com/cortiz2894/stylized-components
- Reference commit: `1cece0a296b29299d1380c34feb5c158b2f04500`
- Used in: `crates/sim-render/src/grass.wgsl` and `crates/sim-render/src/grass.rs`
- License: MIT
- Purpose: The instanced grass blade geometry, wind movement, color gradient, procedural
  patches, and back-lit translucency were adapted from the upstream GrassField shader for
  the Lapster WGSL forward renderer.

MIT License

Copyright (c) 2026 Christian Ortiz (Cortiz)

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

## Pixel Pine Tree Assets

- Source: https://opengameart.org/content/pixel-pine-tree-assets-10-pack
- Author: pistachio
- Used in: `fixtures/sources/trees/iso_pines.png`,
  `crates/sim-render/assets/tree_cards.png`, and
  `crates/sim-render/assets/tree_cards.rgba`
- License: CC0 1.0 Universal
  (https://creativecommons.org/publicdomain/zero/1.0/)
- Purpose: Source art for the track editor's cross-quad conifer cards.

## Drift Tracks

- Source: https://sketchfab.com/3d-models/drift-tracks-90a78772873b462aab3ed5fc5fc0c65e
- Author: The-Third-Dimension (https://sketchfab.com/muneeb.changezi1)
- Used in: `fixtures/tracks/drift-sketchfab/`
- License: Creative Commons Attribution 4.0 (CC BY 4.0)
  (https://creativecommons.org/licenses/by/4.0/)
- Purpose: Drift course visual geometry, embedded textures, and collision source. The baked
  pack selects the drivable meshes and barriers described in `track.json`.

## Speed Dreams cars

- **Deckard Conejo RR (2010 Chevrolet Camaro SS)**
  - Source: https://forge.a-lec.org/speed-dreams/sc-deckard-conejo
  - Source commit: `ba52857eac4d415510685403e06263de67bffee9`
  - Authors: Tom_D3 (original model), W.E.C (2025 3D rework), Chad Phillips
    (interior parts), June Ravenmoon (skins; 2025 rework by W.E.C).
  - Used in: `fixtures/sources/cars/sc-deckard-conejo/` and
    `fixtures/cars/sc-deckard-conejo/`.
  - License: Free Art License 1.3
    (https://artlibre.org/licence/lal/en/).
- **Guthrie Eagle 67GP**
  - Source: https://forge.a-lec.org/speed-dreams/67gp-guthrie-eagle
  - Source commit: `01c82922ba16697e1a7d4766013ebadea26bd72c`
  - Author: Copyright 2018 CHAD (Chad Phillips).
  - Used in: `fixtures/sources/cars/67gp-guthrie-eagle/` and
    `fixtures/cars/67gp-guthrie-eagle/`.
  - License: Free Art License 1.3
    (https://artlibre.org/licence/lal/en/).
- **Boxer 96 Carson Tuners Edition wheel**
  - Source: https://forge.a-lec.org/speed-dreams/sc-carson-96
  - Source commit: `df202bcb9df324384a4ef209420c104e689b2246`
  - Authors/copyright: 2002 Eric Espie, Jean-Christophe Durieu; 2005 Olaf
    Sassnick, Bernhard Wymann; 2007 Bernhard Wymann; 2009 Jean-Philippe Meuret;
    2011, 2013, 2024 W.E.C; 2011 Eckhard M. Jäger; 2021 Chad Phillips.
  - Used in: `fixtures/sources/cars/sc-deckard-conejo/wheel/`.
  - License: Free Art License 1.3
    (https://artlibre.org/licence/lal/en/).

The complete Free Art License text is in
`fixtures/sources/cars/sc-deckard-conejo/LICENSE-FAL-1.3.txt`; the Guthrie Eagle
source keeps its own copy at
`fixtures/sources/cars/67gp-guthrie-eagle/LICENSE-FAL-1.3.txt`. Lapster's
modifications are listed in the source README and in
`docs/speed-dreams-pipeline.md`.

Chevrolet, Camaro, Porsche, and 911/996 are trademarks of their respective
owners. Lapster does not imply manufacturer affiliation.
