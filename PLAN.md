# Track and Car Model Loading Plan

> Live implementation status: see `PROGRESS.md`.

## 1. Objective

Deliver a reliable pipeline that loads one car and one track from source assets into the
runtime, ready for rendering and physics, with early go/no-go checkpoints. The plan avoids
starting with the hardest parser (`.kn5`); it proves the asset contract and runtime first
using synthetic fixtures, then introduces real mod assets behind isolated checkpoints.

## 2. Strategy

- **Contract first.** Define the manifest, chunk layout, units, axes, and runtime loading
  contract before writing format parsers.
- **Synthetic assets first.** Use a tiny hand-authored car and track to validate the full
  pipeline in CI without licensed content.
- **Vertical slices.** Every checkpoint ends with a runnable or inspectable artifact: a CLI
  report, a viewer, or a driveable scene.
- **Real assets last, but early enough.** Introduce a real car `.ini`/`.kn5` immediately
  after the synthetic slice works, then a real track.
- **Measure at every gate.** Record load time, memory, triangle counts, draw calls, raycast
  cost, and failure modes.
- **Clean-room and legal.** Real mod assets are user-supplied and never committed. `.kn5`
  parsing is original work; do not copy GPL drivers or parsers.

## 3. Pipeline Overview

```text
source assets (.ini, .kn5, glTF, textures)
        |
        v
asset-compiler (native CLI)
  - parse and normalize
  - convert units/axes
  - extract physics, visual, collision, metadata
  - bake rkyv chunks + manifest
        |
        v
versioned asset pack (manifest + chunks)
        |
        v
runtime loader (web/src/assets)
  - fetch with progress
  - verify hash, schema, version
  - validate rkyv archives
  - align buffers, cache in IndexedDB
        |
        +-------------------+
        v                   v
sim-worker              sim-render
physics + collision      meshes, materials, textures
```

### Loading sequence

1. Fetch car and track manifests in parallel.
2. Validate schema version, engine units/axes, required chunks, and dependency hashes.
3. Fetch critical chunks first: car physics, track metadata, track collision, spawn data.
4. Start physics with the car at the spawn point.
5. Fetch visual chunks and textures with progress reporting; hand them to the renderer.
6. Run warm-up ticks, then start input, audio, and FFB.
7. On unload, release GPU resources, wasm archive handles, and IndexedDB leases.

## 4. Asset Contract

### Common manifest

| Field | Purpose |
| --- | --- |
| `magic`, `schema_version` | Format identity and compatibility |
| `kind` | `Car` or `Track` |
| `id`, `source_hash` | Stable identity and cache key |
| `units` | `m`, `kg`, `s`, `rad` |
| `frame` | Right-handed, `+Y` up, `-Z` forward |
| `bounds` | Local/world-space AABB for culling and sanity checks |
| `chunks[]` | Offset, length, alignment, hash, compression, schema |
| `entrypoints` | Physics, visual, collision, metadata, audio |
| `stats` | Vertices, triangles, materials, textures, estimated draw calls |
| `warnings[]` | Ignored or approximated source data |

### Car chunks

- `physics`: mass, inertia, center of gravity, wheelbase, track width, wheel radius,
  suspension, tire coefficients, engine torque curve, gear ratios, final drive, clutch,
  brakes, steering geometry, aero.
- `visual`: car body meshes, wheel meshes, steering wheel, materials, texture references,
  wheel node transforms, lights.
- `collision`: optional chassis collision boxes or convex hulls.
- `audio`: optional engine/exhaust metadata.

### Track chunks

- `visual`: road, terrain, buildings, props, materials, texture references, LODs,
  instancing metadata.
- `collision`: triangle mesh with surface IDs, road/wall boundaries, pit lane geometry.
- `metadata`: spawn points, pit boxes, cameras, sectors, checkpoints, track length,
  surface friction table.
- `audio`: optional ambient/reverb zones.

### Content rules

- The asset compiler converts source units and axes into the engine frame. Runtime code does
  no per-format axis fixups.
- Chunks are independently hashed and validated. Missing required chunks are a hard error.
- Unsupported source features are warnings, not silent drops.
- Real assets are excluded from git except for a manifest documenting the manual test setup.

