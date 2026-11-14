# Project Progress

Last updated: 2026-11-13
Current phase: **Venue graph rebuild — `@6` section-point reset (Phase 3)**

This file is the resumable project state. Read `GOALS.md`, `SPEC.md`, `PLAN.md`,
and this file before continuing. Detailed history is archived in
`docs/history/progress-archive-2026-11-08.md`.

## Resume checklist

```bash
# Run these from the repository root.
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p sim-types -p sim-asset -p sim-core -p sim-render -p sim-worker \
  --target wasm32-unknown-unknown

npm --prefix web ci
npm --prefix web test
npm --prefix web run test:viewer
```

Build the browser wasm and pages:

```bash
npm --prefix web run build:viewer
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

Expected browser: Chromium/Chrome. WebGPU is used when available; WebGL2 is the
fallback.

Deployment:

- Live site: https://joegaffey.com/lapster/
- Dev preview: https://joegaffey.com/lapster-dev/ via
  `gh workflow run pages.yml -R joegaffey/lapster-dev -f ref=<branch-or-tag>`

## Current focus

- Venue graph rebuild is in progress on `feat/venue-graph`. The `@5` phases
  added a canonical graph, decoration paths, and a project editor API. ADR 0003
  resets the model to `lapster/track-project@6` (global points, open/closed
  sections, route legs, loose ends, decoration fraction chains), and ADR 0004
  replaces Catmull-Rom with a cubic B-spline: all authored control points are
  off-curve handles; start/finish and junction contact points are derived on the
  road. The render side is migrated and green (70 tests); the web editor boots
  on `@6`, selects/drags points and sections, persists the project, draws the
  control polygon and dotted contact lines, and has the core point-editing
  tools: click the road to insert, delete, and set start/finish, with point
  details/actions in a non-modal point-editor dialog. Decoration editing is
  mode-based: `Points | Kerbs | Props`, with renderer-derived lane rails,
  baked-default import, click-to-select, endpoint-handle resize, a single-item
  inspector for properties/delete, paint on empty rails, overlap errors, and
  persistence. Section overlays sample at route density (TD-0005 resolved).
  Step 0b junctions and the multi-section decoration follow-up are next.
  Per-point width is deferred (TD-0002, TD-0003).
- Relay multiplayer prototype is working: rooms up to four players, optional
  password, 30 Hz snapshots, interpolation, garage-bay spawns, remote smoke and
  audio. No server physics, car-to-car collision, or automated two-browser test
  yet.
- Next: venue graph Step 0b/web UX (see the resume checkpoint), Speed Dreams
  physics, wheel/FFB hardware validation, and CP7 HID/audio integration.

## Completed checkpoints

| Checkpoint | Status |
| --- | --- |
| CP0 — contracts/fixtures | Done |
| CP1 — bake/load round trip | Done |
| CP2 — track render slice | Done |
| CP3 — car render slice | Done |
| CP4 — drivable slice | Done |
| CP4.5 — toon style slice | Done |
| CP5a — GLB car import | Working |
| CP6a — AC track import | Working |
| Collider BVH | Done |
| Default demo assets | Done |
| Renderer upgrade | Done |
| PWA + COI | Done |
| CP6.5 — WebHID wheel/FFB | Working; hardware validation pending |
| Cockpit dial presentation | Working |
| Speed Dreams car import | Working |
| Relay multiplayer prototype | Working prototype |
| Style spike | Working |

## Current capabilities

- 500 Hz fixed-step physics in the SAB worker by default; `?physics=main`
  selects the main-thread fallback.
- Custom vehicle model: raycast suspension, anti-roll, Pacejka tires, load
  transfer, ABS/TC/ESC, auto gearbox, clutch, engine braking, wall contacts,
  and finite-state reset.
- wgpu 30 renderer with WebGPU/WebGL2 fallback, toon shading, outlines, shadow
  modes, grass, particles, cockpit dials, and time-of-day.
- Versioned asset packs: deterministic bake/validate, GLB/AC car and track
  import, IndexedDB cache, and build-version invalidation.
- Viewers: drive, garage, track viewer, style spike, debug viewer, and
  multiplayer.
- Audio spike: Engine_Sim-based engine audio, tire screech, and surface noise;
  no sample assets or track reverb.
- WebHID wheel/FFB: vendored `webhid-wheel` 0.5.0, multi-device profiles, 200 Hz
  constant force; hardware validation pending.
- PWA + COI: installable, service worker injects COOP/COEP, and
  `SharedArrayBuffer` is verified.
- Wasm: 2.83 MiB raw / 1.06 MiB gzip; warm startup ~300–480 ms headless.

## Verified checks

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` — 142 tests
- `cargo check ... --target wasm32-unknown-unknown`
- `npm test` — 38 tests
- `npm run test:viewer` — 95 passed, 1 skipped
- `npm run bench:physics`
- Playwright runs Chromium with WebGPU; WebGL2 fallback checked separately.
- Manual GLB car and AC track checks passed on WebGPU.

