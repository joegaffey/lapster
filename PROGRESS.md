# Project Progress

Last updated: 2026-10-23
Repository root: the directory containing this file
Current phase: **CP0-CP4 complete; next is CP5 (real car model and physics)**

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
| Renderer upgrade | Done | wgpu 30.0.1, WebGPU (`BrowserWebGpu`), WebGL2 fallback, sRGB output fix, wasm-opt/LTO pipeline |
| GitHub Actions | In repo | `.github/workflows/ci.yml` tests Rust, Node, wasm, and Playwright |
| GitHub Pages | In repo | `.github/workflows/pages.yml` publishes `dist/` |

## Current capabilities

- Synthetic car + track asset packs with deterministic baking, validation, and IndexedDB caching.
- 500 Hz fixed-step vehicle physics on the main thread with:
  - triangle raycast suspension,
  - Pacejka longitudinal/lateral forces,
  - engine/gearbox/brake model,
  - wall contacts and finite-state reset,
  - interpolated visual pose.
- Keyboard controls:
  - `W` / `↑` throttle,
  - `S` / `↓` brake; holding at standstill engages auto reverse,
  - `A` / `D` or `←` / `→` steer; left/right direction is verified,
  - `0` manual reverse, `1`–`6` gears, `R` reset.
- HUD: speed, RPM, gear, contact count, rack torque, wheel loads/slip, tick, step p50/p99, FPS.
- Renderer reports backend and surface format in telemetry:
  - backend: `BrowserWebGpu` in Chromium,
  - format: `Bgra8Unorm`,
  - manual sRGB encoding in the shader; sky/ground match the pre-upgrade look.
- Wasm: **2.8 MB raw / 1.09 MB gzip**.
- Local warm startup measured at roughly **300–480 ms** in headless Chromium.

## Verified checks

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` — 21 tests
- `cargo check ... --target wasm32-unknown-unknown`
- `npm test` — 15 tests
- `npm run test:viewer` — 7 Playwright tests
- Playwright uses Chromium with WebGPU; drive test asserts `rendererBackend == "BrowserWebGpu"`.

## Known issues / carry-forward

- **Git:** initialized with `origin` at https://github.com/joegaffey/lapster.
- Physics currently runs on the **main thread**, not the worker/SAB topology in `SPEC.md`.
- Collider is a linear triangle scan; real tracks need a BVH or Rapier/Jolt collision backend.
- No Rapier/Jolt integration yet; the spec now treats them as optional collision backends.
- Renderer has **no textures/UV sampling**, no culling, LOD, instancing, or texture compression yet.
- Asset cache is keyed by URL plus a manual `?v=cp4.2` query, not by content hash.
- Real car/track import (`.kn5`, `.ini`, glTF) is not implemented.
- No audio, HID, or FFB integration yet.
- `web/pkg`, generated page JS, `web/.cache`, `web/test-results`, and `web/node_modules` are generated/ignored.
- Temporary caches and local server processes are not part of the repository and do not survive a machine/session restart.
- Playwright screenshot baselines are Linux/Chromium-specific and are skipped in CI.
- GitHub Pages cannot send COOP/COEP, so the current main-thread demo works there but the
  planned SharedArrayBuffer worker mode will need a different host or custom headers.
- The live Pages URL is https://joegaffey.com/lapster/ (the github.io URL redirects there).

## Next steps

1. **CP5 — real car**: glTF car importer, `.ini` physics mapping, validation, run on the
   synthetic track, with the toon/papercraft style from `docs/art-style.md`.
   - Forward toon shading, flat normals, vertex colors, inverted-hull outlines.
   - Textures only for gauges, signs, decals, and small sprites.
   - WebXR remains a later phase.
2. **CP6 — real track**: `.kn5`/glTF import, surfaces/collision, spawn/camera metadata, performance measurement.
3. Move physics to a dedicated worker with the SAB snapshot/input/FFB contract.
4. Replace per-frame JSON telemetry with typed arrays/SAB.
5. Add content-hash cache invalidation.
6. Add Rapier or Jolt only as a collision/query backend if the custom collider becomes a bottleneck.
7. Add audio and FFB/`webhid-wheel`.
8. Benchmark a real converted track: draw calls, triangles, texture memory, load time, p99 physics.

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
