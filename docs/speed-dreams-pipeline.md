# Speed Dreams Car Pipeline

This document records how Lapster imports Speed Dreams cars, what the Deckard
Conejo integration currently supports, and what is still missing before the
pipeline can be called a general Speed Dreams importer.

The workflow is deliberately tool-assisted: Assimp converts the AC3D `.acc`
source to glTF, glTF Transform assembles the body and wheels, and Lapster's
existing `asset-compiler bake-glb` produces the runtime pack.

## Current status

Implemented for `sc-deckard-conejo`:

- Body, interior, glass, and four wheels exported from Speed Dreams sources.
- Speed Dreams `+X` forward converted to Lapster `-Z` forward.
- Suspension-derived body lift of 0.21 m.
- Exterior and interior textures baked into the runtime pack.
- Interior mesh separated from the exterior so cockpit mode can disable
  exterior outlines.
- Brake and reverse lamp meshes split for runtime emissive lighting.
- Four wheel nodes with per-axle radii and widths and correct right-side
  orientation.
- Car catalog entry in the menu, Garage, and Drive.
- Per-car engine audio: `rotary2` for the RX-7, `v8cross` for the Deckard
  Conejo.
- Source XML mapped into `CarPhysics` for mass, engine, gearbox, brakes,
  steering, suspension, tire mu, and aero.
- Source assets, license text, attribution, reproducible build script, baked
  pack, and validation commands are checked in.

Not yet implemented:

- A generic Speed Dreams XML-to-physics path for arbitrary cars; the current
  mapping is wired for the Deckard section layout.
- Generic cockpit layout, steering-wheel, driver, and source gauge import.
- A standalone Speed Dreams asset importer inside `asset-compiler`.
- Automatic light classification and headlights.
- Per-car source metadata in the pack manifest.

## Source assets

Tracked source: `fixtures/sources/cars/sc-deckard-conejo/`

- `body/sc-deckard-conejo.acc`: Speed Dreams body model.
- `body/sc-deckard-conejo.xml`: car specification, physics, lights, sound, and
  cockpit metadata.
- `body/sc-deckard-conejo.png`: exterior texture.
- `body/sc-deckard-conejo-int.png`: interior texture.
- `body/sc-deckard-conejo-preview.jpg`: upstream preview.
- `body/readme.txt`: original author/license note.
- `wheel/wheel0.acc`: donor wheel from the Boxer 96 Carson Tuners Edition.
- `wheel/wheel3d.png`: donor wheel texture.
- `LICENSE-FAL-1.3.txt`: Free Art License text.

The Deckard Conejo source package does not include wheel geometry, so the import
uses a wheel from the Speed Dreams Boxer 96 Carson Tuners Edition. Attribution
for both cars is in the source README and in `THIRD_PARTY_NOTICES.md`.

### Provenance

| Asset | Repository | Commit |
| --- | --- | --- |
| Deckard Conejo RR | https://forge.a-lec.org/speed-dreams/sc-deckard-conejo | `ba52857eac4d415510685403e06263de67bffee9` |
| Boxer 96 Carson wheel | https://forge.a-lec.org/speed-dreams/sc-carson-96 | `df202bcb9df324384a4ef209420c104e689b2246` |

Both assets are Free Art License 1.3. FAL requires attribution, a pointer to the
original source, identification of modifications, and redistribution of modified
copies under FAL or a compatible license. The local source README lists the
Lapster modifications.

## Build

Prerequisites:

- Assimp 5.3.x or 6.0.x command-line tools.
- Node 24+.
- Rust/Cargo with the repository's normal toolchain.

```bash
npm install --prefix tools/speed-dreams
tools/speed-dreams/build-car.sh \
  fixtures/sources/cars/sc-deckard-conejo \
  fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima
```

The output filename determines the pack ID. Keep
`sc-deckard-conejo.sima` named exactly that; otherwise `asset-compiler` will
derive a different manifest ID.

For this environment Assimp was available from an extracted package under
`/tmp`; a normal system install is preferable:

```bash
sudo apt install assimp-utils
# or
brew install assimp
```

Validate:

```bash
cargo run -p asset-compiler -- inspect fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima
cargo run -p asset-compiler -- validate fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima
```

## Guthrie Eagle 67GP