## Known issues / carry-forward

- Wheel/FFB hardware acceptance is pending; CAMMUS C5 first.
- Audio is a spike: external-state compensations, no per-car sample layer,
  Doppler, reverb zones, or mobile listening test.
- Raw `sim-core` engine RPM is wheel-speed-derived and oscillates during
  wheelspin/TC; the HUD and engine torque see the raw signal.
- Traction control and engine braking are simplified; gear selection remains in
  the web UI layer.
- Engine_Sim upstream has no LICENSE file; the vendored copy includes the MIT
  text. Ask upstream before relying on it for release.
- The physics worker stays opt-in until CPU-throttled results are reproduced on
  target hardware.
- GLB/AC importers are first-pass: no full texture decoding/KTX2, decimation, or
  fence/tent collision; AC `.ini` physics is pending.
- Skidpad 0.5 source license/redistribution is unconfirmed; the baked pack is a
  temporary demo fixture.
- Speedometer, RPM, and seven-segment gear images are user-supplied; confirm
  redistribution rights.
- Playwright screenshot baselines are Linux/Chromium-specific and skipped in
  CI; headless WebGPU canvas capture is white in this environment.
- GitHub Pages has no COOP/COEP; the service worker injects them and the first
  controlled load needs one reload.
- Full history: `docs/history/progress-archive-2026-11-08.md`.

## Venue graph resume checkpoint

State on `feat/venue-graph`:

- `@6` render model and B-spline evaluator: authored points are off-curve
  handles; start/finish and junction contacts are derived; section overlays
  sample at route density. `cargo test -p sim-render` — 70 passing.
- Web editor on `@6`: point/section selection, point drag, save/load/export/
  import, Test Drive, dashed control polygon, dotted contact lines, and a
  non-modal point-editor dialog. Click the road to insert, Delete/Backspace or
  the dialog to delete, and Set start/finish in the dialog. Alt/Option-click a
  section centerline to select it.
- Decoration editor: `Points | Kerbs | Props` modes; `editor_guide_rails()`
  lane rails; baked-default import on first entry (28 decorations);
  click-to-select with endpoint handles and resize; a single-item inspector for
  side/lane/prop kind/spacing/phase/elements and Delete; drag an empty rail to
  paint a single-section kerb/prop/tree; overlap errors; save/load persistence.
  Follow-up: cross-section chains, seam wrap, custom lanes, and multi-leg
  endpoint editing (TD-0006).
- Full gate green: 38 web unit tests, 95 viewer tests passed / 1 skipped,
  142 workspace tests, clippy, fmt, wasm. TD-0001 resolved.
- Docs: ADR 0003, ADR 0004, venue-graph plan, tech debt updated.
- Tag `venue-graph-5` marks the last `@5` checkpoint (`d6e0496`).
- `stash@{0}` holds the superseded `@5` selection UX.
- Local page: `http://127.0.0.1:8123/web/track-editor/`.
- Dev preview: `https://joegaffey.com/lapster-dev/web/track-editor/`.

