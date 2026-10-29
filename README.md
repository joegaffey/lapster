# Lapster

A web-native vehicle simulation engine written in Rust and WebAssembly.

Lapster is an early, runnable vertical slice of a browser racing simulator: deterministic
asset packs, a custom vehicle model, wgpu rendering, and a small set of playable viewers.
It is not a finished game or a complete Assetto Corsa replacement.

## Demos

Live demo: **https://joegaffey.com/lapster/**

Development preview: **https://joegaffey.com/lapster-dev/**

Both builds default to Hybrid environment shadows (full-track static plus near dynamic) and the Dynamic car shadow.

- **Drive** — Mazda RX-7 on the Skidpad 0.5 track: throttle, brake, edge-triggered reverse,
  steering, automatic gearbox with optional auto clutch, chase/cockpit cameras, wheel loads,
  500 Hz physics, and a
  shareable tuning dialog.
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
still be selected with `?car=` and `?track=`. Append `?dev=1` to Drive to enable the developer
drawer: a left slide-in panel with live assists, view, audio, camera profiles, particle
emission/spawn/fade/render settings for every surface, shadow strength/bias/extents, physics
globals, GPU texture sizes, and every car-tuning field as sliders or JSON inputs. It starts
closed; use the auto-hiding `DEV` tab at the left edge or the backslash key to toggle it.
Physics changes use `Apply physics`, audio can be restarted in place, and GPU texture changes
use `Apply GPU & reload` with the tuning persisted across the reload. Drive exposes WebHID
wheel and constant-force FFB controls under **Settings → Controls**. Every 3D viewer shows a
full-screen loading
indicator until its first rendered frame; the info HUD starts collapsed in markup on
every demo, so it cannot flash before boot; drive actions live in a detached top toolbar,
with `Exit` returning to the menu. The drive demo adds on-screen steering, throttle, and
brake controls on touch devices. Debug lines/bounds are off by default in all viewers; drive
can re-enable the debug overlay from Settings. Builds are versioned: the Pages build gives the
Wasm package its own versioned directory, and demos/menu check `web/version.json` on load and
resume, clearing stale caches and reloading when a newer deployment is available.

The demo uses WebGPU when available and falls back to WebGL2. Chromium/Chrome is recommended.
Lapster is installable as a PWA. The installed app launches a demo menu, and every demo has a
`Exit` link to return there. A same-origin service worker supplies the COOP/COEP headers needed
for `SharedArrayBuffer` on GitHub Pages; the first controlled load reloads once. On this
Drive physics runs in a dedicated Rust/Wasm worker by default and exchanges inputs and
snapshots through the SAB. `?physics=main` selects the main-thread fallback when cross-origin
isolation or worker startup fails.

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
  - automatic gearbox with configurable auto gears and auto clutch; the box selects neutral
    at rest and re-engages on throttle or brake when auto gears are enabled,
  - momentary numbered-gear buttons that return to neutral on release, and free revving to
    redline in neutral or with the clutch disengaged,
  - edge-triggered reverse,
  - wall contacts, deterministic stability tests, and finite-state reset,
  - interpolated visual pose.
- Physics worker with a versioned SAB input mailbox, four-slot snapshot ring, seqlock
  publication, an `Atomics.wait` deadline loop, and interpolated main-thread rendering.
  Main-thread physics remains available with `?physics=main`.
- **Wheel/FFB dev spike:** vendored `webhid-wheel` 0.4.42 WebHID input with
  multi-device sessions, portable setup profiles, direct wheel steering, front-axle
  self-aligning torque, and a main-thread fast constant-force path fed from the SAB FFB
  mailbox at 200 Hz. Drive exposes the shared `<webhid-config-panel>` under
  **Settings → Controls** for device management, steering/FFB calibration, pedal and
  button mapping, profile import/export, and emergency stop. Lapster feel controls for rear
  breakaway lightening, front-grip shudder, road noise, roughness, and slip-driven looseness
  live in the panel's app slot. Mapped throttle, brake, clutch, handbrake, and gear buttons
  feed the driving input alongside keyboard and touch fallbacks. Browsers without WebHID
  still boot normally and hide the wheel controls.
- **Cockpit dial cluster:** a generated cowl, runtime speedometer/RPM textures, a
  seven-segment sprite gear display for `N`, `1–6`, and `R`, and red needles calibrated to
  `7:30 → 4:30`; dial faces are recessed into their cylinders.
