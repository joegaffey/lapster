# Lapster

A web-native vehicle simulation engine written in Rust and WebAssembly.

Lapster is an early, runnable vertical slice of a browser racing simulator: deterministic
asset packs, a custom vehicle model, wgpu rendering, and a small set of playable viewers.
It is not a finished game or a complete Assetto Corsa replacement.

## Demos

Live demo: **https://joegaffey.com/lapster/**

Development preview: **https://joegaffey.com/lapster-dev/** — Baked environment and
Dynamic car shadows by default.

- **Drive** — Mazda RX-7 on the Skidpad 0.5 track: throttle, brake, edge-triggered reverse,
  steering, gears, chase/cockpit cameras, wheel loads, 500 Hz physics, and a shareable
  tuning dialog.
- **Garage** — ten RX-7 bays in two facing rows of five: red/blue/yellow/green/orange
  opposite black/white/pink/navy/cyan. Each car sits centered in its canopy bay with a front
  three-quarter start view; screen-side chevrons or arrow keys fly smoothly between bays.
  The chevrons start hidden, fade in on mouse movement or touch, and fade out when idle.
  Wheel steering/spin controls and an orbit/zoom camera follow the selection. The selected
  car is remembered and drives with its showroom paint; the garage `Drive` button opens it
  directly, and the menu's Drive demo uses the same selection.
- **Track viewer** — skidpad render, spawn point, bounds, axes, collision-surface overlay,
  and orbit/pan/zoom camera navigation. Camera height is capped at 60 m to avoid high-angle
  model artifacts.

The demos load these assets by default; no query parameters are required. Custom packs can
still be selected with `?car=` and `?track=`. The info HUD starts collapsed in markup on
every demo, so it cannot flash before boot; drive actions live in a detached top toolbar,
with `Exit` returning to the menu. The drive demo adds on-screen steering, throttle, and
brake controls on touch devices. Debug lines/bounds are off by default in all viewers; drive
can re-enable the debug overlay from Settings. Builds are versioned: the Pages build gives the
Wasm package its own versioned directory, and demos/menu check `web/version.json` on load and
resume, clearing stale caches and reloading when a newer deployment is available.

The demo uses WebGPU when available and falls back to WebGL2. Chromium/Chrome is recommended.
Lapster is installable as a PWA. The installed app launches a demo menu, and every demo has a
`Exit` link to return there. A same-origin service worker supplies the COOP/COEP headers needed
for `SharedArrayBuffer` on GitHub Pages; the first controlled load reloads once. The physics
worker/SAB port is still pending, so the current demo runs physics on the main thread.

## Current features

- Synthetic fixtures plus default GLB car and AC track assets with deterministic baking and
  validation.
- Versioned asset packs: JSON manifest, aligned rkyv chunks, SHA-256 chunk verification.
- 500 Hz fixed-step vehicle physics:
  - raycast suspension with anti-roll bars, stiffer damping, and progressive bump stops,
  - Pacejka longitudinal/lateral tire forces projected onto the contact plane, with relaxed
    slip and combined-slip limiting,
  - CG-based load transfer, engine, gearbox, brakes, optional ABS, shared rear traction
    control, stability control, and speed-sensitive steering for keyboard input,
  - edge-triggered reverse,
  - wall contacts, deterministic stability tests, and finite-state reset,
  - interpolated visual pose.
- wgpu 30 renderer with WebGPU/WebGL2 support, native MSAA (4× desktop / 2× mobile
  auto), a Settings override, and a soft sky gradient.
- Three-way shadow setup: dynamic **High** (4-tap PCF), startup-baked **Baked** environment
  shadows (4096² desktop / 2048² mobile and WebGL2), or **Off**. Car shadows are independent:
  dynamic, a generated silhouette decal, or off. Time of day uses a slider plus Apply and
  tints the sky; Baked maps rebake when lighting is applied.
- Browser viewers for track, car, and driving.
- Runtime **Tuning** dialog for chassis, suspension, tires, engine, gearing, brakes, steering,
  and aero; settings persist per car and can be copied/pasted as `lapster/setup@1` JSON.
- **Settings** dialog split into **Gameplay** and **Graphics** tabs, available in every
  viewer. Graphics covers anti-aliasing, environment and car shadows, time of day, the
  debug overlay, and an optional top-left FPS counter.
- Rust, Node, and Playwright test suites.

## Controls

| Input | Action |
| --- | --- |
| `W` / `↑` | Throttle; brakes while reversing |
| `S` / `↓` | Brake; release and press again at rest to engage reverse |
| `A` / `D` or `←` / `→` | Steer |
| `0` | Manual reverse at rest |
| `1`–`6` | Gears |
| `C` | Cycle chase / bonnet / cockpit camera |
| `R` | Reset |

**Reset** and **Camera** remain top-level toolbar actions. **Tuning** opens the top-layer car
tuning dialog; **Copy settings** produces a versioned JSON payload for sharing, and **Paste
shared settings** imports one. **Settings** opens the assists menu for TC/ESC/ABS enables,
independent strengths, steering sensitivity, anti-aliasing, and the debug overlay; these
settings persist globally. Debug lines/bounds are off by default in drive.

The default RX-7 uses an FD3S baseline: 1,270 kg, 2.425 m wheelbase, 314 Nm / 276 hp,
8,000 rpm redline, and Cd 0.29 with 1.85 m² frontal area. Suspension and inertia values are
estimates until a licensed AC `.ini` source is available.

The RX-7 glass is imported as a separate mesh. External/garage views keep the original
opaque glass; cockpit mode renders it at 32% alpha. The body is visible by default, wheels
always stay visible, and `Body: Visible/Hidden` toggles the body and glass together. Bonnet eye
height is 1.25 m; cockpit eye is 1.10 m, pushed 0.25 m rearward. A real cockpit still requires
an interior model.

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
in [`PLAN.md`](PLAN.md). Next up: wheel input and force feedback on CAMMUS C5 first, targeting
all `webhid-wheel`-supported wheelbases, with Chromium WebHID and Gamepad fallback. The
main-thread FFB spike comes before the worker/SAB migration; then AC `.ini` physics mapping,
track texture decoding, collision/wall policy, audio, and performance hardening.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion
in this project by you shall be dual licensed as above, without any additional terms or
conditions.
