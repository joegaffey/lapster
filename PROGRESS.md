# Project Progress

Last updated: 2026-10-23
Repository root: the directory containing this file
Current phase: **CP5/CP6 in progress — GLB car and AC track imports are the demo defaults**

This file is the resumable project state. The conversation does not persist, but this file,
`SPEC.md`, `PLAN.md`, and the working tree do. Read all three before continuing.

## Resume checklist

```bash
# Run these from the repository root.
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p sim-types -p sim-asset -p sim-core -p sim-render \
  --target wasm32-unknown-unknown

cd web
npm ci
npm test
npm run test:viewer
```

Build the browser wasm and pages:

```bash
cd web
npm run build:viewer
```

Run the local site:

```bash
# From the repository root:
python3 -m http.server 8123 --bind 127.0.0.1
```

Pages:

- Drive: `http://127.0.0.1:8123/web/drive/`
- Garage: `http://127.0.0.1:8123/web/garage/`
- Track: `http://127.0.0.1:8123/web/track-viewer/`

Expected browser: Chromium/Chrome. WebGPU is used when available; WebGL2 is the fallback.

Deployment:

- Repository: https://github.com/joegaffey/lapster
- CI: `.github/workflows/ci.yml`
- Pages: `.github/workflows/pages.yml`
- Live site: https://joegaffey.com/lapster/

## Environment

| Tool | Version / location |
| --- | --- |
| Rust | 1.99.1 via rustup |
| Cargo | 1.99.1 |
| wasm-bindgen | 0.2.129 |
| wgpu | 30.0.1 |
| wasm-opt | Binaryen 123, cached in `web/.cache/binaryen` |
| Node | v26.1.0 |
| Playwright | 1.64.1 |
| Git | `origin` → https://github.com/joegaffey/lapster |

Toolchain setup scripts:

- `web/scripts/ensure-wasm-opt.sh` — downloads Binaryen if `wasm-opt` is absent.
- `web/scripts/setup-playwright-linux.sh` — downloads the NSS/NSPR libs needed by Playwright Chromium on this Linux image.
- `web/scripts/build-viewer.sh` — release wasm build, wasm-bindgen, wasm-opt, esbuild.
- `web/scripts/test-viewer.sh` — Playwright with the local library path.

## Completed work

| Checkpoint | Status | Notes |
| --- | --- | --- |
| CP0 — contracts/fixtures | Done | `sim-types` SAB ABI, asset manifest, synthetic car/track fixtures, layout tests |
| CP1 — bake/load round trip | Done | `asset-compiler bake/inspect/validate`, `sim-asset`, pack loader, Worker test |
| CP2 — track render slice | Done | `sim-render`, `web/track-viewer`, screenshot test, cache reload test |
| CP3 — car render slice | Done | `web/garage`, body/wheels/steering wheel, wheel controls, screenshot test |
| CP4 — drivable slice | Done | `sim-core`, 500 Hz physics, collider, suspension, Pacejka tires, engine/brakes, `web/drive`, soak test |
| CP4.5 — toon style slice | Done | Toon ramp/rim shader, welded inverted-hull outlines, UV/base-color texture path, generated gauge sign, WebGPU + WebGL2 verified |
| CP5a — GLB car import | Working | `asset-compiler bake-glb`, node transform flattening, wheel extraction, palette-to-vertex-colour bake, generic physics, garage/drive query URLs |
| CP6a — AC track import | Working | `asset-compiler bake-ac-track`, KN5 node/material parser, `surfaces.ini`, `AC_START_*` spawn, drivable collision subset, track/garage/drive query URLs |
| Collider BVH | Done | Median-split BVH over collision triangles; shared ray and sphere queries; Skidpad p99 4.5 ms → 0.2 ms |
| Default demo assets | Done | RX-7 and Skidpad 0.5 checked in; no query parameters required; custom `?car=`/`?track=` still supported |
| Renderer upgrade | Done | wgpu 30.0.1, WebGPU (`BrowserWebGpu`), WebGL2 fallback, sRGB output fix, wasm-opt/LTO pipeline |
| GitHub Actions | In repo | `.github/workflows/ci.yml` tests Rust, Node, wasm, and Playwright |
| GitHub Pages | In repo | `.github/workflows/pages.yml` publishes `dist/` |
| PWA + COI | Done | Manifest/icons, service worker, COOP/COEP injection, one-time controlled reload, cross-origin isolation verified |