- **Audio spike:** a vendored [Engine_Sim](https://github.com/YigitSalihEmecen/Engine_Sim)
  `rotary2` AudioWorklet synthesiser drives engine, exhaust, intake, transmission, and turbo
  audio. A synthesized surface layer blends asphalt roar for the pad and banked outer ring,
  a low concrete rumble plus grit for the transition apron, kerb patter, and grass
  rumble/rustle from per-wheel surface names and speed. A tire-screech layer gates
  band-passed noise from lateral and longitudinal skid, load, and speed. Audio starts off on
  first use and the enabled/muted preference is remembered; after a reload audio starts on the
  first interaction because browsers require a user gesture. There are no sample assets yet. See [`docs/audio.md`](docs/audio.md) for the audio architecture and
  credits.
- wgpu 30 renderer with WebGPU/WebGL2 support, native MSAA (4× desktop / 2× mobile
  auto), a Settings override, and a soft sky gradient.
- Four environment shadow modes: **High** dynamic (4-tap PCF), **Hybrid** full-track baked
  coverage plus a near dynamic car map, startup-baked **Baked** (4096² desktop / 2048²
  mobile and WebGL2), or **Off**. Car shadows are independent:
  dynamic, a generated silhouette decal, or off. Time of day uses a slider plus Apply and
  tints the sky; Baked maps rebake when lighting is applied.
- Drive brake and reverse lights: rear-lamp lens geometry split from the body and lit
  emissively by brake input or reverse gear. Garage is unchanged for now.
- Browser viewers for track, car, and driving.
- Runtime **Tuning** dialog for chassis, suspension, tires, engine, gearing, brakes, steering,
  and aero; settings persist per car and can be copied/pasted as `lapster/setup@1` JSON.
- **Settings** dialog split into **Gameplay** and **Graphics** tabs, available in every
  viewer. Graphics covers render scale, anti-aliasing, environment and car shadows, time
  of day, the debug overlay, and an optional top-left FPS counter.
- Rust, Node, and Playwright test suites.

## Controls

| Input | Action |
| --- | --- |
| `W` / `↑` | Throttle; brakes while reversing |
| `S` / `↓` | Brake; release and press again at rest to engage reverse |
| `A` / `D` or `←` / `→` | Steer |
| `C` | Cycle chase / bonnet / cockpit / isometric camera (remembered across reloads) |
| `R` | Reset |

**Reset** and **Camera** remain top-level toolbar actions. The **Audio** button starts audio
(off on each load) and mutes/unmutes it afterwards. **Tuning** opens the top-layer car tuning
dialog; **Copy settings** produces a versioned JSON payload for sharing, and **Paste shared
settings** imports one. **Settings** opens the assists menu for TC/ESC/ABS enables, independent
strengths (TC and ESC default to 50%), steering sensitivity, anti-aliasing, and the debug
overlay; these settings persist globally. Debug lines/bounds are off by default in drive.

The default RX-7 uses an FD3S baseline: 1,270 kg, 2.425 m wheelbase, 314 Nm / 276 hp,
8,000 rpm redline, and Cd 0.29 with 1.85 m² frontal area. Suspension and inertia values are
estimates until a licensed AC `.ini` source is available.

The RX-7 glass is imported as a separate mesh. External/garage views keep the original
opaque glass; cockpit mode renders it at 6% alpha. The body is visible by default, wheels
always stay visible, and `Body: Visible/Hidden` toggles the body and glass together. The bonnet
camera is centered on the car's longitudinal axis at 1.25 m; cockpit eye is 1.10 m, pushed
0.25 m rearward. The generated dial cowl is not yet a full interior model.

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
npm run bench:physics
```

The audio spike is covered by `web/tests/audio.spec.ts`; Engine_Sim's synthesis graph is driven
from a conditioned version of the vehicle's RPM plus actual speed and gear through its
external-state adapter.

`npm run test:viewer` builds the wasm and runs Playwright in Chromium. On Linux it provides
the NSS/NSPR libraries if the system does not have them. `npm run bench:physics` runs the
opt-in main-thread vs worker A/B benchmark; set `PERF_CPU_THROTTLE=4` for a CPU-throttled run.
Frame pacing is only meaningful on GPU-backed Chromium; the benchmark always asserts worker
tick rate. Add `PERF_ASSERT_FPS=1` to gate frame pacing and `PERF_REQUIRE_MAIN_HZ=1` to require
main-thread physics to hold 500 Hz in the same environment.

## Workspace

```text
crates/
  sim-types/       shared ABI, SAB layout, asset schema
  sim-asset/       runtime pack validation and typed archive access
  asset-compiler/  offline bake/inspect/validate CLI
  sim-core/        custom vehicle dynamics and collision trait
  sim-worker/      Rust/Wasm physics worker entry and simulation wrapper
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
- **Engine_Sim** by Yigit Salih Emecen, MIT, powers Drive engine audio. See
  [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) and
  [`docs/audio.md`](docs/audio.md). No upstream audio samples are included.

## Status and roadmap

Implementation status and resume instructions are in [`PROGRESS.md`](PROGRESS.md). The plan is
in [`PLAN.md`](PLAN.md). Drive uses the SAB physics worker by default and an external-state
Engine_Sim rotary2 audio spike. Next is listening/tuning and deciding whether to restore more
EngineSim drivetrain transients or move the audio DSP directly onto SAB snapshots. Remaining
work includes the HID/FFB worker, AC `.ini` physics mapping, track texture decoding,
collision/wall policy, and hardening.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion
in this project by you shall be dual licensed as above, without any additional terms or
conditions.

Drive audio vendors [Engine_Sim](https://github.com/YigitSalihEmecen/Engine_Sim) under the MIT
license. See [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md),
`web/src/audio/engine-sim/NOTICE.md`, and `LICENSE`.