`67gp-guthrie-eagle` is the second car through the pipeline and the first built
from a per-car `import.json`. The config supplies the body, XML, and a single
wheel model plus the initial body lift, glass-name patterns, and the front/rear
axle, half-track, radius, and width values. `build-car.sh` and `assemble.mjs`
read the config, so the import no longer hardcodes the Deckard Conejo geometry.

- Source: `fixtures/sources/cars/67gp-guthrie-eagle/`.
- Config: `fixtures/sources/cars/67gp-guthrie-eagle/import.json`.
- Initial body lift: 0.104 m.
- Glass nodes: `WIGLASS*`.
- Baked pack: `fixtures/cars/67gp-guthrie-eagle/67gp-guthrie-eagle.sima`.

Rebuild:

```bash
tools/speed-dreams/build-car.sh \
  fixtures/sources/cars/67gp-guthrie-eagle \
  fixtures/cars/67gp-guthrie-eagle/67gp-guthrie-eagle.sima
```

The pack uses Assimp 6.0.4. That version triangulates the assembled Guthrie pack
to 19,328 triangles; the same version builds the Deckard Conejo as 16,322 triangles instead
of the 13,058 in the committed 5.3.x pack. Mesh bounds, vertex counts, materials,
and XML physics match on the Deckard, so the triangulation difference does not
change the runtime asset. The Guthrie source is staged from commit
`01c82922ba16697e1a7d4766013ebadea26bd72c`.

Integration findings:

- The XML parser path covers all six 67GP source files. The remaining five need
  fixtures and tests, not new parser sections.
- The accepted import reuses one wheel model at all four corners. Per-corner
  `wheel1..3.acc` models and right/left flips were tried and reverted.
- Wheel meshes use flat shading with no rim light and no outline; body and track
  keep the toon/rim treatment.
- Open-wheel cockpit cameras use a centered, lower shot and a single centered
  synthetic RPM dial. The source interior/dashboard is kept.
- Adding a car updates hardcoded catalog tests and project-qualified Playwright
  snapshots. The garage loads all catalog packs, so the remaining five need a
  loading strategy before they are added.
- Full detail: `docs/features/67gp-guthrie-eagle-mvp.md`.

## GenkiGrip wheel normal repair

The four 309-vertex GenkiGrip wheels (`67gp-guthrie-eagle`,
`67gp-kuiper-t81`, `67gp-mango-ms7`, `67gp-upping-ut19`) were baked with
Assimp normals that included zero-length vertex normals and reversed-winding
duplicate/zero-area triangles. The flat wheel-shading path then showed dark
rim wedges and facets.

`tools/speed-dreams/assemble.mjs` now restores the AC3D source normals and
drops the reversed-winding duplicate and zero-area triangles before joining the
wheel mesh. Rebuild the four packs with `tools/speed-dreams/build-car.sh` and
validate them with `asset-compiler validate`. The normal audit after the repair
is 0 flipped / 0 zero-length normals per wheel, down from 254 flipped and 480
zero-normal triangles across 850 triangles. The repaired wheel is inspectable
in the **Wheel Lab** under the launcher Tools section.

The 500-vertex Legspin wheel (`67gp-cavallo-123`, `67gp-murasama-37a`) has
correct normals in the current packs. The bake now also repairs missing or
zero-length Assimp normals for this wheel, so future rebuilds do not bake zero
normals. A dry-run rebuild of Cavallo and Murasama audits 0 flipped / 0 zero
normals per wheel with matching geometry, wheel metadata, and texture hashes.
The current packs are intentionally left unchanged until the Legspin inner-face
work needs a rebuild.

## Conversion stages

The commands below show the original Deckard-specific invocation. The current
scripts read `import.json` for the body, XML, wheel model, lift, glass, and
axle geometry; see the Guthrie section above and
`tools/speed-dreams/README.md`.

### 1. Assimp export

`tools/speed-dreams/build-car.sh` runs:

```bash
assimp export body/sc-deckard-conejo.acc body.glb -f glb2 -embtex
assimp export wheel/wheel0.acc wheel.glb -f glb2 -embtex
```

`-embtex` embeds the referenced PNG textures in the GLB so the next stage is
self-contained.

### 2. glTF assembly

`tools/speed-dreams/assemble.mjs`:

