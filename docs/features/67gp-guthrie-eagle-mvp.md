# 67GP Guthrie Eagle MVP plan

Status: proposed
Date: 2026-11-21
Owner: architect

## Objective

Add one Speed Dreams 67GP car, the Guthrie Eagle (`67gp-guthrie-eagle`), to
Lapster using the existing standard toolchain. No custom importers, no new pack
format, and no optional fidelity work.

## Scope

In:

- Tracked FAL source fixture with attribution and license.
- One baked `.sima` pack.
- One catalog entry using the existing `v12` audio profile.
- Parser unit test, Playwright drive test, and pack/source READMEs.
- Minimal parameterisation of the existing Speed Dreams tools.

Out:

- The other five 67GP cars.
- Steering wheel, driver, source cockpit gauges, and per-car cockpit metadata.
- Downforce, top-speed, engine-inertia, LSD, and tire-temperature work.
- Category/filter UI.
- Contract changes.

## Assumptions

- Assimp 6.0.4 is the AC3D importer.
- The existing `@gltf-transform` and `asset-compiler` flow is retained.
- Body lift is calibrated visually; `0.104 m` is the starting value.
- Audio is approximate; the existing `v12` profile is reused.
- Only one car is added in this batch.

## Environment

| Tool | Version |
| --- | --- |
| Assimp | 6.0.4 (`/usr/bin/assimp`, `glb2` and `-embtex` supported) |
| Node | 26.13.1 |
| @gltf-transform | 4.5.1, installed under `tools/speed-dreams/node_modules` |

Assimp 6 is newer than the 5.3.x the Deckard pipeline was built against, so the
first implementation step is a Deckard rebuild smoke test.

## Source data

| Item | Value |
| --- | --- |
| Upstream | https://forge.a-lec.org/speed-dreams/67gp-guthrie-eagle |
| Commit | `01c82922ba16697e1a7d4766013ebadea26bd72c` |
| Author | Copyright 2018 CHAD |
| License | Free Art License 1.3 |
| Body | `67gp-guthrie-eagle.acc`, 1,656,928 B |
| Body texture | `67gp-guthrie-eagle.png`, 2048x2048 RGBA |
| XML | `67gp-guthrie-eagle.xml`, 23,938 B |
| Wheel | `wheel0.acc`, 51,998 B |
| Wheel texture | `wheel3d.png`, 304,272 B |
| Mass / CG | 620 kg / 0.19 m |
| Front weight share | 0.45 |
| Axles | front `+0.96`, rear `-1.50`; wheelbase 2.46 m |
| Track | front 1.54 m, rear 1.58 m |
| Wheel radius | front 0.3101 m, rear 0.3399 m |
| Wheel width | front 0.260 m, rear 0.332 m |
| Ride height | front 90 mm, rear 120 mm |
| Initial body lift | 0.104 m |
| Engine | 3.0 V12, idle 1000 rpm, max 9500 rpm, 14 torque points |
| Gearbox | 5-speed: 3.26 / 2.28 / 1.50 / 1.01 / 0.87, final 3.73 |
| Tire mu | 1.20 |
| Brakes | 14,500 kPa, 300 mm front discs |
| Glass names | `WIGLASS1`, `WIGLASS2`, `WIGLASS3` |
| Body geometry | ~7,869 vertices, ~15,928 triangles |

The rear brake-disc hub in the body model is at y ~= 0.236. With the 0.3399 m
rear wheel radius this gives the initial `body_lift ~= 0.104 m`; calibrate it
visually.

## Design

### Per-car config

Add `fixtures/sources/cars/67gp-guthrie-eagle/import.json`:

```json
{
  "id": "67gp-guthrie-eagle",
  "body": "body/67gp-guthrie-eagle.acc",
  "xml": "body/67gp-guthrie-eagle.xml",
  "wheel": "wheel/wheel0.acc",
  "body_lift": 0.104,
  "glass_names": ["^WIGLASS"],
  "front": { "axle_x": 0.96, "half_track": 0.77, "radius": 0.3101, "width": 0.260 },
  "rear": { "axle_x": -1.50, "half_track": 0.79, "radius": 0.3399, "width": 0.332 }
}
```

No new parser dependency: the values are small and explicit. The XML remains
the source for physics, parsed by the existing Rust `xml-rs` path.

### `build-car.sh`

Change only the hardcoded Deckard inputs:

- Read `import.json` for the body, XML, and wheel paths.
- Keep the temp-directory flow and
  `assimp export ... -f glb2 -embtex`.
- Pass the config to `assemble.mjs`.
- Keep `bake-glb --textures --physics-xml`.

Keep the Deckard path working by giving it an equivalent config or by
preserving a fallback.

### `assemble.mjs`

Read `import.json` and keep the existing structure:

- Body pre-rotation unchanged: `[0, -Math.SQRT1_2, 0, Math.SQRT1_2]`.
- Body translation `[0, body_lift, 0]`.
- Glass material assigned to nodes matching the config regexes.
- Four wheel nodes:
  - `x = front.axle_x` or `rear.axle_x`;
  - `y = radius - body_lift`;
  - `z = -half_track` for left, `+half_track` for right;
  - scale `[radius * 2, radius * 2, width]`;
  - right-side rotation 180 degrees about Y;
  - names `WHEEL_FL`, `WHEEL_FR`, `WHEEL_RL`, `WHEEL_RR`.