## 5. Early Checkpoints

These checkpoints must pass before work begins on a full `.kn5` parser or a real track.
Suggested timeboxes assume one focused developer; adjust after CP0.

### CP0: Contract, fixtures, and test harness

**Indicative timebox:** 2-3 days.

**Goal:** Lock the data contract and create CI-safe synthetic assets.

**Tasks:**

- Define `AssetManifest`, `ChunkRef`, `CarPhysics`, `CarVisual`, `TrackMetadata`,
  `TrackCollision`, and `TrackVisual` in `sim-types` with magic and schema version.
- Define offset/size assertions and a TypeScript mirror for the runtime loader.
- Create synthetic fixtures:
  - `fixtures/tracks/minimal`: a plane, two walls, a start line, a spawn point, and
    labeled X/Y/Z axes.
  - `fixtures/cars/minimal`: a box chassis, four cylinder wheels, wheel node metadata, and
    a hand-authored physics configuration with known values.
- Add a `.gitignore` rule and a manual test checklist for real user-supplied assets.
- Add a viewer/debug page skeleton that can draw a unit cube and axes.

**Exit criteria:**

- `sim-types` layout tests pass natively and for `wasm32-unknown-unknown`.
- Manifest schema is documented and versioned.
- Fixtures are deterministic and checked in.
- The axes fixture visibly confirms `+Y` up and `-Z` forward.

**Fail action:** Do not write any parser or renderer code until the contract and fixtures
are stable. A later change to the contract is more expensive than a delay here.

### CP1: Bake and load round trip (no graphics, no physics)

**Indicative timebox:** 2-3 days.

**Goal:** Prove the compiler and runtime loader agree byte-for-byte.

**Tasks:**

- Implement `asset-compiler bake car <dir>` and `bake track <dir>` for the synthetic
  fixtures.
- Implement `asset-compiler inspect <pack>` to print manifest, bounds, chunk table, triangle
  counts, wheel nodes, and physics parameters.
- Implement `asset-compiler validate <pack>` to check hashes, alignment, schema versions,
  and required chunks.
- Implement the runtime loader in `web/src/assets`:
  - fetch manifest and chunks;
  - verify hash and version;
  - validate rkyv archives with bytecheck;
  - copy into aligned buffers;
  - return typed handles.
- Add a Worker-based loader test and an IndexedDB cache keyed by
  `asset id + source hash + schema version + compiler version`.
- Add deterministic bake tests: the same source produces the same manifest and chunk hashes.

**Exit criteria:**

- `bake` -> `inspect` -> `validate` succeeds for both fixtures.
- The runtime loader loads both packs inside a Worker and returns expected values.
- Corrupt hash, wrong version, missing chunk, and unaligned archive are rejected with typed
  errors.
- Fixture load time and memory are recorded.

**Fail action:** Do not start the renderer integration. If the round trip fails, fix the
container, alignment, and validation first.

### CP2: Track render-only vertical slice

**Indicative timebox:** 2-3 days.

**Goal:** Render the synthetic track correctly from a baked pack.

**Tasks:**

- Add `sim-render` track loading: vertex/index buffers, materials, textures, camera,
  frustum culling (optional for the fixture), and debug overlays.
- Add a `web/track-viewer` page that loads the fixture pack and renders it.
- Display the origin axes, bounds, spawn point, and surface IDs.
- Add a golden screenshot or image-diff test in Chromium.
- Report draw calls, triangles, texture memory, and frame time.

**Exit criteria:**

- Track geometry is not mirrored, scaled incorrectly, or rotated incorrectly.
- Spawn point and bounds match the fixture metadata.
- The viewer runs at 60 fps on the reference desktop.
- Reloading the page uses the cached pack and skips the network fetch.

**Fail action:** Fix coordinate conversion, winding order, material binding, or texture
color space before adding the car. Do not compensate in runtime code.

### CP3: Car render-only vertical slice

**Indicative timebox:** 2-3 days.

**Goal:** Render the synthetic car on the synthetic track with correct wheel placement.

**Tasks:**

