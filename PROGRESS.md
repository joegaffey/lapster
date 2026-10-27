# Project Progress

Last updated: 2026-10-26
Repository root: the directory containing this file
Current phase: **Audio spike — SAB worker default and Engine_Sim rotary2 AudioWorklet**

This file is the resumable project state. The conversation does not persist, but this file,
`SPEC.md`, `PLAN.md`, and the working tree do. Read all three before continuing.

## Resume checklist

```bash
# Run these from the repository root.
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p sim-types -p sim-asset -p sim-core -p sim-render -p sim-worker \
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
- 500 Hz fixed-step vehicle physics in the SAB worker by default (`?physics=main` selects the
  main-thread fallback):
  - triangle raycast suspension with anti-roll bars, progressive bump stops, and a rear grip
    bias to reduce power oversteer,
  - CG-based load transfer,
  - Pacejka longitudinal/lateral forces with tire relaxation,
  - tangent-plane tire forces and body contact probes for raised/sloped collision surfaces,
  - optional ABS, shared rear traction control and stability control defaulting to 50%
    strength, plus speed-sensitive steering for keyboard input,
  - engine/gearbox/brake model with closed-throttle engine braking,
  - traction-control launch torque limiting and raw-slip feedback at low speed,
  - wall contacts and finite-state reset,
  - SAB input/snapshot seqlocks, four-slot snapshot history, and interpolated visual pose;
  - `Atomics.wait` deadline scheduling, bounded catch-up, and rate diagnostics;
  - automatic main-thread fallback if SAB/worker startup fails.
- Audio spike: vendored Engine_Sim MIT synthesiser (`rotary2`) with combustion events,
  exhaust/intake resonators, transmission, and optional turbo; toolbar mute; no sample assets
  or track reverb yet. A small external-state adapter drives its RPM/speed/gear from the
  vehicle and locks the clutch, so the engine note follows the simulation instead of
  Engine_Sim's own drivetrain. A 60 ms RPM conditioner with a 12,000 rpm/s slew limit removes
  pitch judder from wheelspin/TC oscillation while preserving launch flare. Load uses a fast
  attack and 320 ms release, with an overrun load floor and reduced overrun attenuation, so
  lift-off does not cut the sound abruptly. Continuous lash rattle is disabled in external
  mode; a single click/thump cue is triggered immediately on forward gear changes. The audio
  RPM conditioner shortens its time constant during a shift so pitch and cue stay together.
  A separate tire-screech layer uses two band-passed noise bands, gated by per-wheel lateral
  and longitudinal slip, load, contact, and speed; it fires for cornering scrub, wheelspin,
  and lockup; its threshold is skid excess rather than ordinary cornering slip, and its gain
  is exposed for diagnostics/tests. A road/surface bed blends asphalt roar for the pad and
  banked outer ring, concrete apron rumble/grit, kerb patter, and grass rumble/rustle from
  per-wheel surface names and speed; tire squeal is suppressed on grass. The HUD exposes the current wheel
  surface and road-noise gain. Audio starts off on first use and remains on after the toolbar
  button is used; the enabled/muted preference is remembered across reloads and resumes on the
  first gesture. EngineSim's transmission mesh whine is disabled for now (`mix.transmission = 0`).
- Automatic gearbox by default: forward gears shift using throttle/RPM thresholds from the
  car setup; reverse remains the edge-triggered brake action. Manual gear keys are removed for
  the auto-only spike. Shift logic currently lives in `web/drive/main.ts`; move it into
  `sim-core` once the behaviour is accepted.
- Keyboard controls:
  - `W` / `↑` throttle,
  - `S` / `↓` brake; release and press again at rest to engage reverse,
  - `A` / `D` or `←` / `→` steer; left/right direction is verified,
  - `W` / `↑` brakes while reversing,
  - automatic forward gearbox (upshift/downshift with 0.35 s anti-hunt), `R` reset.
- HUD: speed, RPM, gear, contact count, rack torque, wheel loads/slip, tick, step p50/p99, FPS.
- Tuning dialog: native top-layer setup for inertia/CG, per-axle suspension and tires, engine
  curve, gearing, brakes, steering lock, and aero; persisted per car with reset and copy/paste
  JSON (`lapster/setup@1`). Mass and geometry remain fixed.
- Settings dialog: split Gameplay/Graphics tabs in all viewers. Gameplay covers TC/ESC/ABS,
  strengths, and steering sensitivity. Graphics covers render scale, anti-aliasing,
  environment shadows, car shadows, time of day, a debug overlay toggle, and an
  off-by-default FPS counter.
  Settings persist globally; Reset and Camera remain top-level toolbar actions. Drive hides
  debug lines/bounds by default.
- Native MSAA through wgpu: auto 4× desktop / 2× mobile, with Off/2×/4× overrides. Unsupported
  counts fall back to the highest supported count not above the request (usually 1× for 2× on
  WebGPU).
- Responsive UI: collapsible HUD on all demos, pre-collapsed in markup so it cannot flash
  before boot. Every demo keeps Info and Exit in a detached toolbar; drive adds Reset, Camera,
  Body, TC, and ESC, and track adds Reset camera. Touch devices get non-selectable
  steering/throttle/brake controls. Debug
  lines/bounds are off by default in drive, garage, and track viewer.
- Track and garage cameras: drag to orbit and wheel or pinch to zoom, with a reset-camera
  button. Track adds right/shift-drag panning and caps the camera at 60 m to avoid high-angle
  model artifacts; garage is orbit/zoom only. Garage starts from a front three-quarter view
  with the wheels turned slightly.
- Garage showroom: two facing rows of five cars each (red/blue/yellow/green/orange opposite
  black/white/pink/navy/cyan), aligned to the five canopy bays. Screen-side chevrons and
  Left/Right arrow keys fly the camera between cars, with per-bay camera homes and wheel
  controls following the selection. The chevrons start hidden, appear on mouse movement or
  touch, and fade out when idle. The selected car persists in `localStorage` and becomes the
  driving car, including its showroom paint; the garage `Drive` button and the menu Drive
  demo both use it.
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
- Forward toon renderer with a pale-horizon/blue-zenith sky gradient:
  - quantized diffuse ramp and rim light,
  - object-space inverted-hull outlines with welded/averaged normals,
  - optional UV/base-color textures with a 1x1 white fallback,
  - generated 64x64 gauge texture on the synthetic track sign,
  - asset schema version 3.
- Shadow modes: **High** dynamic focused map (2048² desktop, 1024² mobile/WebGL2) with
  4-tap PCF; **Baked** startup-baked static light-space shadow map (4096² desktop,
  2048² mobile/WebGL2) using the same 4-tap PCF without a per-frame depth pass; **Off**.
  Independent car shadow setting selects dynamic, generated silhouette decal, or off.
  Dynamic car shadows use a dedicated 4096² desktop / 2048² mobile car-only map with
  32-tap PCF and a tight car-plus-shadow focus. Car silhouette follows car yaw and stretches
  from the footprint at low sun angles. Garage cars are static and cast from the environment
  shadow map.
  Time of day uses a slider plus Apply, tints the sky, updates sun direction, and rebakes
  Baked maps. Settings are global across all viewers. All desktop and mobile builds default
  to Baked environment shadows and the Dynamic car shadow. Auto is not offered; legacy
  stored Auto shadow values are coerced to Baked and Dynamic, while explicit modes are
  preserved.
- Drive brake and reverse lights: rear-lamp lens geometry is split from the body and lit
  emissively by brake-pedal input or reverse gear only. Garage remains unchanged.
- GLB car import:
  - `cargo run -p asset-compiler -- bake-glb <car.glb>`
  - Bakes node transforms, palette textures into vertex colours, and derives four wheel transforms.
  - Supports custom `?car=`/`?track=` paths in the garage and drive viewers.
  - First local fixture: Mazda RX-7, 18,322 triangles after removing logo geometry.
- AC track import:
  - `cargo run -p asset-compiler -- bake-ac-track <track-dir>`
  - Parses unencrypted KN5 v6 geometry/materials, AC `surfaces.ini`, spawn markers, and the
    drivable collision subset.
  - First local fixture: Skidpad 0.5, 76,545 visual triangles, 43,885 collision triangles,
    30 draw calls.
  - Collision includes the central pad, ASP surfaces, banked outer asphalt ring, and grass;
    the tan `TRANSITION` concrete apron is tagged `ANELLO` so the concrete audio layer
    engages. Grass friction is capped at 0.35. Textures are not imported and walls/fences
    remain visual-only.
  - Visual coplanar layers are offset: the concrete apron sits 1.5 cm below the tarmac, and
    thin road overlays sit 5 mm above it.
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
- `cargo test --workspace` — 54 tests
- Rust stability tests cover combined acceleration/cornering/braking, a high-speed lane
  change, outside-wheel load transfer, anti-roll roll reduction, a hard landing, traction
  control, ESC yaw/body-slip reduction, powered-cornering balance, speed-sensitive steering,
  braking to a stop while steering, setup apply/reset/round-trip, assist strength controls,
  slope settling/maneuvering, and ABS wheel-lock prevention.
- `cargo check ... --target wasm32-unknown-unknown`
- `npm test` — 22 tests
- `npm run test:viewer` — 37 Playwright tests plus an opt-in physics benchmark
- `npm run bench:physics` — main-thread vs worker tick rate and frame pacing; set
  `PERF_CPU_THROTTLE=4` for a throttled run
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
- Drive defaults to the SAB worker. `?physics=main` selects the main-thread fallback. Garage
  and Track Viewer have no physics loop.
- Audio is a spike only: the external-state adapter bypasses Engine_Sim's own clutch/shift
  transients. Continuous gear rattle is disabled; a synthetic click/thump covers forward gear
  changes. This external mode is a set of compensations around library tuning that assumes
  EngineSim's own drivetrain (load floor, overrun attenuation, rattle suppression, and shift
  cue). Launch slip and some overrun pops are reduced. The conditioned audio RPM lags the raw
  signal by roughly 60 ms outside shifts. There is no per-car profile, sample layer, Doppler,
  reverb zones, or mobile listening test yet. Tire screech is a first-pass centered noise layer
  with no per-axle panning or sample layer. Surface noise uses per-wheel surface names
  (`ASPHALT`/`ASP*`, `KERB`, `ANELLO`, `GRASS`). Skidpad's banked outer asphalt ring maps to
  `ASPHALT`; its tan `TRANSITION` concrete apron maps to `ANELLO` so the concrete layer
  engages. The ANELLO case uses a low-pass rumble plus grit and amplitude modulation; grass
  uses a lower rumble plus a low rustle band.
  Noise is centered mono. Audio is unavailable when physics falls back to the main thread
  because there is no SAB producer.
- The raw `sim-core` engine RPM is still wheel-speed-derived and visibly oscillates during
  wheelspin/TC intervention (observed 0.0-0.7 slip and large rpm swings). Audio is smoothed,
  but the HUD and engine torque still see the raw signal; add engine/clutch inertia or a
  sim-core RPM filter if this needs fixing at the source.
- Traction control now limits launch torque from current raw slip below 8 m/s; above that it
  relies on the filtered slip controller. Engine braking is a simple rpm-scaled negative
  driveline torque, not a full engine-inertia/clutch model.
- Engine_Sim declares MIT in its README/package.json but had no license file upstream; the
  vendored copy includes the MIT text and provenance in `web/src/audio/engine-sim/NOTICE.md`;
  `THIRD_PARTY_NOTICES.md` is copied into the Pages output. Ask upstream to add a LICENSE file
  before relying on this for a release.
- The worker uses an `Atomics.wait` deadline loop with bounded catch-up. In a local 4× CPU
  throttling run at 1280×720 it held ~500 Hz and ~17 fps while main-thread physics fell to
  ~298 Hz and ~9 fps; under no throttling both modes held 60 fps. Headless SwiftShader cannot
  measure frame pacing meaningfully, but the benchmark still checks worker tick rate.
- Keep the worker opt-in until CPU-throttled results are reproduced on the target hardware.
- EngineSim's transmission mesh whine is muted pending retuning; gear changes use the
  click/thump cue instead.
- The auto gearbox is currently a UI-layer spike. `sim-core` still takes an explicit gear, so
  `simulate()` tests and any future AI must supply gears themselves until the logic moves into
  Rust.
- Collider uses a median-split BVH for ray and wall sphere queries; the Skidpad p99 physics step dropped from ~4.5 ms to ~0.2 ms.
- No Rapier/Jolt integration yet; the spec now treats them as optional collision backends.
- Renderer now samples one base-color texture per mesh, but has no image decoder/KTX2, mipmaps, culling, LOD, or instancing yet.
- GLB importer keeps full mesh density, applies the RX-7 FD3S baseline to imported cars,
  ignores glTF extensions beyond baseColor, and has no decimation or steering/interior
  extraction. AC `.ini` physics import remains pending.
- Imported body paint is currently forced to a demo red; source material colours are baked
  for other parts.
- AC track importer has no texture decoding, uses approximated material colours, skips
  trees/fences, has no wall collision, and includes the central pad, ASP surfaces, outer
  banked asphalt ring, and grass in collision.
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
  navigations and worker script/Wasm responses. The first controlled load needs one reload.
  Worker responses need the same COOP/COEP/CORP headers; the service worker supplies them.
- The live Pages URL is https://joegaffey.com/lapster/ (the github.io URL redirects there).

## Next steps

1. **Audio spike (next)**: listen to Engine_Sim's `rotary2` preset with the RPM conditioner.
   If the drivetrain transients are missed, restore selected EngineSim shift/launch events
   rather than returning to its independent drivetrain. Decide separately whether to smooth
   engine RPM in `sim-core` and retune TC.
2. **CP6.5 — wheel/FFB feel spike**: add the input abstraction, direct-wheel steering,
   self-aligning torque, main-thread FFB, toolbar device panel, and CAMMUS C5 hardware test.
3. **Upstream `webhid-wheel`**: multi-device rig, high-frequency constant force, worker
   compatibility, profiles, and hardware fixtures.
4. **CP7 — HID/audio integration**: move the tested FFB adapter into the HID worker, add
   per-car audio profiles / an optional sample layer, expand the hardware matrix, and run
   performance traces. The physics worker, input mailbox, snapshot ring, FFB mailbox, and
   audio worklet spike are already in place.
5. **CP5 — real car**: finish the GLB path with decimation, palette/vertex-colour tuning,
   wheel geometry cleanup, and licensed AC `.ini` physics when a suitable pack is available.
   - Add a simple steering-wheel/interior placeholder.
   - Add real image decoding/KTX2 only where textures are the right choice.
   - WebXR remains a later phase.
6. **CP6 — real track**: finish the AC path with texture decoding (DDS/KTX2 where useful),
   wall/fence collision policy, outer-ring collision or streaming, and performance measurement.
7. Replace per-frame JSON telemetry with typed arrays/SAB.
8. Add content-hash cache invalidation.
9. Add Rapier or Jolt only as a collision/query backend if the custom BVH becomes limiting.
10. Benchmark a real converted track: draw calls, triangles, texture memory, load time, p99 physics.

## Key files

- `SPEC.md` — architecture, contracts, engine policy.
- `PLAN.md` — checkpoint plan.
- `crates/sim-core` — custom vehicle physics and collider.
- `crates/sim-asset` — runtime pack validation/access.
- `crates/asset-compiler` — bake/inspect/validate.
- `crates/sim-render` — wgpu renderer and wasm viewers.
- `crates/sim-worker` — Rust/Wasm physics worker simulation wrapper.
- `web/src/audio/engine-sim/` — vendored Engine_Sim MIT DSP; see `NOTICE.md` and `LICENSE`.
- `docs/audio.md` — current audio architecture, surface model, credits, and deferred work.
- `web/drive`, `web/garage`, `web/track-viewer` — browser pages.
- `web/tests` — Node and Playwright tests.
- `fixtures/cars/minimal`, `fixtures/tracks/minimal` — synthetic fixtures and baked packs.

## Next-agent prompt

> Read `SPEC.md`, `PLAN.md`, and `PROGRESS.md`. Run the resume checklist. Continue at
> **CP5: real car model and physics**, starting with a glTF/GLB car importer and `.ini`
> mapping. Do not switch the vehicle model to a general-purpose engine's vehicle
> controller; treat Rapier/Jolt as optional collision backends.