- Output the assembled GLB.

The existing scale convention is retained for the MVP. Wheel-mesh recentering is
optional; the 67GP wheel is offset by about 1 cm after scaling.

### Physics

No parser logic changes. The existing `apply_physics` already supports every
required Guthrie section. Add a test only.

## Implementation order

### 0. Assimp smoke test

Rebuild the Deckard pack to a temp path and validate it:

```bash
tools/speed-dreams/build-car.sh \
  fixtures/sources/cars/sc-deckard-conejo \
  /tmp/sc-deckard-conejo.sima
cargo run -p asset-compiler -- validate /tmp/sc-deckard-conejo.sima
```

If Assimp 6.0.4 regresses the AC3D output, pin Assimp 5.3.x before touching the
Guthrie pipeline.

### 1. Stage sources

Create `fixtures/sources/cars/67gp-guthrie-eagle/`:

```text
body/67gp-guthrie-eagle.acc
body/67gp-guthrie-eagle.xml
body/67gp-guthrie-eagle.png
body/readme.txt
wheel/wheel0.acc
wheel/wheel3d.png
LICENSE-FAL-1.3.txt
import.json
README.md
```

`README.md` records the upstream URL, commit, file SHA-256 hashes, authors,
license, and Lapster modifications.

### 2. Parameterise the tools

- Update `tools/speed-dreams/build-car.sh` to read `import.json`.
- Update `tools/speed-dreams/assemble.mjs` to consume the config.
- Update `tools/speed-dreams/README.md` with the config-driven command.

### 3. Bake and validate

```bash
tools/speed-dreams/build-car.sh \
  fixtures/sources/cars/67gp-guthrie-eagle \
  fixtures/cars/67gp-guthrie-eagle/67gp-guthrie-eagle.sima

cargo run -p asset-compiler -- inspect fixtures/cars/67gp-guthrie-eagle/67gp-guthrie-eagle.sima
cargo run -p asset-compiler -- validate fixtures/cars/67gp-guthrie-eagle/67gp-guthrie-eagle.sima
```

Expected inspect values: 620 kg, 2.46 m wheelbase, 1.54/1.58 m tracks, five
forward gears, and a 3.73 final drive.

### 4. Visual calibration

Open:

```text
/web/garage/?car=../../fixtures/cars/67gp-guthrie-eagle/67gp-guthrie-eagle.sima
```

Check:

- wheels align with the suspension arms and brake discs;
- body sits at a believable ride height;
- glass is transparent;
- no z-fighting or detached wheels.

Adjust `body_lift`, and optionally add a small pitch for the 90/120 mm
ride-height difference, until correct.

Final wheel treatment: wheel meshes use a flat mesh path (no toon ramp, no rim
light, and no outline) while the body and track keep the toon/rim treatment.
The renderer sets the flat flag from the mesh name containing `wheel`, so it
applies to all cars. Implemented in `crates/sim-render/src/renderer.rs` and
`crates/sim-render/src/track.wgsl`.

### 5. Web integration

Add one entry to `web/src/ui/car-selection.ts`:

```ts
{
  id: "67gp-guthrie-eagle",
  name: "Guthrie Eagle",
  url: "../../fixtures/cars/67gp-guthrie-eagle/67gp-guthrie-eagle.sima",
  engine: "v12",
  vehicle: "sports",
}
```

Reuse the existing `v12` audio profile and `sports` vehicle preset for the MVP.

### 6. Tests

- Rust: add `maps_guthrie_eagle_xml_physics` to
  `crates/asset-compiler/src/speed_dreams.rs`, asserting mass 620, CG 0.19,
  9500 rpm, 14 torque points, five gears, 3.73 final drive, tire mu 1.2,
  front/rear spring rates, and brake bias.
- Playwright: add a drive test using the 67GP pack URL; assert mass 620,
  wheelbase 2.46, gear greater than 1, speed greater than 20 km/h, and no page
  errors.
- Optionally add a garage smoke test with the same `?car=` URL.

### 7. Documentation and attribution

- `fixtures/cars/67gp-guthrie-eagle/README.md`: pack SHA-256, size, schema,
  mesh/texture counts, physics summary, rebuild command.
- `fixtures/sources/cars/67gp-guthrie-eagle/README.md`: source provenance and
  hashes.
- `THIRD_PARTY_NOTICES.md`: add the Guthrie Eagle FAL entry and required credit
  line.
- `fixtures/README.md`: list the new car and source fixture.
- `docs/speed-dreams-pipeline.md`: note the config-driven one-car support and
  remaining generic-importer work.
- `tools/speed-dreams/README.md`: update the build command.

### 8. Gate

Because `fixtures/**` changes, run:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p sim-types -p sim-asset -p sim-core -p sim-render -p sim-worker \
  --target wasm32-unknown-unknown