- Add `sim-render` car loading: body mesh, four wheel meshes, wheel node transforms,
  steering wheel, materials.
- Spawn the car at the track spawn point.
- Add a `web/car-viewer` or combined `web/garage` page.
- Assert wheel count, wheel radius, track width, and bounding box.
- Add a screenshot test for the combined scene.

**Exit criteria:**

- Car scale and orientation are correct on the track.
- Wheels are attached at the expected positions and can be rotated/pivoted by debug input.
- Car materials and textures display correctly.
- No physics is required yet.

**Fail action:** Fix the visual chunk schema or node-transform flattening before wiring
physics. Wheel placement bugs become much harder to diagnose once the car is moving.

### CP4: Drivable synthetic vertical slice

**Indicative timebox:** 4-5 days.

**Goal:** The synthetic car drives on the synthetic track using baked physics and collision.

**Tasks:**

- Map `CarPhysics` into `sim-core` parameters; add defaults and validation for missing
  values.
- Load the track collision chunk and surface table; implement suspension raycasts.
- Spawn the car; apply gravity; settle on suspension; reset on failure.
- Add keyboard input, HUD, snapshot interpolation, and a reset control.
- Run 500 Hz physics; record step p50/p99, catch-up events, and NaNs.
- Add an automated soak test: 60 seconds of scripted input on the fixture.

**Exit criteria:**

- Car settles on its suspension without jitter or NaN.
- Throttle, brake, steering, and reverse work; the car stays on the track.
- HUD shows speed, RPM, gear, wheel loads, and contact state.
- Scripted 60-second drive completes without instability or unbounded catch-up.
- No asset reload or heap growth during the drive.

**Fail action:** Do not start real car or track work until the synthetic scene is stable.
Most coordinate, collision, and physics-mapping bugs surface here.

## 6. Real Asset Checkpoints

### CP5: Real car model and physics

**Indicative timebox:** 5-8 days.

**Goal:** Load one real, user-supplied car without manual asset edits.

**Tasks:**

- Implement `.ini` parsing for the supported physics subset. Emit a validation report for
  ignored keys, out-of-range values, and derived defaults.
- Implement clean-room `.kn5` (or glTF fallback) parsing for the car visual hierarchy.
- Convert textures to KTX2/Basis where needed.
- Extract wheel nodes, steering wheel, materials, and LODs.
- Run the real car on the synthetic track.
- Compare key physics values against the source and document tolerances.
- Add recorded HID/input replay for regression testing where possible.

**Exit criteria:**

- Real car visual loads and matches the source in the viewer.
- Real car physics loads and drives on the synthetic track.
- Unsupported source features are listed; no silent data loss.
- The car does not depend on fixture-specific assumptions.

**Fail action:** Keep the car on the synthetic track until both visual and physics pipelines
are stable. Do not combine car and track debugging.

### CP6: Real track model

**Indicative timebox:** 5-10 days.

**Goal:** Load one real, user-supplied track and drive the real car on it.

**Tasks:**

- Parse real track `.kn5` and `.ini`: visual hierarchy, materials, textures, collision mesh,
  surface types, spawn points, pit lane, cameras, sectors.
- Bake track chunks; add streaming/LOD only if the budget requires it.
- Implement collision-surface mapping and validate wall/road boundaries.
- Add a collision-mesh debug view.
- Load the real car and track together; complete a manual lap.
- Record load time, wasm memory, texture memory, draw calls, and raycast cost.

**Exit criteria:**

- Track visual matches the source in the viewer.
- Collision mesh aligns with the visual road and walls; no falling through.
- Surface friction, spawn, pit lane, sectors, and cameras work.
- The real car can complete a lap under manual input.
- Load time and memory are within the documented budgets, or a waiver is recorded.

**Fail action:** If track scale/axes or collision are wrong, return to the asset compiler
and fix normalization. Do not add runtime workarounds.

### CP7: Integration and performance

**Indicative timebox:** 4-6 days.

**Goal:** Full car + track + input + audio + FFB loop with production loading UX.

**Tasks:**

