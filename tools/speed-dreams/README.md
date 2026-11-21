# Speed Dreams car tools

Reproduces Lapster's Speed Dreams car import outside the runtime. This is an
offline conversion helper, not part of the web build.

Full pipeline and limitations: [`docs/speed-dreams-pipeline.md`](../../docs/speed-dreams-pipeline.md).

## Prerequisites

- Rust and Cargo.
- Node 24+.
- Assimp command-line tools 5.3.x or 6.0.x (`assimp-utils` on Debian/Ubuntu,
  `brew install assimp` on macOS).
- The tool dependencies:

```bash
npm install --prefix tools/speed-dreams
```

## Per-car `import.json`

Each source directory has an `import.json` consumed by both scripts:

```json
{
  "id": "sc-deckard-conejo",
  "body": "body/sc-deckard-conejo.acc",
  "xml": "body/sc-deckard-conejo.xml",
  "wheel": "wheel/wheel0.acc",
  "body_lift": 0.21,
  "glass_names": ["GLASHOUSE", "^FLGT"],
  "front": { "axle_x": 1.23, "half_track": 0.81, "radius": 0.3471, "width": 0.245 },
  "rear": { "axle_x": -1.5, "half_track": 0.81, "radius": 0.35025, "width": 0.275 }
}
```

- `body`, `xml`, and `wheel` are paths relative to the source directory.
- `body_lift` is the initial body lift in metres.
- `glass_names` are regular expressions matched case-insensitively against body
  node names; matching primitives receive the shared transparent `Glass`
  material.
- `exclude_nodes` optionally lists case-insensitive regular expressions matched
  against body root child node names; matching nodes and their meshes are
  disposed before assembly, so they are not written to the output. Omit the
  field to keep all source body nodes.
- `front` and `rear` hold the axle `axle_x`, `half_track`, `radius`, and `width`
  in metres used to position and scale the wheel nodes.

## Build the Deckard Conejo pack

```bash
tools/speed-dreams/build-car.sh \
  fixtures/sources/cars/sc-deckard-conejo \
  fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima
```

## Build the Guthrie Eagle 67GP pack

```bash
tools/speed-dreams/build-car.sh \
  fixtures/sources/cars/67gp-guthrie-eagle \
  fixtures/cars/67gp-guthrie-eagle/67gp-guthrie-eagle.sima
```

The script writes only the final `.sima`. Intermediate GLBs and embedded
textures stay in a temporary directory.

Validate the result:

```bash
cargo run -p asset-compiler -- inspect fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima
cargo run -p asset-compiler -- validate fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima
cargo run -p asset-compiler -- inspect fixtures/cars/67gp-guthrie-eagle/67gp-guthrie-eagle.sima
cargo run -p asset-compiler -- validate fixtures/cars/67gp-guthrie-eagle/67gp-guthrie-eagle.sima
```

The output filename determines the pack ID. Keep the `<id>.sima` name matching
the source `import.json` ID; otherwise `asset-compiler` derives a different
manifest ID.

## How it works

1. `build-car.sh` resolves the source directory, reads `import.json`, and runs
   `assimp export` on the configured body and wheel model with embedded textures.
2. `assemble.mjs` deduplicates, flattens, joins, and prunes the wheel model;
   assigns the configured glass materials; rotates and lifts the body; and adds
   four wheel nodes from the configured axle, track, radius, and width, reusing
   one mesh and rotating the right-side nodes 180 degrees about Y.
3. `asset-compiler bake-glb --textures --physics-xml <xml>` bakes the runtime
   pack, replacing the GLB physics baseline with the source Speed Dreams XML
   mass, engine, gearbox, brakes, steering, suspension, tire mu, and aero.

The tools are config-driven for the staged Speed Dreams cars. Cars with a
different XML section layout still need the physics mapping in
`crates/asset-compiler/src/speed_dreams.rs`; see the pipeline doc.