npm --prefix web test
npm --prefix web run test:viewer
npm --prefix web run test:viewer -- --project=default --grep "WebGL2 fallback"
```

Also run `git diff --check`.

## Acceptance criteria

- The Guthrie Eagle pack validates and inspects as 620 kg / 2.46 m wheelbase /
  five-speed RWD.
- The car appears in the picker and garage, drives, and shifts without NaN.
- Wheels and body align in the garage at a believable ride height.
- Source, license, hashes, and attribution are committed.
- Deckard still builds with Assimp 6.0.4, or Assimp is pinned to 5.3.x.
- Full gate passes.

## Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Assimp 6.0.4 changes AC3D/GLB output | Deckard smoke test first; pin 5.3.x if needed |
| Body lift/rake is wrong | Config value; visual calibration in the garage |
| 2048 body texture baked to 1024 | Accept for MVP; revisit texture policy later |
| No downforce and 216 km/h clamp | Accept; document as a physics limitation |
| No steering wheel or driver model | Use the open-wheel cockpit camera; document |
| Garage loads all catalog packs | One extra car is acceptable; the six-car batch needs lazy loading (TD-0016) |
| FAL source redistribution | Commit the pruned source and license with hashes |
| Wheel mesh 1 cm offset | Accept; recenter as a follow-up |

## Integration findings

- Assimp 6.0.4 builds the Guthrie pack and the Deckard rebuild validates. The
  Deckard rebuild triangulates 16,322 triangles instead of the committed 13,058
  and produces 6,466,136 bytes instead of 6,426,968. Bounds, vertices,
  materials, draw calls, and XML physics match. `-tri` does not change the
  count. Pin Assimp 5.3.x only if byte-level determinism against the committed
  pack is required.
- The existing XML parser already covers all six 67GP cars. No parser logic
  changed. The Guthrie test asserts 620 kg, 0.19 m CG height, 1,000/9,500 rpm,
  14 torque points, five gears, 3.73 final drive, 30 degree steering lock, tire
  mu 1.2, spring and anti-roll rates, and brake bias.
- The accepted pack is the single-wheel configuration: one `wheel0.acc` reused
  at all four corners, body lift 0.104 m, 620 kg, 2.46 m wheelbase, 1.54/1.58 m
  tracks, SHA-256 `7a0340...`, 5,354,452 bytes, 9,105 vertices / 19,328
  triangles.
- The source provides four corner-specific wheel models with different UV
  quadrants. Per-corner mapping, right-side flip, and left-side flip were tried
  and reverted. They did not improve the visible result enough and added config
  complexity. Keep the single wheel unless a specific car has a visual failure.
- The gray wheel-slab artifact was the toon ramp, additive rim light, and wheel
  outline, not the mesh or texture. Wheel meshes now use flat shading
  (`0.45 + 0.55 * N dot L`), no rim light, and no outline. The renderer flags
  meshes by name containing `wheel`, so the body and track keep the toon/rim
  look.
- The default cockpit camera was left-seat and 1.10 m high. Open-wheel cars
  (mass below 900 kg) now use eye `[0, 0.84, 0.16]`, target `[0, 0.78, -20.0]`,
  and fov 1.0. The source driver position is centered and about 0.834 m high.
- Open-wheel synthetic cockpits keep only a centered RPM dial at
  `[0, 0.64, -0.32]` (radius 0.050, depth 0.045). The speed dial, gear segments,
  and synthetic dashboard cowl are removed. The runtime
  `set_cockpit_dial_offset("rpm", x, y, z)` API remains for tuning.
- The source interior/dashboard is kept. Excluding the `INT` object did not
  remove the lower source dials and was reverted. The synthetic RPM gauge
  overlaps the source dash at the chosen position. `exclude_nodes` remains an
  optional tool feature.
- Adding a third catalog car changed hardcoded garage/catalog expectations and
  the bay-5 selection test. The garage-camera test must preselect the car that
  owns the bay (`sc-deckard-conejo` for bay 5).
- Playwright uses project-qualified snapshot names
  (`garage-viewer-default-linux.png`, `track-viewer-default-linux.png`). The old
  `*-linux.png` baselines remain stale; the new files are part of this batch.
- The garage still loads every `CAR_OPTIONS` pack at boot. A six-car 67GP batch
  would load roughly 30-40 MB. Lazy or curated loading is required before
  adding the other five.
- The assets worker's first pass briefly wrote three tracked tooling files into
  the shared race worktree and restored them. The race worktree was verified
  intact. Future scoped workers must use absolute worktree paths and `workdir`.

## Follow-ups

- Import the remaining five 67GP cars using the same config. Address the garage
  pack-loading scale before adding them.
- Replace the open-wheel mass-below-900 kg camera/dial heuristic with per-pack
  cockpit metadata.
- Decide whether to keep or remove the stale `*-linux.png` snapshot baselines.
- Add a 67GP audio profile and vehicle preset.
- Import steering wheel and driver models.
- Apply downforce and revisit the speed clamp.
- Add engine inertia, LSD, and tire temperature if the physics model grows.
