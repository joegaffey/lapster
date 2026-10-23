# Engineering Specification: Modular Web-Native Simulation Engine

This specification defines the architecture, workspace layout, and data contracts for a
high-performance, zero-backend, web-native vehicle simulation engine. It is designed for
execution by an AI coding agent adhering to **permissive licensing** and **original engine
implementation**.

The engine has no application server. It does require a static host that can send
cross-origin isolation headers, and it uses third-party permissive dependencies where that
is the correct engineering choice. Notably, HID and force feedback are implemented by the
MIT-licensed [`webhid-wheel`](https://github.com/joegaffey/webhid-wheel) library.

## Scope and Non-Goals

### MVP scope

- One vehicle model, one track, four raycast wheels.
- Rigid chassis, suspension, Pacejka-style tires, engine, gearbox, clutch, brakes.
- WebGPU rendering with a best-effort WebGL2 fallback.
- Engine audio driven by simulation state.
- Constant-force FFB on a documented, limited device matrix.
- Deterministic fixed-step replay from recorded inputs on the same build.

### Non-goals for MVP

- Cross-platform deterministic physics.
- Universal force feedback across all wheelbases.
- Multiplayer, AI opponents, damage, weather, tire thermal/wear simulation.
- Hard real-time guarantees. Browsers do not provide them.
- Delegating vehicle handling to a general-purpose physics engine's vehicle controller.

---

## 1. System Architecture & Threading Topology

Responsibility is split across isolated execution contexts. Hot state moves through a
versioned `SharedArrayBuffer`; lifecycle and control move through `MessageChannel`s.

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ MAIN THREAD (DOM / UI / RENDER)                                             │
│  - Canvas lifecycle, WebGPU (wgpu) renderer, HUD, telemetry overlay         │
│  - Asset fetch, IndexedDB cache, SAB allocation                             │
│  - WebHID device authorization (requires a user gesture)                    │
│  - Worker bootstrap and lifecycle control                                   │
└───────────────┬───────────────────────────────┬─────────────────────────────┘
                │ SAB snapshots                 │ control MessageChannel
     ┌──────────▼───────────┐       ┌───────────▼──────────────────────────┐
     │ HID & FFB WORKER     │       │ PHYSICS WORKER                       │
     │ (Dedicated Worker)   │       │ (Dedicated Worker, Rust/Wasm)        │
     │ - webhid-wheel (ESM) │       │ - sim-worker + sim-core              │
     │ - TS adapter         │       │ - 500 Hz default fixed step          │
     │ - input publication  │       │ - 1 kHz desktop opt-in               │
     │ - FFB coalescing     │       │ - snapshot writer                    │
     │ - safety watchdog    │       │ - FFB request writer                 │
     └──────────┬───────────┘       └───────────────┬──────────────────────┘
                │ SAB input mailbox                 │ SAB snapshots
                └───────────────┬───────────────────┘
                                │
                    ┌───────────▼────────────┐
                    │ AUDIO WORKLET          │
                    │ (AudioWorklet thread)  │
                    │ - sim-audio DSP        │
                    │ - 128-frame quantum    │
                    │ - wait-free SAB reads  │
                    └────────────────────────┘
```

### Thread roles

- **Main thread:** owns the DOM, canvas, WebGPU device, UI, asset loading, and device
  authorization. It reads snapshots from the SAB for the HUD and telemetry. It must not run
  the physics loop.
- **HID & FFB worker:** owns `webhid-wheel`, wheel input, FFB output, and FFB safety. This
  worker exists so HID event and Promise jitter cannot stall the physics loop.
- **Physics worker:** owns `sim-core` and the fixed-step loop. It reads input from the SAB,
  writes snapshots and FFB requests to the SAB, and handles lifecycle messages.
- **Audio worklet:** reads snapshots from the SAB and synthesizes engine audio. It must be
  wait-free and allocation-free in `process()`.
- **Renderer:** runs on the main thread for MVP. Moving it to an `OffscreenCanvas` worker is
  a later optimization, not a requirement.

### Important platform constraints

- **WebHID is Chromium-only.** The primary FFB target is desktop Chromium. Firefox and
  Safari get input/gamepad fallbacks and no WebHID FFB.
- **WebHID device selection requires transient user activation.** `requestDevice()` is
  called on the main thread from a user gesture. The HID worker then acquires the already
  authorized device via `navigator.hid.getDevices()` and matches by vendor/product ID.
- **SAB requires cross-origin isolation:** `Cross-Origin-Opener-Policy: same-origin` and
  `Cross-Origin-Embedder-Policy: require-corp`, plus a secure context. Static hosts without
  custom headers can use a same-origin service worker to synthesize those headers on demo
  navigations; the first controlled load requires one reload. GitHub Pages uses this path.
- **No hard real-time scheduling exists on the web.** The engine targets measured budgets
  and graceful degradation, not guaranteed tick timing.

---

## 2. Cargo Workspace Structure

```text
web-sim-engine/
├── Cargo.toml                 # resolver = "2"
├── crates/
│   ├── sim-types/             # Shared repr(C) ABI, SAB layout, snapshot/message types
│   ├── sim-core/              # Pure Rust: custom vehicle dynamics, tires, collision trait, fixed step
│   ├── sim-worker/            # wasm-bindgen entry, worker loop, scheduling, SAB I/O
│   ├── sim-render/            # wgpu/WebGPU renderer, WebGL2 fallback, mesh/material upload
│   ├── sim-audio/             # DSP + AudioWorklet wasm adapter
│   └── asset-compiler/        # Native CLI: .ini/.kn5/glTF -> versioned rkyv asset packs
├── web/
│   ├── package.json           # depends on webhid-wheel (pinned commit or version)
│   └── src/
│       ├── main.ts            # UI, canvas, SAB allocation, worker bootstrap
│       ├── hid/
│       │   ├── adapter.ts     # webhid-wheel integration, torque mapping, safety, coalescing
│       │   └── profiles.ts    # app-level device allow-list and settings
│       ├── render/            # wgpu wasm loader and render loop glue
│       └── audio/             # AudioWorklet module and wasm shim
└── docs/
    ├── asset-format.md
    ├── art-style.md
    ├── sab-layout.md
    └── device-matrix.md
```

### Workspace rules

- There is **no `sim-hid` Rust crate**. HID and FFB are implemented by `webhid-wheel` in the
  HID worker, with a small TypeScript adapter at `web/src/hid/adapter.ts`.
- `sim-core` has no browser, wasm, or DOM dependencies. It must compile and test natively.
- `sim-core` owns the collision-backend trait and the default in-house triangle collider.
  Optional Rapier or Jolt adapters must be feature-gated or separate crates; no
  general-purpose physics engine is a hard dependency of the vehicle model.
- Runtime crates must compile for `wasm32-unknown-unknown`. `asset-compiler` is native-only
  and excluded from wasm target checks.
- `webhid-wheel` is pinned to a specific commit or version and recorded in `NOTICE`.
- Target-specific dependencies belong in target-specific sections of `Cargo.toml`.

---

## 3. Module Specifications

### Module 1: `sim-types` (Shared ABI)

**Responsibility:** Define the byte-level contract shared by Rust, TypeScript, and workers.

**Key requirements:**

- `#[repr(C)]` structs, fixed-width integer fields, no `usize` in archived or shared data.
- Constants for magic, version, offsets, slot count, slot stride, and mailbox offsets.
- Compile-time and test-time assertions for every field offset and total size.
- A TypeScript mirror of the layout in `web/src/` with matching offset tests.
- Versioned layout: readers reject unknown major versions and clear the SAB rather than
  reading garbage.

### Module 2: `sim-core` (Physics & Tire Dynamics)

**Responsibility:** Implement the vehicle state vector, suspension raycasting, tire force
generation, fixed-step integration, and the collision-backend abstraction.

**Key requirements:**

- Pure Rust, no web APIs, no DOM, no filesystem.
- Zero heap allocation in the user-authored hot path (`step()`). Use fixed-size arrays,
  preallocated buffers, and no dynamically growing collections. Verify with a counting
  global allocator in tests.
- `glam` for math. Enable `simd128` for wasm where supported; maintain a scalar fallback.
- Vehicle dynamics are first-party `sim-core` code. Chassis integration, suspension,
  tire forces, drivetrain, brakes, and steering are not delegated to a general-purpose
  physics engine.
- Collision and spatial queries are behind a `CollisionBackend` trait: raycasts, shape
  casts, contact queries, and broadphase. The default backend is the in-house triangle
  collider used by the synthetic track. Rapier (`rapier3d`, Apache-2.0) is the preferred
  optional backend for complex track meshes and query performance.
- Do **not** use Rapier's `DynamicRayCastVehicleController`, its tire forces, or any
  general-purpose engine's vehicle controller as the vehicle model. Rapier's internal
  allocations are outside the zero-allocation guarantee and must be measured when enabled.
- Units: meters, kilograms, seconds, radians. Coordinate frame: right-handed, `+Y` up,
  `-Z` forward. The asset compiler converts source assets into this frame.
- Fixed step: 500 Hz default; 1 kHz is an explicit desktop opt-in. Use semi-implicit
  integration or a substep scheme for stiff suspension forces. Clamp forces and reject
  non-finite states with a documented reset path.

**Minimum state contract (conceptual):**

```rust
pub struct Controls {
    pub steering: f32,   // -1..1
    pub throttle: f32,   // 0..1
    pub brake: f32,      // 0..1
    pub clutch: f32,     // 0..1
    pub gear: i32,
}

pub struct WheelState {
    pub angular_velocity: f32,
    pub steer_angle: f32,
    pub suspension_travel: f32,
    pub contact: bool,
    pub slip_ratio: f32,
    pub slip_angle: f32,
    pub normal_load: f32,
    pub surface_friction: f32,
}

pub struct VehicleState {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub velocity: glam::Vec3,
    pub angular_velocity: glam::Vec3,
    pub mass: f32,
    pub inertia: glam::Mat3,
    pub wheels: [WheelState; 4],
    pub controls: Controls,
    pub engine_rpm: f32,
    pub gear: i32,
    pub steering_rack_torque: f32,
    pub tick: u32,
}
```

**Tire model requirements:**

- Pacejka Magic Formula for longitudinal and lateral force.
- Load sensitivity (`Fz`) and combined-slip handling, not independent `Fx`/`Fy` lookup.
- Relaxation length / transient slip at low speed.
- Low-speed singularity handling (slip ratio and slip angle at near-zero velocity).
- Camber and surface friction inputs.
- Self-aligning torque `M_z` feeding a steering model that includes steering-axis geometry,
  caster, kingpin inclination, column compliance, friction, and damping.
- `steering_rack_torque` is an engine-level normalized torque request, not a raw HID report.
  The FFB adapter maps it to the connected device.

**Determinism:** Same build and platform only. Record inputs and fixed-step ticks for
replay. Do not claim cross-platform determinism unless a specific backend has been verified
in-browser; Jolt is a candidate, Rapier is not assumed.

#### Physics and Collision Backend Policy

**Decision:** vehicle dynamics are first-party code. A general-purpose physics engine may
provide collision and spatial queries, but it is not the source of vehicle handling.

- **Rapier** (`rapier3d`, Apache-2.0) is the preferred optional collision/query backend.
  Enable it only when needed for complex track meshes or query performance. Do not use its
  vehicle controller or tire model.
- **Jolt** (MIT) is the documented upgrade path if Rapier's large-scene performance,
  determinism, or constraint support becomes limiting. Jolt is still a collision/rigid-body
  backend; its wheel constraints do not replace Pacejka/suspension.
- **Box3D** (MIT) is a watch item. It is new and provides wheel joints but no tire model. Do
  not depend on it until it has a stable API and a proven Rust/wasm integration.
- **Not adopted:** ODE, Ammo.js, cannon-es, Bullet's `btRaycastVehicle`, and equivalent
  general-purpose vehicle controllers. They do not provide sim-grade tire/suspension
  behavior and add integration or licensing cost without a fidelity benefit.
- **Reference only:** rustyAC uses a Rust port of ODE for collision and a 1:1 port of
  Assetto Corsa's physics. Its physics is GPL-3.0-or-later and must not be copied. Its split
  supports this policy: ODE is collision; AC's custom model is handling.
- **Determinism:** the custom fixed-step model is the replay authority. If a collision
  backend advertises cross-platform determinism (Jolt does; verify it), do not change the
  replay policy without in-browser proof. Rapier determinism is same-build/platform unless
  proven otherwise.
- **Swap gate:** before replacing a collision backend, pass golden replay traces, raycast
  and contact conformance tests, p99 step budgets, and browser wasm-size/performance checks.

### Module 3: `sim-worker` (Worker Runtime)

**Responsibility:** Drive `sim-core` from a dedicated worker and publish state through the
SAB.

**Key requirements:**

- Deadline-driven fixed-step loop using a monotonic clock. Use `Atomics.wait` for coarse
  sleep and a short spin for the final sub-millisecond. Never use `setTimeout`/`setInterval`
  as the tick source; nested timers clamp to ~4 ms.
- Maximum catch-up of 4 ticks. Beyond that, drop accumulated time and report a starvation
  event. Never let the backlog grow unbounded.
- Read the latest input mailbox once per tick. If the input seqlock is inconsistent, reuse
  the last valid input; do not retry in the hot loop.
- Write one snapshot per tick, then publish the slot index with an atomic store.
- Write the latest FFB torque request into the FFB mailbox every tick. The HID worker
  coalesces and rate-limits it.
- Use SAB flags for pause/reset; keep `MessageChannel` for non-hot lifecycle commands.
- Expose diagnostics: tick count, step time p50/p99, catch-up events, and dropped time.
- No heap allocation in the tick path.

### Module 4: HID & FFB (`webhid-wheel` + TypeScript Adapter)

**Responsibility:** Read wheel input and drive constant-force FFB through `webhid-wheel`.
There is no Rust HID implementation.

**Key requirements:**

- Import `webhid-wheel` in the HID worker. Do not duplicate HID descriptor parsing, USB-PID
  encoding, or vendor report layouts.
- Main thread performs initial device authorization with `navigator.hid.requestDevice()`
  from a user gesture. The worker acquires the authorized device with
  `navigator.hid.getDevices()` and matches by vendor/product ID. If worker acquisition
  fails, fall back to running the adapter on the main thread with a documented latency
  penalty and diagnostics.
- Publish normalized steering, throttle, brake, clutch, buttons, and a timestamp into the
  input mailbox.
- Read the FFB torque mailbox, coalesce to the latest value, clamp, apply strength and
  inversion, and send at a device-safe rate. Target 100-250 Hz updates. Do not block the
  physics loop on HID writes.
- Create the effect once where the device/profile allows it. On updates, prefer sending only
  the magnitude. The generic `webhid-wheel` `constant()` path sends multiple reports per
  call; where an upstream fast path or profile hook is available, use it. Otherwise coalesce
  calls and rate-limit full updates to a device-safe rate.
- Handle `sendReport` backpressure and errors. Never queue an unbounded number of writes.

**Known integration risks:**

- Verify that the pinned `webhid-wheel` constructor and `connect(device)` path work in the
  target worker context. `connect()` can accept an already selected device, which avoids a
  worker-side `requestDevice()` call, but the constructor still feature-detects
  `navigator.hid`. If worker construction or acquisition fails, run the adapter on the main
  thread and forward input/FFB through the SAB, with a documented latency penalty.
- Generic USB-PID detection is heuristic. A device that enumerates is not automatically
  proven to support FFB. Keep the explicit support matrix and a manual test checklist.
- Vendor protocols may need periodic force updates rather than a single latched effect.
  Profile capabilities must declare this, and the adapter must honor a safe update rate.

**FFB safety contract:**

- Conservative default strength; hard clamp in the adapter.
- Watchdog: if no physics FFB update arrives for 50 ms, send `stop()`.
- Send `stop()` on disconnect, error, pause, page hide, and worker termination.
- Explicit user opt-in; no silent auto-strong mode.
- Provide an emergency stop control in the UI.
- Log device, profile, and last error for diagnostics.

**Device support matrix (MVP):**

| Device | Input | FFB | Status |
| --- | --- | --- | --- |
| CAMMUS C5 | steering/buttons/axes | constant force (USB-PID) | Verified, Windows 11 + Chrome |
| CAMMUS C12 | steering/buttons/axes | constant force (USB-PID) | Profile present, verify |
| FFBeast | steering | constant force (vendor) | Profile present, verify |
| Generic USB-PID wheel | generic HID | constant force (auto-detect) | Best effort |
| Logitech / Fanatec | generic/vendor | not supported in MVP | Future vendor profiles |

Chromium-only. Firefox and Safari fall back to Gamepad API or keyboard; no WebHID FFB.

**Licensing and clean-room note:** `webhid-wheel` is MIT-licensed and may be used directly
with attribution. Do not copy code from GPL drivers such as `new-lg4ff` or
`hid-fanatecff`. Use public protocol documents and black-box traffic observation for future
vendor profiles.

### Module 5: `sim-render` (wgpu Graphics Engine)

**Responsibility:** Render the simulation using WebGPU, with a best-effort WebGL2 fallback.

**Key requirements:**

- Runs on the main thread for MVP. The web frontend owns the canvas and event loop; `winit`
  is not used on web.
- Consume versioned asset packs and flat vertex/index buffers from the asset compiler.
- Forward-rendered toon/flat-shaded materials. PBR is not required. Vertex colors are the
  default; textures are allowed where practical (dashboard gauges, signs, decals, sprites,
  liveries) but not as full PBR sets. See `docs/art-style.md`.
- Object-space inverted-hull outlines and procedural grid shaders. No screen-space edge
  filters or post-processing that would break stereo depth.
- Instanced track-side objects, frustum culling, and LOD selection.
- Read two completed snapshots from the SAB and interpolate by timestamp. Never render a
  snapshot that failed its seqlock check.
- Handle device/context loss: pause rendering, request a new device, and restore resources.
- WebGPU is primary. If unavailable, attempt a WebGL2 path where the wgpu build supports it;
  otherwise show an explicit unsupported message. Do not silently run a degraded renderer
  that misrepresents simulation state.
- Pin `wgpu` to a browser-compatible release; current target is `wgpu` 30.x. wgpu 0.19 is
  not acceptable because current Chrome rejects its WebGPU limits. Use
  `new_instance_with_webgpu_detection` so WebGPU is preferred and WebGL2 remains the fallback.
- Build the wasm release with LTO, `panic = "abort"`, and `wasm-opt -O3`; record raw and gzip
  sizes. The renderer backend and surface format must be visible in telemetry for browser
  diagnostics.
- WebGPU canvas formats are commonly non-sRGB (`Bgra8Unorm`). Do not assume the surface
  format is sRGB: either use an sRGB view where supported or apply the sRGB transfer
  function in the shader. A linear shader written directly to a non-sRGB canvas produces a
  visibly dark sky, ground, and materials.

### Module 6: `sim-audio` (AudioWorklet DSP)

**Responsibility:** Synthesize engine and vehicle audio from simulation snapshots.

**Key requirements:**

- Runs in an `AudioWorkletGlobalScope`; process quanta are 128 frames (about 2.7 ms at
  48 kHz), not 1 kHz.
- Read snapshots from the SAB using the same seqlock rules. Never block, spin, or allocate
  in `process()`. If a read is inconsistent, reuse the last valid parameters.
- Interpolate RPM, load, throttle, and vehicle speed between snapshots.
- Engine synthesis should use order-based harmonics and load-dependent timbre, not a single
  RPM-to-frequency mapping.
- Load the wasm DSP with a small JS shim. If wasm is unavailable in the worklet, fall back to
  a reduced JS synth or silence with a diagnostic.
- Audio must start from a user gesture and must not prevent the page from becoming idle.

### Module 7: `asset-compiler` (Offline Binary Baker)

**Responsibility:** Convert community mod assets into versioned engine assets.

**Key requirements:**

- Native CLI (`cargo run --bin asset-compiler`). `std::fs` is allowed here only; runtime
  crates must not depend on native filesystem APIs.
- Parse `.ini` physics configuration and glTF/`.kn5` visual hierarchies. `.kn5` parsing must
  be an original clean-room implementation for user-supplied assets. Do not redistribute
  copyrighted content.
- Serialize with a pinned `rkyv` version. Write a versioned manifest with magic, schema
  version, source hash, asset hashes, chunk table, and alignment requirements.
- Guarantee alignment for zero-copy reads. When a runtime buffer is not naturally aligned,
  copy into an aligned allocation before `rkyv` access.
- Validate every archive before use (`rkyv` bytecheck/`access` with validation). Never treat
  untrusted mod data as trusted. Reject invalid archives, out-of-range values, and
  oversized/malformed inputs. Fuzz the parser.
- Convert units and axes into the engine frame (meters, `+Y` up, `-Z` forward).
- Flatten meshes into contiguous vertex/index buffers with bounds, materials, LOD, and
  instancing metadata. Use compressed texture formats (KTX2/Basis) where supported.
- Chunk large assets for streaming and cache them in IndexedDB keyed by content hash.
- Keep the legacy `.car`/`.trk` naming if desired, but the format is a versioned manifest
  plus chunks, not a single opaque blob.

### Module 8: Web Frontend (`web/`)

**Responsibility:** Bootstrap, UI, device configuration, SAB allocation, and fallbacks.

**Key requirements:**

- Check `crossOriginIsolated` before allocating the SAB. If false, show a clear error and
  offer a degraded no-SAB mode (main-thread physics at reduced rate, no FFB) rather than
  silently failing.
- Ensure all cross-origin assets are compatible with COEP (CORS or
  `Cross-Origin-Resource-Policy`). An asset that cannot satisfy the isolation policy must be
  same-origin, bundled, or explicitly rejected.
- Allocate the fixed-size SAB on the main thread before starting workers. SAB size is fixed;
  do not rely on growable shared memory.
- Provide a device authorization flow and a device/profile diagnostics panel.
- Display tick, step time, worker drops, FFB state, and device diagnostics.
- Handle visibility changes: pause and send FFB stop when the page is hidden.
- Provide keyboard and Gamepad API fallbacks for non-WebHID browsers.

---

## 4. Shared State Memory Contract (`SharedArrayBuffer`)

The SAB is a fixed 4096-byte block. All multi-byte values are little-endian. All `f32x4`
groups start at 16-byte-aligned offsets. The layout is versioned; unknown versions must be
rejected.

### 4.1 Layout

| Region | Offset | Size | Purpose |
| --- | ---: | ---: | --- |
| Header | 0 | 64 | Magic, version, slot metadata, latest slot |
| Snapshots | 64 | 1024 | 4 slots × 256 bytes |
| Input mailbox | 1088 | 64 | HID worker -> physics |
| FFB mailbox | 1152 | 64 | Physics -> HID worker |
| Reserved | 1216 | 2880 | Future use; must be zero-initialized |

#### Header (offset 0, 64 bytes)

| Offset | Type | Field |
| ---: | --- | --- |
| 0 | u32 | Magic (`LSIM`) |
| 4 | u16 | Layout major version |
| 6 | u16 | Header size |
| 8 | u32 | Slot count (4) |
| 12 | u32 | Slot stride (256) |
| 16 | u32 | Latest published slot index (atomic) |
| 20 | u32 | Flags |
| 24 | u32 | Writer tick (atomic, diagnostic) |
| 28 | u32 | Reserved |
| 32 | f64 | Writer time base, seconds (absolute) |
| 40 | u8[24] | Reserved |

#### Snapshot slot (offset `64 + index * 256`, 256 bytes)

| Slot offset | Type | Field |
| ---: | --- | --- |
| 0 | u32 | Slot seqlock (atomic; odd = writing) |
| 4 | u32 | Reserved |
| 8 | u64 | Physics tick |
| 16 | f64 | Absolute timestamp, seconds |
| 24 | u8[8] | Reserved (alignment) |
| 32 | f32x4 | Position (x, y, z, pad) |
| 48 | f32x4 | Rotation quaternion (x, y, z, w) |
| 64 | f32x4 | World velocity (x, y, z, pad) |
| 80 | f32x4 | Angular velocity (x, y, z, pad) |
| 96 | f32x4 | Wheel angular speeds (FL, FR, RL, RR) |
| 112 | f32x4 | Suspension travel (FL, FR, RL, RR) |
| 128 | f32x4 | Engine RPM, rack torque, throttle, brake |
| 144 | f32x4 | Slip ratio (FL, FR, RL, RR) |
| 160 | f32x4 | Slip angle (FL, FR, RL, RR) |
| 176 | f32x4 | Tire normal load (FL, FR, RL, RR) |
| 192 | f32x4 | Contact flags per wheel (0 or 1) |
| 208 | f32x4 | Surface friction per wheel |
| 224 | u8[32] | Reserved |

#### Input mailbox (offset 1088, 64 bytes)

| Offset | Type | Field |
| ---: | --- | --- |
| 0 | u32 | Seqlock (atomic; odd = writing) |
| 4 | u32 | Button bitmask |
| 8 | f64 | Device timestamp, seconds |
| 16 | f32 | Steering, -1..1 |
| 20 | f32 | Throttle, 0..1 |
| 24 | f32 | Brake, 0..1 |
| 28 | f32 | Clutch, 0..1 |
| 32 | i32 | Requested gear |
| 36 | u8[28] | Reserved |

#### FFB mailbox (offset 1152, 64 bytes)

| Offset | Type | Field |
| ---: | --- | --- |
| 0 | u32 | Seqlock (atomic; odd = writing) |
| 4 | u32 | Flags: bit 0 active, bit 1 stop request |
| 8 | f64 | Physics timestamp, seconds |
| 16 | f32 | Normalized torque request, -1..1 |
| 20 | u8[44] | Reserved |

### 4.2 Seqlock protocol

Every mailbox and every snapshot slot uses an even/odd seqlock.

Writer:

1. Read the current sequence and store an odd value (`seq + 1`).
2. Write all fields.
3. Store the next even value (`seq + 2`).

Reader:

1. Load the sequence. If odd, retry or use the last known value.
2. Read all fields.
3. Load the sequence again. If it changed, retry or use the last known value.

Rules:

- `Atomics` operations are required for sequence and publication fields.
- Do not assume individual `f32` reads are atomic or mutually consistent.
- Physics and audio must not spin on a seqlock. Physics may retry a bounded number of times;
  audio reuses the last valid parameters.
- Snapshot slots are single-writer (physics). Input and FFB mailboxes are single-writer.
- Use four snapshot slots so a reader has at least 4 ticks of history. A writer may overwrite
  a slot only after publishing a newer one.

### 4.3 Publication and interpolation

- Physics writes a snapshot into `slot = tick % 4`, completes the slot seqlock, then publishes
  `latest_slot` with an atomic store.
- Readers load `latest_slot` twice around the slot read. If the index changed, use the
  previous slot or retry once.
- Rendering selects the two newest completed snapshots by tick/timestamp and interpolates
  position, rotation, wheel angles, and camera targets. Clamp `alpha` to `[0, 1]`.
- Use an absolute time base (`performance.timeOrigin + performance.now()`) or exchange and
  verify a startup offset. Do not assume `performance.now()` values are directly comparable
  across contexts without verification.
- Render pressure must never feed back into physics. Physics runs on its own deadline.

### 4.4 Input and FFB flow

- HID worker writes input with a seqlock and device timestamp. Physics reads the latest input
  once per tick and reuses the last valid input if the read is inconsistent.
- Physics writes the FFB torque request every tick. The HID worker reads the latest value,
  coalesces, rate-limits, and sends it to the device. Stale requests older than 50 ms trigger
  `stop()`.

---

## 5. Scheduling, Timing, and Degradation

- **Physics:** 500 Hz default. 1 kHz is disabled unless the reference device meets the
  performance budget. Use a deadline loop, bounded catch-up, and explicit starvation
  diagnostics.
- **FFB:** 100-250 Hz device updates, coalesced from the latest physics request. Never block
  the physics worker on HID writes.
- **Render:** `requestAnimationFrame`, interpolated from snapshots.
- **Audio:** 128-frame render quantum, interpolated from snapshots.
- **Background tabs:** browsers throttle workers. Pause simulation and send FFB stop when the
  page is hidden. Do not attempt to simulate in the background.
- **Degradation ladder:**
  1. Full mode: cross-origin isolated, SAB, workers, WebGPU, WebHID FFB.
  2. No WebHID: input via Gamepad API/keyboard, no FFB.
  3. No SAB: main-thread physics at a reduced rate with `MessageChannel` state, no FFB.
  4. No WebGPU: WebGL2 if available, otherwise an explicit unsupported message.

There is no claim of guaranteed 1 kHz operation. Measure on reference hardware and publish
the result.

---

## 6. Asset and Coordinate Conventions

- Units: meters, kilograms, seconds, radians.
- Coordinate frame: right-handed, `+Y` up, `-Z` forward.
- Quaternions: `(x, y, z, w)`, normalized every tick.
- Steering: positive = left or right as documented once, consistently, in sim, render, HID,
  and asset conversion.
- Tire slip: document the sign convention for kappa and alpha in `sim-core` and keep the
  asset compiler consistent.
- Asset compiler converts source-asset axes and units into this frame; runtime code must not
  contain per-format axis fixups.

---

## 7. Licensing, Security, and Compliance

### Licensing

- Engine code must be original or permissively licensed (MIT/Apache-2.0 compatible).
- `webhid-wheel` is MIT. Include its license in `NOTICE` and pin the version.
- Run `cargo-deny` and `cargo-about`, and generate an SBOM.
- Do not copy code from GPL projects. Public protocol documents and black-box observations
  are acceptable references for clean-room vendor profiles. rustyAC's vehicle physics is
  GPL-3.0-or-later and is reference-only.
- Physics/collision backends must be permissively licensed: Rapier Apache-2.0, Jolt MIT,
  Box3D MIT. ODE is dual LGPL/BSD but is not adopted. Ammo.js is zlib/Bullet; cannon-es is
  MIT. None are required by the MVP.
- Do not bundle copyrighted vehicle, track, or audio assets.

### Security

- Validate every deserialized asset. Treat mod files as untrusted input.
- Enforce size, count, and recursion limits in the asset compiler and runtime loader.
- Do not expose filesystem paths, device serial numbers, or raw HID reports in logs by
  default.
- Use a strict Content Security Policy. WebHID access is permission-gated and must be
  explained in the UI.
- FFB is a physical actuator. The safety contract in Module 4 is mandatory.

### Privacy

- No telemetry or analytics by default.
- Device diagnostics are opt-in and local unless the user explicitly exports them.

---

## 8. Testing, CI, and Acceptance Criteria

### Required checks

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p sim-types -p sim-core -p sim-worker -p sim-render -p sim-audio \
  --target wasm32-unknown-unknown
cargo deny check
npm ci
npm run build
npm test
```

### Test layers

- **sim-types:** offset/size assertions and TypeScript mirror tests.
- **sim-core:** unit tests for tire forces, low-speed handling, steering torque, and
  integration stability; property tests for finite state; golden replay traces.
- **Collision backend:** raycast/contact conformance tests and golden replay traces; run
  before and after adding Rapier or Jolt.
- **HID adapter:** `webhid-wheel` parser fixtures and recorded report tests in JS; manual
  hardware checklist for each supported device.
- **SAB:** seqlock stress tests with a writer thread and a reader thread; no torn snapshots.
- **Assets:** fuzzing, malformed archive rejection, alignment tests, version rejection.
- **Browser:** Playwright smoke tests in Chromium (load, render, no console errors). WebHID
  hardware selection remains a manual test.
- **Performance:** in-browser traces, not native microbenchmarks alone.

### Performance budgets (reference: desktop Chromium, 4-core CPU, integrated GPU)

| Metric | Target |
| --- | --- |
| Physics step p99 at 500 Hz | <= 1.0 ms |
| Physics step p99 at 1 kHz | <= 0.8 ms |
| Missed deadlines | < 0.1% over 60 s |
| Catch-up events | 0 over 60 s |
| FFB update rate | 100-250 Hz, bounded queue |
| FFB stop latency | <= 50 ms |
| Render | 60 fps at 1080p |
| Physics wasm payload | <= 3 MB gzip |
| Initial app payload (no assets) | <= 10 MB gzip |
| Cached start | <= 2 s |

### Browser/device matrix

| Platform | Rendering | SAB | WebHID FFB |
| --- | --- | --- | --- |
| Chromium desktop (Windows primary) | WebGPU | Yes, isolated | Yes, supported devices |
| Chromium desktop (Linux/macOS) | WebGPU | Yes, isolated | Best effort |
| Firefox | WebGL2/WebGPU | Yes, isolated | No |
| Safari | WebGPU/WebGL2 | Yes, isolated | No |
| Non-isolated host | WebGL2/WebGPU | No | No |

### MVP definition of done

- One supported wheel drives 500 Hz physics with constant FFB and a working watchdog.
- Render interpolates snapshots smoothly; no torn reads or visible tick aliasing.
- Audio runs without underruns on the reference desktop.
- The asset compiler bakes a sample car and track, and the runtime validates and loads them.
- All required checks pass and documented budgets are met or explicitly waived with data.

---

## 9. Coding Agent Guardrails & Directives

When building or extending this workspace, an AI coding agent must enforce:

1. **Permissive licensing only.** Use MIT/Apache-2.0-compatible dependencies. Do not port
   GPL code. Run `cargo-deny` and record third-party licenses.
2. **Use `webhid-wheel`; do not reimplement HID.** Put HID/FFB in the HID worker and keep it
   behind the TypeScript adapter. Do not add a Rust HID parser.
3. **Keep `sim-core` pure.** No web APIs, no filesystem, no DOM, no worker globals.
4. **Wasm compatibility.** Runtime crates must compile for `wasm32-unknown-unknown` and must
   not use `std::fs` at runtime. `asset-compiler` is the only native filesystem user.
5. **No hot-loop allocations.** The physics tick and audio `process()` must not allocate.
   Verify with instrumentation; do not rely on assertions alone.
6. **No data races.** Use the seqlock and publication rules in Section 4. Never read a
   multi-field snapshot without validating its sequence.
7. **Validate untrusted data.** rkyv archives, mod assets, and HID reports from unknown
   devices must be validated and bounded.
8. **Respect real-time limits.** Do not claim guaranteed 1 kHz or hard real-time behavior.
   Measure, document, and degrade gracefully.
9. **Enforce FFB safety.** Watchdog, clamp, explicit stop, and conservative defaults are
   required, not optional.
10. **Do not use `winit` on web.** The web frontend owns the canvas and event loop.
11. **Test at the boundaries.** SAB layout, wasm target builds, asset validation, and HID
    adapter parsing need dedicated tests. Manual hardware tests are required for FFB.
12. **Report supported devices honestly.** Do not claim universal direct-drive support. The
    device matrix in Module 4 is the current contract.
13. **Keep vehicle dynamics first-party.** Do not delegate tire, suspension, drivetrain, or
    steering behavior to a general-purpose engine's vehicle controller.
14. **Keep collision backends replaceable.** Access collision through the trait; do not leak
    Rapier, Jolt, or other backend types into the vehicle model.
15. **Do not port GPL physics.** rustyAC and similar GPL vehicle-physics implementations are
    reference material only; do not copy their code.