Resume in this order:

1. Step 0b (render): junction contact derivation, branch caps, partial route
   legs, cross-section/seam decoration wrap.
2. Web UX: Make junction + capped branch; then decoration chain continuation,
   seam wrap, custom-lane controls, and multi-leg endpoint editing (TD-0006).
3. Multi-layout and pit lane (Phases 4–5).
4. Full gate, dev preview validation, then a PR to `dev`.

## Next steps

1. **Speed Dreams physics (next)**: parse `sc-deckard-conejo.xml` into
   `CarPhysics`, including mass, CG, torque curve, gears, final drive, brakes,
   aero, springs/dampers, and tire mu. Confirm the reverse-light split visually.
   Move per-car audio and cockpit metadata into the pack before adding more
   Speed Dreams cars.
2. **CP6.5 — wheel/FFB validation**: run the CAMMUS C5 checklist, tune the feel
   mixer, and decide whether the main-thread adapter stays or moves into the
   HID worker.
3. **Vendored `webhid-wheel` validation**: run multi-device profile,
   pedal/button mapping, and worker-compatibility checks; then complete the
   CAMMUS C5 hardware checklist.
4. **CP7 — HID/audio integration**: move the tested FFB adapter into the HID
   worker, add per-car audio profiles or an optional sample layer, expand the
   hardware matrix, and run performance traces. The physics worker, input
   mailbox, snapshot ring, FFB mailbox, and audio worklet spike are in place.
5. **CP5 — real car**: finish the GLB path with decimation, palette/vertex-colour
   tuning, wheel geometry cleanup, and licensed AC `.ini` physics when a
   suitable pack is available. Add a simple steering-wheel/interior placeholder.
   Add real image decoding/KTX2 only where textures are the right choice. WebXR
   remains a later phase.
6. **CP6 — real track**: finish the AC path with full texture decoding
   (PNG/JPEG/DDS/KTX2 where useful), fence/tent collision policy, outer-ring
   collision or streaming, and performance measurement.
7. **Texture pipeline**: add full texture decoding to the GLB and AC importers,
   bake-time texture compression (KTX2/Basis with fallbacks), and texture atlases
   for compatible materials.
8. Replace per-frame JSON telemetry with typed arrays/SAB.
9. Add content-hash cache invalidation.
10. Add Rapier or Jolt only as a collision/query backend if the custom BVH
    becomes limiting.
11. Benchmark a real converted track: draw calls, triangles, texture memory,
    load time, and p99 physics.

## Key files

- `GOALS.md` — vision, goals, success criteria, and non-goals.
- `SPEC.md` — architecture, contracts, and engine policy.
- `PLAN.md` — checkpoint plan.
- `agents/` — multi-agent role cards, contracts, and protocol.
- `crates/sim-core` — custom vehicle physics and collider.
- `crates/sim-asset` — runtime pack validation and access.
- `crates/asset-compiler` — bake/inspect/validate.
- `crates/sim-render` — wgpu renderer and wasm viewers.
- `crates/sim-worker` — Rust/Wasm physics worker wrapper.
- `web/src/audio/engine-sim/` — vendored Engine_Sim MIT DSP.
- `docs/audio.md` — audio architecture, surface model, credits, and deferred work.
- `web/drive`, `web/garage`, `web/track-viewer` — browser pages.
- `web/tests` — Node and Playwright tests.
- `fixtures/cars/minimal`, `fixtures/tracks/minimal` — synthetic fixtures.

## Next-agent prompt

> Read `GOALS.md`, `SPEC.md`, `PLAN.md`, and `PROGRESS.md`. Run the resume
> checklist. Continue with the first item in Next steps. Do not switch the
> vehicle model to a general-purpose engine's vehicle controller; treat
> Rapier/Jolt as optional collision backends.