- deduplicates, flattens, joins, and prunes the wheel model;
- assigns a transparent `Glass` material to `GLASHOUSE` and `FLGT*` nodes;
- disposes body root child nodes matching the optional `exclude_nodes` patterns
  before glass assignment and body rotation;
- pre-rotates the body `-90°` around Y;
- lifts the body by `0.21 m`;
- adds `WHEEL_FL`, `WHEEL_FR`, `WHEEL_RL`, and `WHEEL_RR` nodes;
- scales wheels to the source radii and widths;
- rotates right-side wheels `180°` around Y;
- leaves the assembly under the normal GLB importer's `180°` yaw.

Speed Dreams uses `+X` forward, `+Y` up, `+Z` right. Lapster uses `-Z` forward,
`+Y` up, `+X` right. The pre-rotation and the existing GLB importer yaw produce
the final frame:

```text
Speed Dreams +X forward
        -> pre-rotate -90° Y in Assimp output
        -> GLB importer yaw 180°
        -> Lapster -Z forward
```

### 3. Pack bake

```bash
cargo run -p asset-compiler -- bake-glb assembled.glb sc-deckard-conejo.sima --textures
```

`--textures` imports base-colour images, groups body geometry by source texture
size, downsamples the body to 1024 px when the source is at least 2048 px,
downsamples interior and wheel textures to 512 px, and keeps the light-object
vertex colours needed for brake/reverse classification.

The manifest ID comes from the GLB filename stem. The build script names the
temporary assembled GLB after the output `.sima` to preserve
`sc-deckard-conejo`.

## Geometry constants

These values come from `sc-deckard-conejo.xml` and the Speed Dreams static
suspension setup.

| Value | Value |
| --- | --- |
| Body length | 4.840 m |
| Body width | 1.920 m |
| Body height | 1.380 m |
| Mass | 1,700 kg |
| CG height | 0.30 m |
| Front axle | x = +1.23 m |
| Rear axle | x = -1.50 m |
| Wheelbase | 2.73 m |
| Track width | 1.62 m |
| Front wheel | 245/38 R20, radius 0.3471 m |
| Rear wheel | 275/35 R20, radius 0.35025 m |
| Static body lift | 0.21 m |

The model's own body height is about 1.11 m, while the XML reports 1.38 m.
The assembly uses the model geometry and the suspension-derived floor height;
the XML height is not forced onto the mesh.

## Runtime integration

- Car catalog: `web/src/ui/car-selection.ts` defines `mazda-rx7` and
  `sc-deckard-conejo`, their pack URLs, and their audio profiles.
- Menu: both launch pages expose a Car picker and persist it in
  `lapster.car.v1`.
- Garage: loads every catalog pack and mixes the models across the ten bays.
  Bay navigation persists the selected bay's car; there is no garage car
  dropdown, selection menu, or `?car=` override.
- Drive: with no `?car=` parameter it loads the selected catalog car. `?car=`
  still overrides for custom packs.
- Audio: the selected catalog entry supplies the Engine_Sim profile. Unknown
  packs keep the legacy `rotary2`/`sports` fallback.
- Cockpit: cars with a `body_interior` get the interior camera override and the
  hardcoded Camaro dial layout. The Deckard pack is the only car using this path.
- Lighting: textured rear-lamp triangles are split by lateral position into
  outer brake and inner reverse lenses. Plain-white body triangles are skipped.
- Bay paint: textured cars keep their baked livery; the garage bay colour does
  not tint them. Catalog models alternate by bay, with the bay colour index
  carried into Drive as the paint choice.

## Pack summary

`sc-deckard-conejo.sima`:

- Asset ID: `sc-deckard-conejo`.
- 11,352 vertices, 13,058 triangles.
- 4 materials, 3 textures, 7 draw calls.
- Body/interior split, glass, four wheels, brake/reverse meshes.
- Physics: 1,700 kg, 0.30 m CG height, 52/48 weight distribution, 6-speed RWD.
  Mass, engine, gearbox, brakes, steering, suspension, tire mu, and aero come
  from `sc-deckard-conejo.xml`; see [Physics](#physics).

## Known issues

The reverse-light lens split is a Deckard-specific heuristic and has not been re-confirmed
visually since the last fix.

## What remains

### Physics

`crates/asset-compiler/src/speed_dreams.rs` parses the source XML and replaces the
GLB-import baseline during `bake-glb --physics-xml`. The mapped values are:

- 1,700 kg mass, 0.30 m CG height, 52/48 front-rear weight distribution.
- 6.2 L V8: 1,000 rpm idle, 6,930 rpm maximum, and the source 21-point torque
  curve.
- Six-speed ratios 3.01/2.07/1.43/1.00/0.84/0.57 with a 3.8 final drive.
- 45° steer lock, source wheel rotation, brake pressure/repartition, and
  per-wheel piston areas/disk diameters.
- Tire mu, spring rates, suspension courses, slow bump/rebound damper rates,
  anti-roll bars, and aero drag/area/lift coefficients.

Mapping assumptions:

- Wheelbase, track, wheel radii, widths, and positions still come from the
  assembled GLB geometry.
- Wheel rate is `spring * bellcrank²`; suspension travel is
  `course / bellcrank`. Dampers use the source slow bump/rebound rates.
- Brake torque is estimated as `pressure * piston area * pad mu * disk radius`.
  The front brake section omits pad mu, so it uses 0.4; the rears use the source
  0.25.
- Tire `friction_scale` uses the source mu; Pacejka shape coefficients remain
  Lapster defaults.
- Inertia is estimated from the mapped mass, wheelbase, and track.
- Downforce is stored but the current vehicle model does not apply it.

Remaining physics work:

- Add engine inertia and engine-braking coefficients to the runtime schema;
  they are present in the source XML but not yet modelled.
- Map reverse gear ratio and Speed Dreams differential behavior.
- Map Pacejka coefficients from Speed Dreams tire stiffness/elasticity rather
  than only the friction scale.
- Apply aero lift/downforce in the vehicle model.
- Generalise section lookup and units for arbitrary Speed Dreams cars.

### Visual and cockpit

- Import the source steering wheel (`steerlr.acc`/`steerhr.acc`) and driver models.
- Use `conejotach.png` and `conejospeed.png` instead of the hardcoded Lapster
  gauge layout, or expose per-car dial metadata in the pack.
- Add headlight and indicator geometry and runtime states.
- Confirm the reworked reverse-lens split visually; the current 0.52 m lateral split is a
  Deckard-specific heuristic.
- Generalise light-lens classification and add proper rear-lamp materials.
- Preserve LODs, alternate skins, and source shadow meshes where useful.
- Add a menu thumbnail/garage preview for car selection.

### Tooling and schema

- Replace car-specific build assumptions with a Speed Dreams source manifest.
- Add pack metadata for source attribution, license, engine audio, cockpit
  layout, and light semantics.
- Move car audio/cockpit hardcoding out of the web client and into the pack.
- Add an `asset-compiler` native Speed Dreams import path so the pipeline does
  not depend on Assimp/glTF Transform/Node for routine rebuilds.
- Add import warnings for unsupported materials, lights, drivers, and LODs.
- Compress textures and tighten the 6.4 MB pack budget.
- Add visual-regression and import-determinism tests for the car.
- Verify FAL redistribution requirements in every packaged build, including
  source availability and modification notices.

### Controls and selection

- Add previews and per-car metadata to the garage/menu car browser.
- Add query-parameter/catalog validation for unknown or mismatched car IDs.
- Test the mixed garage showroom and per-car audio in Playwright.
- Revisit car-specific physics/setup defaults and the cockpit camera for the
  Camaro.

## Attribution

- **Deckard Conejo RR** by Tom_D3 (original model), W.E.C (2025 3D rework), Chad
  Phillips (interior parts), and June Ravenmoon (skins, 2025 rework by W.E.C).
  Free Art License 1.3, via Speed Dreams.
- **Boxer 96 Carson Tuners Edition wheel** by Eric Espie, Jean-Christophe
  Durieu, Olaf Sassnick, Bernhard Wymann, Jean-Philippe Meuret, W.E.C, Eckhard
  M. Jäger, and Chad Phillips. Free Art License 1.3, via Speed Dreams.
- Source commits, file hashes, and required credit lines:
  `fixtures/sources/cars/sc-deckard-conejo/README.md`.
- Distribution notices: `THIRD_PARTY_NOTICES.md`.

Chevrolet, Camaro, Porsche, and 911/996 are trademarks of their respective
owners. Lapster does not imply manufacturer affiliation.