## Current capabilities

- Synthetic car + track asset packs with deterministic baking, validation, and IndexedDB caching.
- 500 Hz fixed-step vehicle physics on the main thread with:
  - triangle raycast suspension with anti-roll bars, progressive bump stops, and a rear grip
    bias to reduce power oversteer,
  - CG-based load transfer,
  - Pacejka longitudinal/lateral forces with tire relaxation,
  - optional shared rear traction control and stability control, plus speed-sensitive
    steering for keyboard input,
  - engine/gearbox/brake model,
  - wall contacts and finite-state reset,
  - interpolated visual pose.
- Keyboard controls:
  - `W` / `↑` throttle,
  - `S` / `↓` brake; release and press again at rest to engage reverse,
  - `A` / `D` or `←` / `→` steer; left/right direction is verified,
  - `W` / `↑` brakes while reversing,
  - `0` manual reverse at rest, `1`–`6` gears, `R` reset.
- HUD: speed, RPM, gear, contact count, rack torque, wheel loads/slip, tick, step p50/p99, FPS.
- Tuning dialog: native top-layer setup for inertia/CG, per-axle suspension and tires, engine
  curve, gearing, brakes, steering lock, and aero; persisted per car with reset and copy/paste
  JSON (`lapster/setup@1`). Mass and geometry remain fixed.
- Settings dialog: TC and ESC enables, independent strength controls, steering sensitivity,
  and a debug overlay toggle. Settings persist globally; Reset and Camera remain top-level
  toolbar actions. Drive hides debug lines/bounds by default.
- Responsive UI: collapsible HUD on all demos, collapsed by default. Every demo keeps Info
  and Exit in a detached toolbar; drive adds Reset, Camera, Body, TC, and ESC, and track adds
  Reset camera. Touch devices get non-selectable steering/throttle/brake controls.
- Track viewer camera: drag to orbit, right/shift-drag to pan, wheel or pinch to zoom, with
  a reset button.
- Low-speed rolling resistance settles the car and stops wheel idle-spin when stationary.
- Glass: GLB glass is split into a separate mesh. External/garage views use opaque glass;
  cockpit mode switches it to 32% alpha in a transparent forward pass; bonnet and external
  views keep it opaque. Body and glass hide together; wheels remain visible.
- Drive camera: chase → bonnet → cockpit (`C` / button). Bonnet eye height is 1.25 m; cockpit
  eye is 1.10 m and 0.25 m rearward. Body defaults visible; `Body: Visible/Hidden` toggles body
  and glass. A real cockpit still needs an interior model.
- Renderer reports backend and surface format in telemetry:
  - backend: `BrowserWebGpu` in Chromium,
  - format: `Bgra8Unorm`,
  - manual sRGB encoding in the shader; sky/ground match the pre-upgrade look.
- Forward toon renderer:
  - quantized diffuse ramp and rim light,
  - object-space inverted-hull outlines with welded/averaged normals,
  - optional UV/base-color textures with a 1x1 white fallback,
  - generated 64x64 gauge texture on the synthetic track sign,
  - asset schema version 3.
- GLB car import:
  - `cargo run -p asset-compiler -- bake-glb <car.glb>`
  - Bakes node transforms, palette textures into vertex colours, and derives four wheel transforms.
  - Supports custom `?car=`/`?track=` paths in the garage and drive viewers.
  - First local fixture: Mazda RX-7, 18,322 triangles after removing logo geometry.
- AC track import:
  - `cargo run -p asset-compiler -- bake-ac-track <track-dir>`
  - Parses unencrypted KN5 v6 geometry/materials, AC `surfaces.ini`, spawn markers, and the
    drivable collision subset.
  - First local fixture: Skidpad 0.5, 76,545 visual triangles, 8,214 collision triangles,
    30 draw calls.
  - The outer ANELLO ring is visual-only, textures are not imported, and collision is
    currently limited to the central pad/ASP surfaces.
- Build freshness: generated build version in default pack URLs, `version.json` checks on
  load/resume/focus, stale IndexedDB pack clearing, versioned `main.js` and `pkg/` URLs, and
  no-store navigation/version responses from the service worker.
