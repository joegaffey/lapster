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

- Assimp 5.3.x command-line tools.
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

## Conversion stages

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

- deduplicates, flattens, joins, and prunes the four wheel primitives into one
  reusable wheel mesh;
- assigns a transparent `Glass` material to `GLASHOUSE` and `FLGT*` nodes;
- pre-rotates the body `-90°` around Y;
- lifts the body by `0.21 m`;
- adds `WHEEL_FL`, `WHEEL_FR`, `WHEEL_RL`, and `WHEEL_RR` nodes;
- scales wheels to the source radii and widths;
- rotates right-side wheels `180°` around Y so their rims face outward;
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