- Wire HID input, FFB, audio, rendering, and physics together.
- Add a loading screen with progress, cancellation, retry, and typed error messages.
- Add IndexedDB caching, cache invalidation, and offline behavior.
- Add memory disposal and context-loss recovery.
- Add streaming/LOD if budgets are exceeded.
- Run browser performance traces and record the final numbers.

**Exit criteria:**

- Full session starts from a cold cache and a warm cache within budget.
- No asset reloads during gameplay; no torn snapshots.
- FFB watchdog and stop behavior work with the real car and track.
- All budgets in SPEC section 8 are met or explicitly waived with data.
- Chromium smoke test passes; fallback browsers render and accept keyboard/gamepad input.

### CP8: Hardening and release

**Indicative timebox:** 4-6 days.

**Goal:** Make the loading pipeline robust, documented, and maintainable.

**Tasks:**

- Fuzz the asset compiler and runtime validator with malformed and adversarial inputs.
- Test schema-version migration and cache invalidation.
- Add error-path tests for missing, corrupt, oversized, and unsupported assets.
- Audit licensing: no committed copyrighted assets, no GPL code, NOTICE updated.
- Document the mod workflow, supported features, troubleshooting, and device matrix.
- Add CI jobs for fixtures, golden manifests, wasm target checks, and loader tests.

**Exit criteria:**

- Corrupt or unsupported assets fail gracefully with actionable errors.
- Bake output is reproducible and hashes are stable.
- CI is green; docs cover the full workflow.
- A new contributor can bake and load the fixture car and track from the docs alone.

## 7. Test and Fixture Matrix

| Fixture | Committed | CI | Manual | Purpose |
| --- | --- | --- | --- | --- |
| `cars/minimal` | Yes | Yes | No | Compiler/loader/physics contract |
| `tracks/minimal` | Yes | Yes | No | Visual, collision, spawn contract |
| Real car `.ini` + `.kn5` | No | No | Yes | Parser and physics validation |
| Real track `.kn5` | No | No | Yes | Track parser, collision, surfaces |
| Malformed assets | Yes | Yes | No | Rejection and error UX |
| Large track stress fixture | Generated | Yes | No | Streaming, memory, load time |

## 8. Risks and Mitigations

| Risk | Mitigation |
| --- | --- |
| `.kn5` parser becomes the critical path | Prove the container and viewer with glTF first; isolate parser behind a trait; add a dedicated viewer and fuzz tests |
| Units or axes are wrong | CP0 axes fixture; compiler owns conversion; no runtime fixups |
| Collision mesh does not match visuals | Bake surface IDs; add a collision debug overlay; compare road/wall alignment |
| Missing or unsupported physics parameters | Validation report; documented defaults; fail loudly for required values |
| Real asset size causes memory pressure | Chunking, compression, KTX2, LOD, streaming; track budgets at CP6 |
| Loader and compiler drift | Shared `sim-types` schema; manifest hash; golden manifests in CI |
| Worker transfer copies too much data | Transferable `ArrayBuffer`s; aligned copies only when required |
| COOP/COEP blocks assets | Test static hosting early; require CORP/CORS on all assets |
| Legal contamination | User-supplied assets; clean-room parser; no GPL code; NOTICE |
| Real assets unavailable during development | Synthetic fixtures cover CI; real assets only for manual gates |

## 9. Immediate Next Actions

1. Create `sim-types` manifest and chunk structs with offset/size tests.
2. Create the minimal car and track fixtures, including labeled axes.
3. Implement `asset-compiler bake` and `inspect` for the fixtures.
4. Implement the Worker loader round trip and rejection tests.
5. Render the synthetic track and verify axes, bounds, and spawn.
6. Add the synthetic car, then make it drive on the synthetic track.

Do not begin `.kn5` parsing until CP1 through CP4 are green.

## 10. Definition of Done

- Versioned car and track packs bake reproducibly.
- The runtime loads, validates, caches, and disposes both packs.
- A real car and a real track load without manual asset edits.
- The car drives on the track with collision, surfaces, spawn, and cameras working.
- Loading UX reports progress and actionable errors.
- Performance budgets are met or documented with waivers.
- CI covers fixtures, loader errors, layout, wasm targets, and golden manifests.
- Licensing and clean-room constraints are satisfied.
