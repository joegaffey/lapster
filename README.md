# Lapster

A web-native vehicle simulation engine written in Rust and WebAssembly.

Lapster is an early, runnable vertical slice of a browser racing simulator: deterministic
asset packs, a custom vehicle model, wgpu rendering, and a small set of playable viewers.
It is not a finished game or a complete Assetto Corsa replacement.

## Demos

Live demo: **https://joegaffey.com/lapster/**

- **Drive** — Mazda RX-7 on the Skidpad 0.5 track: throttle, brake, auto reverse, steering,
  gears, chase camera, wheel loads, and 500 Hz physics.
- **Garage** — RX-7 on the skidpad with wheel steering/spin controls and debug bounds.
- **Track viewer** — skidpad render, spawn point, bounds, axes, and collision-surface overlay.

The demos load these assets by default; no query parameters are required. Custom packs can
still be selected with `?car=` and `?track=`. The info HUD is collapsible everywhere and
starts collapsed on mobile; the drive demo adds on-screen steering, throttle, brake, and
reset controls on touch devices.

The demo uses WebGPU when available and falls back to WebGL2. Chromium/Chrome is recommended.
GitHub Pages cannot send the COOP/COEP headers required for `SharedArrayBuffer`; the current
demo runs physics on the main thread. The planned worker/SAB mode will require a host that can
set those headers.

## Current features

- Synthetic fixtures plus default GLB car and AC track assets with deterministic baking and
  validation.
- Versioned asset packs: JSON manifest, aligned rkyv chunks, SHA-256 chunk verification.
- 500 Hz fixed-step vehicle physics:
  - raycast suspension,
  - Pacejka longitudinal/lateral tire forces with combined-slip limiting,
  - engine, gearbox, brakes, and auto reverse,
  - wall contacts and finite-state reset,
  - interpolated visual pose.
- wgpu 30 renderer with WebGPU and WebGL2 support.
- Browser viewers for track, car, and driving.
- Rust, Node, and Playwright test suites.

## Controls

| Input | Action |
| --- | --- |
| `W` / `↑` | Throttle |
| `S` / `↓` | Brake; at standstill, auto reverse |
| `A` / `D` or `←` / `→` | Steer |
| `0` | Manual reverse |
| `1`–`6` | Gears |
| `R` | Reset |

## Build and run

Prerequisites: stable Rust, the `wasm32-unknown-unknown` target, `wasm-bindgen` 0.2.129,
Node 24+, and Python 3 for the local server.

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked

cd web
npm ci
npm run build:viewer

cd ..
python3 -m http.server 8123 --bind 127.0.0.1
```

Open `http://127.0.0.1:8123/`.

The build script downloads Binaryen to `web/.cache` on first use.

## Test

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p sim-types -p sim-asset -p sim-core -p sim-render \
  --target wasm32-unknown-unknown

cd web
npm test
npm run test:viewer
```

`npm run test:viewer` builds the wasm and runs Playwright in Chromium. On Linux it provides
the NSS/NSPR libraries if the system does not have them.

## Workspace

```text
crates/
  sim-types/       shared ABI, SAB layout, asset schema
  sim-asset/       runtime pack validation and typed archive access
  asset-compiler/  offline bake/inspect/validate CLI
  sim-core/        custom vehicle dynamics and collision trait
  sim-render/      wgpu renderer and wasm viewers
web/               TypeScript glue, viewers, build scripts, tests
fixtures/          synthetic car and track fixtures plus baked packs
```

Vehicle dynamics are intentionally first-party. General-purpose physics engines are optional
collision/query backends only; they are not used as the vehicle model. See `SPEC.md` for the
physics backend policy.

The visual direction is a forward-rendered technical/papercraft toon style with simple
geometry, vertex colors, object-space outlines, and textures only where they are the
practical choice (gauges, signs, decals). See [`docs/art-style.md`](docs/art-style.md).

## Credits

- **Mazda RX-7** by IvOfficial, CC BY 3.0, via
  [Poly Pizza](https://poly.pizza/m/SnIoWlh7S2). The import removes the source logo mesh;
  Mazda and RX-7 are trademarks of their respective owners.
- **Skidpad 0.5** is a temporary demo track. The source archive contained no license
  metadata; replace it or confirm redistribution rights before a public/commercial release.

## Status and roadmap

Implementation status and resume instructions are in [`PROGRESS.md`](PROGRESS.md). The plan is
in [`PLAN.md`](PLAN.md). Next up: AC `.ini` physics mapping, track texture decoding,
collision/wall policy, worker/SAB physics, audio, and force feedback.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion
in this project by you shall be dual licensed as above, without any additional terms or
conditions.
