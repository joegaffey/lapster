# Speed Dreams car tools

Reproduces Lapster's Speed Dreams car import outside the runtime. This is an
offline conversion helper, not part of the web build.

Full pipeline and limitations: [`docs/speed-dreams-pipeline.md`](../../docs/speed-dreams-pipeline.md).

## Prerequisites

- Rust and Cargo.
- Node 24+.
- Assimp command-line tools 5.3.x (`assimp-utils` on Debian/Ubuntu,
  `brew install assimp` on macOS).
- The tool dependencies:

```bash
npm install --prefix tools/speed-dreams
```

## Build the Deckard Conejo pack

```bash
tools/speed-dreams/build-car.sh \
  fixtures/sources/cars/sc-deckard-conejo \
  fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima
```

The script writes only the final `.sima`. Intermediate GLBs and embedded
textures stay in a temporary directory.

Validate the result:

```bash
cargo run -p asset-compiler -- inspect fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima
cargo run -p asset-compiler -- validate fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima
```

## How it works

1. `assimp export` converts the Speed Dreams `.acc` body and donor wheel to GLB
   with embedded textures.
2. `assemble.mjs` deduplicates, flattens, and joins the four wheel primitives;
   assigns the glass material; rotates and lifts the body; and adds four wheel
   nodes.
3. `asset-compiler bake-glb --textures --physics-xml <xml>` bakes the runtime
   pack, replacing the GLB physics baseline with the source Speed Dreams XML
   mass, engine, gearbox, brakes, steering, suspension, tire mu, and aero.

The script is intentionally specific to the Deckard Conejo layout. Generalising
it for arbitrary Speed Dreams cars is remaining work; see the pipeline doc.