- PWA/COI: installable manifest/icons and a same-origin service worker that injects
  COOP/COEP and no-store caching on demo navigations. `crossOriginIsolated` and
  `SharedArrayBuffer` verified. The installed menu checks `version.json` on open/resume.
- PWA navigation: installed app launches the `/web/` demo menu; every demo links back to it.
- Wasm: **2.83 MiB raw / 1.06 MiB gzip**.
- Local warm startup measured at roughly **300–480 ms** in headless Chromium.

## Verified checks

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` — 39 tests
- Rust stability tests cover combined acceleration/cornering/braking, a high-speed lane
  change, outside-wheel load transfer, anti-roll roll reduction, a hard landing, traction
  control, ESC yaw/body-slip reduction, powered-cornering balance, speed-sensitive steering,
  braking to a stop while steering, setup apply/reset/round-trip, and assist strength
  controls.
- `cargo check ... --target wasm32-unknown-unknown`
- `npm test` — 15 tests
- `npm run test:viewer` — 17 Playwright tests
- Playwright uses Chromium with WebGPU; drive test asserts `rendererBackend == "BrowserWebGpu"`.
- WebGL2 fallback checked separately: `rendererBackend == "Gl"`, no page/console errors.
- Manual GLB check: Mazda RX-7 imported, rendered in garage, and driven on WebGPU and
  WebGL2 without page/console errors.
- Manual AC track check: Skidpad 0.5 imported, rendered in the track/garage viewers, and
  driven with the Mazda RX-7 on WebGPU without page/console errors.

## Wheel and FFB workstream

**Status:** planned, not implemented. No HID or FFB dependency is wired into the app yet.

**Decisions:**

- Target every wheelbase supported by [`webhid-wheel`](https://github.com/joegaffey/webhid-wheel),
  not a CAMMUS-only path.
- Test CAMMUS C5 first, then CAMMUS C12 and FFBeast; generic USB-PID wheels are best effort.
- Logitech/Fanatec initially get input only; their vendor FFB protocols are not in scope until
  the upstream library adds profiles.
- CP6.5 runs the FFB adapter on the **main thread**. Move it to the HID worker later as an
  optimization if measurements show frame-pacing or `sendReport` jitter problems.
- Keep `webhid-wheel` behind an input/FFB adapter so the worker migration does not change the
  physics or UI contracts.
- Do not reimplement HID descriptors, USB-PID encoding, or vendor reports in this repository.

**CP6.5 scope:**

1. Input abstraction for WebHID, Gamepad API, keyboard, and touch, with device hot-plug and
   calibration.
2. Direct-wheel steering through the vehicle steering ratio and lock, bypassing the keyboard
   speed-sensitive aid.
3. Front-axle self-aligning torque feeding a normalized FFB request and telemetry.
4. Main-thread constant-force output at 100-250 Hz with clamp, strength/invert, watchdog, and
   stop on hide/pause/disconnect.
5. Device/FFB controls in the detached toolbar and assist settings with independent
   strength and steering sensitivity.
6. Unit, Rust, and Playwright coverage; manual CAMMUS C5 hardware checklist.

**Upstream `webhid-wheel` work tracked separately:**

- Optional multi-device rig for wheel + pedals + shifter.
- High-frequency constant-force path with coalescing and backpressure.
- Worker-compatible acquisition and reconnect.
- Expanded profiles and recorded report fixtures.
- CAMMUS C5 primary verification, then C12/FFBeast.

**Hardware acceptance:** CAMMUS C5 steering and FFB, with watchdog/stop behavior. The device
matrix then expands to the other supported wheelbases.

## Known issues / carry-forward

- **Git:** initialized with `origin` at https://github.com/joegaffey/lapster.
- Physics currently runs on the **main thread**, not the worker/SAB topology in `SPEC.md`.
- Collider uses a median-split BVH for ray and wall sphere queries; the Skidpad p99 physics step dropped from ~4.5 ms to ~0.2 ms.
- No Rapier/Jolt integration yet; the spec now treats them as optional collision backends.
- Renderer now samples one base-color texture per mesh, but has no image decoder/KTX2, mipmaps, culling, LOD, or instancing yet.
- GLB importer keeps full mesh density, applies the RX-7 FD3S baseline to imported cars,
  ignores glTF extensions beyond baseColor, and has no decimation or steering/interior
  extraction. AC `.ini` physics import remains pending.
- Imported body paint is currently forced to a demo red; source material colours are baked
  for other parts.
- AC track importer has no texture decoding, uses approximated material colours, skips
  trees/fences, has no wall collision, and only includes the central pad/ASP surfaces in
  collision.
- Car setup must be re-checked against the AC `.ini` importer when that lands; the current
  editable set is based on the GLB-derived physics model.
- The Skidpad 0.5 source archive had no bundled license/readme metadata. Its baked pack is
  checked in as a temporary demo fixture; confirm redistribution rights or replace it before
  a public/commercial release.
- Asset cache is keyed by URL; default URLs use the generated build version, and the Pages
  build versions the Wasm package directory. Explicit custom pack URLs still need a
  cache-busting query after changing the asset.
- `.ini` physics mapping and full texture decoding remain; GLB/AC importers are first-pass.
- No audio or HID/FFB integration yet; wheel/FFB is planned as CP6.5.
- `web/pkg`, generated page JS, `web/.cache`, `web/test-results`, and `web/node_modules` are generated/ignored.
- Temporary caches and local server processes are not part of the repository and do not survive a machine/session restart.
- Playwright screenshot baselines are Linux/Chromium-specific and are skipped in CI.
- Headless Playwright captures the WebGPU canvas as white in this environment; renderer
  stats and debug counters are asserted, and the flat-screen look was checked manually.
- GitHub Pages supplies no COOP/COEP headers; the PWA service worker injects them for demo
  navigations. The first controlled load needs one reload. The physics worker/SAB port is
  still pending, so physics remains on the main thread.
- The live Pages URL is https://joegaffey.com/lapster/ (the github.io URL redirects there).

## Next steps

1. **CP6.5 — wheel/FFB feel spike (next)**: add the input abstraction, direct-wheel steering,
   self-aligning torque, main-thread FFB, toolbar device panel, and CAMMUS C5 hardware test.
2. **Upstream `webhid-wheel`**: multi-device rig, high-frequency constant force, worker
   compatibility, profiles, and hardware fixtures.
3. **CP7 — worker integration and audio**: SAB mailboxes, HID worker migration, audio, full
   hardware matrix, and performance traces.
4. **CP5 — real car**: finish the GLB path with decimation, palette/vertex-colour tuning,
   wheel geometry cleanup, and licensed AC `.ini` physics when a suitable pack is available.
   - Add a simple steering-wheel/interior placeholder.
   - Add real image decoding/KTX2 only where textures are the right choice.
   - WebXR remains a later phase.
5. **CP6 — real track**: finish the AC path with texture decoding (DDS/KTX2 where useful),
   wall/fence collision policy, outer-ring collision or streaming, and performance measurement.
6. Replace per-frame JSON telemetry with typed arrays/SAB.
7. Add content-hash cache invalidation.
8. Add Rapier or Jolt only as a collision/query backend if the custom BVH becomes limiting.
9. Benchmark a real converted track: draw calls, triangles, texture memory, load time, p99 physics.

## Key files

- `SPEC.md` — architecture, contracts, engine policy.
- `PLAN.md` — checkpoint plan.
- `crates/sim-core` — custom vehicle physics and collider.
- `crates/sim-asset` — runtime pack validation/access.
- `crates/asset-compiler` — bake/inspect/validate.
- `crates/sim-render` — wgpu renderer and wasm viewers.
- `web/drive`, `web/garage`, `web/track-viewer` — browser pages.
- `web/tests` — Node and Playwright tests.
- `fixtures/cars/minimal`, `fixtures/tracks/minimal` — synthetic fixtures and baked packs.

## Next-agent prompt

> Read `SPEC.md`, `PLAN.md`, and `PROGRESS.md`. Run the resume checklist. Continue at
> **CP5: real car model and physics**, starting with a glTF/GLB car importer and `.ini`
> mapping. Do not switch the vehicle model to a general-purpose engine's vehicle
> controller; treat Rapier/Jolt as optional collision backends.
