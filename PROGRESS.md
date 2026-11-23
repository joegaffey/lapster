# Project Progress

Last updated: 2026-11-22
Current phase: **AI race — motion smoothing and performance controls**

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
  -p track-pack-wasm --target wasm32-unknown-unknown

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
- Garage: `http://127.0.0.1:8123/web/garage/` (concrete studio; add
  `?garage=showroom` for the track-backed showroom)
- Track: `http://127.0.0.1:8123/web/track-viewer/`

Expected browser: Chromium/Chrome. WebGPU is used when available; WebGL2 is the
fallback.

Deployment:

- Live site: https://joegaffey.com/lapster/
- Dev preview: https://joegaffey.com/lapster-dev/ via
  `gh workflow run pages.yml -R joegaffey/lapster-dev -f ref=<branch-or-tag>`

## Current focus

- Start-menu cleanup is on `feat/start-menu-cleanup`: Drive and Race launch
  panels replace the car/track link wall, the selected car opens the Garage,
  Drive uses `race=0` solo mode, Race is limited to Loop Spline Point Terrain
  and marked under development, Track Editor is a main panel, and developer
  views sit under Tools. Drive and Race track selections persist separately.
  Full gate: 245 Rust tests, 76 web tests, 117 viewer passed / 4 skipped, and
  WebGL2 2/2.
- AI motion smoothing is ready for deployment on `feat/ai-motion-smoothing`
  (PR #54), created from `dev` at `0754a7c`: 62.5 Hz AI frames, 40 ms
  interpolation, 20 Hz race state, AI steering rate 3/s, and a configurable
  player steering rate (default 6/s, wheel direct). AI frames are batched into
  one `ai_states` worker message. Graphics adds Tyre effects High/Low/Off, an
  output-resolution readout, and a 50% manual render-scale floor. Full gate
  passed: 233 Rust tests, 76 web tests, 113 viewer passed / 4 skipped, and the
  two WebGL2 fallback tests.
- Concrete garage replaces the Garage showroom on `feat/concrete-garage`
  (from `dev` at `0754a7c`): an eight-car single-line concrete studio with
  overhead spot lights, a procedural floor, a ten-colour paint picker for the
  Mazda, Drive paint propagation, and the Drive flat wheel shading in the
  studio. The Mazda appears once; all six 67GP cars and the Deckard are in the
  catalogue. The old `?garage=showroom` fallback and the showroom
  `GarageViewer::create` constructor are removed. ADR-0016 is superseded by
  ADR-0017. Full gate green; merged to `dev` as `6cb1f3a` (PR #55).
- Loop Spline Point Terrain is added as a fixture and public track option on
  `feat/loop-spline-point-terrain`: 27 MB baked pack, 8 grid slots, and the
  `section-1` race layout. Follow-up `feat/smooth-loop-spline-terrain` raises
  the spline sample density to 32 for a smoother road ribbon.
- AI opponents batch is merged to `dev` as `fc4fdf3` (PR #49): one AI on
  race-ready tracks, simulated in the physics worker, rendered through the
  remote-car path, with no car-to-car contact.
- AI field batch is merged to `dev` as `670312f` (PR #50): three AI by default,
  `?ai=0..3`, `?aiPace=chill|club|fast`, `AiPace` scaling, `AI N` HUD, and
  `aiCount` diagnostics.
- AI race batch is merged to `dev` as `3f83bc8` (PR #52): player grid start,
  countdown, live positions, three laps, and results.
- Editor consolidation is active. `docs/editor-consolidation-report.md` records
  the analysis; ADR-0007 promotes track-ed as the sole editor and ADR-0008
  defers layouts as a future route layer. `docs/features/editor-consolidation-plan.md`
  is the execution plan. A1 and C1–C3 are done: the hidden Kerbs/Walls/Props
  authoring UI is removed from track-ed (`d466d94`), Roadside is the only
  decoration path, and Lapster now has a hosted track-ed Playwright smoke plus
  `track-ed`/`track-ed-hosted` CI jobs, `npm audit` and pin/notice gates. A2/A3
  are also done: `attachEditing` and `createTrackView` are split into
  `editing/` and `scene/` modules behind thin facades. B1 autosave is done and
  B3 is decided in ADR-0010 as a one-release read-only legacy window: the
  legacy nav card is removed and the page carries an export/re-author notice.
  D1 benchmarked a deterministic 10-section venue: ~8.4 s bake and re-bake,
  ~1.5 s validate, ~1.7 s export, so D2 (kernel optimisation, then a Worker)
  is warranted before any Rust/wasm port. Remove the legacy editor only after
  the deprecation window closes (review 2026-12-20).
- Production release **`9a02580`** (PRs #40/#41) ships the consolidation:
  track-ed `main` `cc410dc`, A1/A2/A3/B1/B3/C1–C3/D1, hosted track-ed autosave
  and the legacy-page deprecation notice. The landing's legacy card was
  removed in the #41 hotfix (the `/lapster/` landing is built from the root
  `index.html`).
- track-ed is released to production from `main` (`16da199`, PR #36; latest
  release `9a02580`). The
  shared `crates/track-pack` compiler and `track-pack-wasm` power the hosted
  track-ed **Test drive** and `web/track-import/`; Drive resolves
  `?track=track-ed` from the local pack key `lapster.track-ed.v1`. The
  submodule is pinned to track-ed `main` (`cc410dc` as of production
  `9a02580`), and the Pages build ships
  its notices and licenses. The racing line samples the baked asphalt surface
  (track-ed PR #3). ADR 0006 records the hosting and compiler decision. The
  legacy procedural editor remains as **Legacy Track Editor**; removal is gated
  per ADR-0007 on migration, autosave, CI coverage and a final parity check.
  Production Test drive and the hosted page were verified after the hotfix
  deploy.
- Venue graph rebuild is integrated on `dev` (PR #26). The `@5` phases
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
  Step 0b is integrated on `dev` (`aae590d`): derived junction contacts,
  capped loose-end branches, partial/wrapped route legs, cross-section and
  seam-wrapped decoration chains, and a Make junction editor action.
  Seam-wrapped decoration authoring is complete and merged to `dev` in PR #27
  (`ff67366`): paint across the closed-section seam and endpoint drag across
  the seam both produce a wrapped leg, with Playwright coverage.
  Junction road geometry is still a gap in the legacy editor on `dev`, but it
  is superseded by track-ed's working planar-arrangement junction bake
  (ADR-0007). Do not continue legacy Phase 3d junction work; hosted track-ed is
  the supported path. The independent non-route section/cap shading fix merged
  in PR #29 (`fix/track-section-shading`). Legacy TD-0006 custom-lane and
  junction-chain work is deferred and becomes moot when the legacy editor is
  removed. Per-point width is deferred (TD-0002, TD-0003).
- Imported track rendering is on `feat/imported-track-rendering` (renderer-only,
  editor-independent): GLB imports carry procedural tarmac/grass textures, UVs,
  outline extras, and a distant mountain backdrop, and get screen-space Bayer
  dither fringes at asphalt/dirt/grass boundaries. This is the path track-ed
  exports render through in Lapster.
- Relay multiplayer prototype is working: rooms up to four players, optional
  password, 30 Hz snapshots, interpolation, garage-bay spawns, remote smoke and
  audio. No server physics, car-to-car collision, or automated two-browser test
  yet.
- Next: execute `docs/features/editor-consolidation-plan.md` — remove hidden
  legacy authoring UI from track-ed, split `attachEditing`/`createTrackView`,
  add hosted browser smoke and CI/dependency gates, add autosave, decide legacy
  project migration, then remove the legacy editor. Legacy Phase 3d junction
  work is superseded by track-ed (ADR-0007) and must not be continued. After
  consolidation: Speed Dreams physics, wheel/FFB hardware validation, and CP7
  HID/audio integration.

## Track race MVP resume checkpoint

Branch: `feat/track-race-mvp`, created from `dev`.

- Documentation commit: `6fea5dd` (plan, ADR-0011, registry and progress
  updates).
- `origin/dev` is merged in as `6e94c69` (PR #43, 67GP Guthrie Eagle car).
  There were no conflicts with this branch or the foundation work.
- The contract/compiler foundation is committed on this branch:
  - `sim-types`: `RaceData`, `Layout`, `StartFinish`, `ChunkKind::Race`,
    optional `EntryPoints.race`.
  - `sim-asset`: `Pack::track_race_owned()`.
  - `track-pack`: `sim-track-glb-v1`/`v2` optional race parsing, validation,
    Race chunk and entrypoint.
  - `asset-compiler` and `sim-render`: `race: None` constructor fixes.
  - web loader: optional Race entrypoint support and tests.
  - TS mirror: `web/src/contracts/race.ts`.
  - runtime exposure: `DriveViewer.race()`, plus `raceAvailable`, `layoutId`,
    and `trackSourceHash` in telemetry.
  - web identity: `trackIdentity()` in the pack loader, `RaceView` /
    `parseRaceView`, and contract tests.
  - architect role card/adapter: `web/src/contracts/**` directory permission
    (ADR-0012); confirmed after the server restart.
- After the dev merge and the runtime exposure batch, the full gate passed:
  `cargo fmt --check`, `cargo test --workspace`, workspace clippy, wasm check,
  `npm --prefix web test`, `CI=1 npm --prefix web run test:viewer`
  (105 passed, 1 skipped), the WebGL2 fallback tests, and
  `python3 scripts/check-agent-sync.py`.
- Resolved: the TS mirror exists, the generic contract-root permission works,
  and the grid-inside-road and start/finish-fraction validations are in place.
- Track-ed implicit-layout export is merged into track-ed `main` as
  `e087ee1` (PR #6):
  - closed spawn sections emit the optional `race` block in
    `sim-track-glb-v2` with a centreline layout, start/finish, 8 grid slots,
    and 4 sectors;
  - open spawn sections omit `race` with a warning;
  - track-ed scope docs and the GLB manifest schema were updated.
- The parent `web/track-ed-src` pin is updated to `e087ee1`; the hosted-track-ed
  Test drive now asserts `race()` availability, 8 grid slots, and 4 sectors.
- The race-ready demo fixture is committed at `fixtures/tracks/race-loop/`:
  `project.json` from track-ed's loop project, generated `track.json`/`track.glb`,
  baked `pack.sima`, and a README. The pack carries layout `main`, 180 points,
  8 grid slots, and 4 sectors.
- The launcher exposes **Race Loop** through
  `web/src/ui/track-selection.ts` and `web/index.html`; Drive resolves
  `?track=race-loop`. The default remains Skidpad.
- Coverage: `crates/track-pack/tests/race_loop_fixture.rs` proves the checked-in
  pack matches a fresh compile and carries race data; web pack-loader tests
  cover the Race entrypoint and `chunkBytes("Race")`; `web/tests/race-loop.spec.ts`
  covers launcher selection and the runtime race view; the default Skidpad test
  asserts race-unavailable.
- Final full gate passed: cargo fmt/clippy/workspace tests/wasm; parent
  `npm test` (50); `CI=1 npm --prefix web run test:viewer` (106 passed,
  1 skipped); WebGL2 fallback (2/2); track-ed tests (219 passed, 1 skipped) and
  build; agent sync.
- Next: AI driver, race manager, and online-contact follow-ups. The demo fixture
  is the only shipped track with race data; Skidpad, Drift Tracks, and
  `fixtures/tracks/minimal` remain race-unavailable.
- Full details: `docs/features/track-race-mvp.md` §3.1, ADR-0011, and
  ADR-0012.

## Race manager MVP resume checkpoint

Branch: `feat/race-manager`, created from `dev` after PR #44.

- ADR-0013 approves AI racing and online contact as the next direction; `GOALS.md`
  no longer lists AI opponents as a non-goal. AI implementation remains a later
  batch.
- Contract: `RaceState` in `crates/sim-types/src/race.rs`; mirror
  `web/src/contracts/race-state.ts`; registry updated in `AGENTS.md` and
  `agents/contracts.md`.
- Core: `sim-core::RaceTracker` projects the car onto the layout, derives
  progress/sector, detects forward/backward start-finish crossings, starts the
  lap timer on movement, records last/best lap, and handles reset/degenerate
  inputs.
- Render: `DriveViewer` owns the tracker, updates it on main-thread, worker, and
  reset paths, and exposes `race_state()` plus telemetry `raceState`.
- Web: compact race overlay, `RaceStateView` contract tests, and Race Loop /
  Skidpad browser coverage.
- Track-ed spawn direction: the exporter did not pre-flip the metadata spawn for
  Lapster's runtime drive flip. Fixed in track-ed PR #7 (merged); the Race Loop
  fixture was regenerated, `pack.sima` re-baked, and the parent pin moved to the
  merged track-ed `main` commit 1b22f5a.
- Gate passed: cargo fmt, workspace clippy, 194 workspace tests, wasm check;
  parent `npm test` (57); `CI=1 npm --prefix web run test:viewer` (106 passed,
  1 skipped); WebGL2 fallback (2/2); track-ed tests (220 passed, 1 skipped) and
  build; agent sync. Track-ed PR #7 merged, the parent pin is updated, and the
  parent PR was merged to `dev` as `45fa03d` (PR #47).
- Next: AI opponents batch is complete; AI field batch is active.

## AI opponents resume checkpoint

Branch: `feat/ai-opponents`, created from `dev` at `abb7507`.

- ADR-0014 accepts AI-in-physics-worker: one AI by default, up to three
  supported, deterministic layout following, 500 Hz simulation, and 62.5 Hz
  `ai_state` frames over the worker control channel.
- Plan: `docs/features/ai-opponents.md`.
- No versioned contract change. Reuse `RaceData`, `RaceState`, and the
  64-float `SNAPSHOT_STATE` payload.
- Player stays at the track spawn/start-finish; AI starts at `race.grid[1..]`.
- AI uses `sim_core::Vehicle` and the track collider; no car-to-car contact.
- Web creates hidden remote rigs for AI and applies frames with
  `DriveViewer.set_remote_state`.
- Core: `sim_core::AiDriver`, `RaceTracker::sample`, and
  `RaceTracker::nearest_sample`.
- Worker: up to three AI cars, 500 Hz stepping, grid slots 1..N, and 64-float
  `ai_snapshot` frames.
- Web: `PhysicsClient.onAiFrame`, `ai_state` worker messages, one AI on
  race-ready single-player tracks, and remote-rig rendering.
- Race Loop grid rotations do not match the layout direction; AI heading is
  derived from the nearest layout tangent. Recorded as TD-0021.
- Manual Race Loop probe: AI stayed within 3.62 m of the centreline (5 m
  half-width), completed a lap in about 40 s, and produced no page errors.
- Gate: cargo fmt, workspace clippy, 213 workspace tests, wasm check; 59 web
  tests after the dev merge; 106 viewer tests passed / 1 skipped; WebGL2
  fallback 2/2; agent sync.
- Status: complete; merged to `dev` as `fc4fdf3` (PR #49).

## AI field resume checkpoint

Branch: `feat/ai-field`, created from `dev` at `fc4fdf3`.

- Plan: `docs/features/ai-field.md`.
- No versioned contract change. Reuse the existing AI snapshot and race data.
- Physics: `AiPace::{Chill, Club, Fast}` scales target speed and lateral
  acceleration; the worker accepts a pace index and applies it to all AI via a
  four-argument `Simulation::new`.
- Web: three AI by default, `?ai=0..3`, `?aiPace=chill|club|fast`, `AI N` HUD,
  and `aiCount` diagnostics/telemetry.
- Measured Race Loop, headless Chromium/SwiftShader: 1 AI p50 0.020 ms / p99
  0.034 ms; 3 AI p50 0.039 ms / p99 0.061 ms; both 499.85 Hz, no dropped ticks.
- Gate: cargo fmt, workspace clippy, 216 workspace tests, wasm check; 59 web
  tests; 110 viewer tests passed / 1 skipped; WebGL2 fallback 2/2; agent sync.
- Status: complete; merged to `dev` as `670312f` (PR #50).

## AI race resume checkpoint

Branch: `feat/ai-race`, created from `dev` at `670312f`.

- ADR-0015 accepts a web-owned race director, grid-start lap semantics, worker
  race placement/start/laps, and AI race state in `ai_state`.
- Plan: `docs/features/ai-race.md`.
- No versioned contract change. `RaceState` and the 64-float snapshot stay the
  same.
- Player starts at `race.grid[0]`; AI at `race.grid[1..]`; both use the layout
  tangent for heading.
- Countdown holds the worker paused; GO resumes and starts the race clock.
- Default race is 3 laps; `?laps=1..5` is for validation.
- Results appear when the player finishes, using current order for unfinished
  AI.
- Core: `RaceTracker::arm_grid_start`, `Vehicle::set_spawn`.
- Worker: race-grid placement, explicit start/lap limit, AI stop-after-limit,
  and AI `RaceState` JSON in `ai_state`.
- Render: `DriveViewer::place_on_race_grid`.
- Web: `RaceDirector`, 3-2-1-GO countdown, `P?/4` and `Lap ?/3`, results
  overlay, restart, and `?laps=1..5`.
- Gate for the AI race changes before the track-option merge: cargo fmt,
  workspace clippy, 228 workspace tests, wasm check; 68 web tests; 114 viewer
  tests passed / 1 skipped; WebGL2 fallback 2/2; agent sync. Combined branch CI
  covers the merged track-option fixture and launcher tests.
- Status: complete; merged to `dev` as `3f83bc8` (PR #52).

## Loop Spline Point Terrain track option

Branch: `feat/loop-spline-point-terrain`, created from `dev` at `670312f`.

- Fixture: `fixtures/tracks/loop-spline-point-terrain/` with the provided
  `track-ed/project@5` project, exported `track.json`/`track.glb`, freshly baked
  `pack.sima`, and README.
- Pack: id `loop-spline-point-terrain`, 941.3 m layout, 471 centreline points,
  8 grid slots, 4 sectors, Race chunk present. Pack size is 27 MB; source
  directory is 45 MB. Follow-up smoothing uses `samplesPerSegment: 32`.
- Launcher: `Loop Spline Point Terrain` option in `TRACK_OPTIONS` and
  `web/index.html`.
- Coverage: a `track-pack` fixture test loads the checked-in pack and checks the
  race shape; Playwright opens the new launcher entry and checks race
  availability.
- Gate: cargo fmt, workspace clippy, 217 workspace tests, wasm check; 59 web
  tests; 111 viewer tests passed / 1 skipped; WebGL2 fallback 2/2; agent sync.
- Status: complete; PR pending.

## AI motion smoothing resume checkpoint

Branch: `feat/ai-motion-smoothing`, created from `dev` at `0754a7c`.

- Plan: `docs/features/ai-motion-smoothing.md`.
- No versioned contract change. `RaceState` and the 64-float snapshot stay the
  same.
- AI: `AiDriver` steering rate default 3.0 units/s; reset clears it.
- Worker: publish AI frames every 8 ticks (62.5 Hz); race state at 20 Hz.
- Web: one `RemoteStateBuffer(40)` per AI slot; race state updates immediately.
- Player: time-based steering rate setting, default 6 units/s, 0 = off; wheel
  input stays direct.
- Gate: cargo fmt, workspace clippy, 233 workspace tests, wasm check; 76 web
  tests; 116 viewer tests passed / 1 skipped; WebGL2 fallback 2/2; agent sync.
- Status: complete; full gate passed; PR pending.

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
| track-ed hosted editor + local Test drive | Released to production |
| Track race foundation | Done; Race chunk, race-loop fixture, launcher entry |
| Race manager MVP | Done; merged to `dev` (PR #47) |
| AI opponents | Done; merged to `dev` (PR #49) |
| AI field | Done; merged to `dev` (PR #50) |
| AI race | Done; merged to `dev` (PR #52) |

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
- Track authoring: hosted track-ed with browser-side `track-pack` compilation,
  local Test drive, and `track.glb` + `track.json` file import. The procedural
  editor remains available as Legacy Track Editor.
- Track race foundation: optional Race chunk, layout as the race route,
  start/finish, 8 grid slots, 4 sectors, track identity, and the Race Loop demo
  fixture/launcher entry.
- Viewers: drive, garage, track viewer, track-ed, track import, legacy track
  editor, style spike, debug viewer, and multiplayer.
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
- `cargo test --workspace` — 194 tests
- `cargo check ... --target wasm32-unknown-unknown` including `track-pack-wasm`
- `npm test` — 57 tests
- `CI=1 npm run test:viewer` — 106 passed, 1 skipped
- `npm --prefix web/track-ed-src test` — 220 passed, 1 skipped
- `npm --prefix web/track-ed-src run build` — tsc + Vite clean
- `bash web/scripts/build-track-ed.sh` — hosted build with notices/licenses
- `python3 web/scripts/check-track-ed-deps.py` — exact pins, lockfile,
  5 runtime notices and licenses ok
- Hosted track-ed smoke — boot, Roadside modes, point edit, junction, export,
  Test drive, autosave restore, New-loop clear, invalid-data warning —
  3 passed
- WebGL2 fallback — 2/2 passed
- Production `9a02580` — landing legacy link count 0, legacy deprecation notice
  present, hosted track-ed and Drive return 200
- track-ed host build and `build-pages.sh`; notices and licenses present under
  `dist/web/track-ed/`
- Production smoke: hosted track-ed Test drive on a current `project@2`
  terrain-follow project opens drive with no page or console errors
- `npm run bench:physics`
- Track race MVP final gate — cargo fmt/clippy/workspace tests/wasm; parent
  `npm test` 50; `CI=1 npm --prefix web run test:viewer` 106 passed, 1 skipped;
  WebGL2 fallback 2/2; track-ed 219 passed, 1 skipped and build; agent sync.
- Race manager MVP final gate — cargo fmt/clippy/workspace tests/wasm; parent
  `npm test` 57; `CI=1 npm --prefix web run test:viewer` 106 passed, 1 skipped;
  WebGL2 fallback 2/2; track-ed 220 passed, 1 skipped and build; agent sync.
- Playwright runs Chromium with WebGPU; WebGL2 fallback checked separately.
- Manual GLB car and AC track checks passed on WebGPU.

## Known issues / carry-forward

- Legacy junction road geometry is still a stub: overlapping full-width ribbons
  cause dirt/grass over tarmac, z-fighting, and holes at angles. It is
  superseded by track-ed (ADR-0007); do not continue ADR-0005/Phase 3d in the
  legacy editor. TD-0008 is superseded.
- The legacy procedural track editor remains; removal is gated per ADR-0007 on
  migration, autosave and CI coverage. Hosted track-ed Test drive and the
  file-import path have manual coverage rather than a PR-gate smoke test.
- Production Pages builds depend on the pinned track-ed `main` submodule commit
  and build the host at deploy time.
- track-ed `project@1` JSON files are import-only. Editing them is unsupported
  and produces stale control handles and crossing geometry; use a current
  `project@2`/`@3` project for authoring.
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

State on `dev` at `aae590d`, merged from `feat/venue-graph-step0b` (branched at
`662527e`) after the `@6` integration (`11622a7`). Dev preview validated the
junction branch, cap, host preservation, and track-pack export:

- PR CI runs `web-smoke` + the editor smoke; the full sharded web suite runs on
  `dev`/`main` pushes and manual dispatch.
- `@6` render model and B-spline evaluator: authored points are off-curve
  handles; start/finish and junction contacts are derived; section overlays
  sample at route density. `cargo test -p sim-render` — 81 passing.
- Step 0b render: `make_junction` adds a Normal role and a short open capped
  branch perpendicular to the host tangent without splitting the host; junction
  contacts are the nearest host-section point and drive both host smoothness and
  attached-section endpoints; all non-route sections bake into the visual and
  collision world; `PointEnd::Cap` loose ends get cap geometry and collision;
  partial/wrapped route legs sample correctly; decoration chains map across
  sections and closed-section seams with continuous element/phase/spacing.
  Unsupported loose-end treatments and non-contiguous/absent chains fail loudly.
- Step 0b web: Make junction in the point editor, branch centerlines and
  loose-end points are visible/selectable/draggable, host point lists stay
  unchanged, and the branch persists. The new Playwright test covers creation,
  dragging, Test Drive export, and reload. A pre-existing wasm reentrancy race
  in the prop-decoration test was fixed by waiting for the rebuild to settle.
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
- Full gate green on this branch: 38 web unit tests, 95 viewer tests passed /
  1 skipped (1 pre-existing garage-camera flake), 153 workspace tests, clippy,
  fmt, wasm. TD-0001 resolved; TD-0005 resolved.
- TD-0006 seam batch merged to `dev` as `ff67366` (PR #27): painting across a
  closed-section seam wraps a single leg (`wrap: true`, entry > exit) with the
  drag hint `painting across the seam`; dragging an endpoint across the seam
  wraps the leg and keeps the far endpoint. Both are covered in
  `web/tests/track-editor.spec.ts`. Custom-lane authoring stays dropped for
  now; junction chain continuation is blocked on multi-leg routes (Phase 4).
- PR #24 merged to `dev` as `79d5c02`: CI excludes the WebGL2 fallback tests
  from both the sharded `web` job and `web-smoke`. `.githooks/pre-push` runs
  both fallback tests locally after `bash scripts/setup-hooks.sh`; the hook
  passed 2/2 and the post-merge `dev` CI run is green.
- Released to `main` as `506abaf` (PR #28); `dev` was synced with `main`
  afterwards. Production now includes the venue graph `@6`, seam decoration
  authoring, and the CI fast path.
- track-ed released to `main` in PR #35 (`914b2f6`) and hotfixed in PR #36
  (`16da199`). `track-pack` and `track-pack-wasm` compile GLB tracks in the
  browser; hosted track-ed and `web/track-import/` store `lapster.track-ed.v1`
  and open `/web/drive/?track=track-ed`. The submodule `e4325a5` tracks
  track-ed `main`, and the Pages build copies the track-ed notices and
  licenses. ADR 0006 is accepted. The racing line now samples the baked asphalt
  surface instead of raw terrain height.
- Road shading fix (PR #29): non-route sections and loose-end caps now use a
  runtime, material-driven `track_blend` instead of mesh-name sniffing, so
  junction branches and extra sections render as tarmac with dirt shoulders.
  Loose-end caps sample the centre asphalt band. Covered by a sim-render test
  and dev-preview validation; no asset-schema change.
- Legacy junction road geometry was re-assessed after angled-junction testing.
  The incremental branch outward/length/trim/cut experiments were reverted.
  It is superseded by track-ed's working junction bake and ADR-0007; do not
  continue ADR-0005/venue Phase 3d in the legacy editor. TD-0008 is superseded.
- Docs: ADR 0003, ADR 0004, ADR 0005, ADR 0006, ADR 0007, ADR 0008,
  editor consolidation report/plan, venue-graph plan, tech debt updated.
- Tag `venue-graph-5` marks the last `@5` checkpoint (`d6e0496`).
- `stash@{0}` holds the superseded `@5` selection UX.
- Local pages: `http://127.0.0.1:8123/web/track-editor/` (legacy),
  `.../web/track-ed/` (hosted), and `.../web/track-import/`.
- Dev preview: `https://joegaffey.com/lapster-dev/web/track-ed/` and
  `.../web/track-import/`.

Resume in this order:

1. Editor consolidation (`docs/features/editor-consolidation-plan.md`):
   remove hidden track-ed authoring UI, split app hotspots, add CI/browser and
   dependency gates, add autosave, decide migration, then remove the legacy
   editor.
2. Legacy Phase 3d junction work and TD-0006 are superseded for the supported
   product path by track-ed/ADR-0007; do not continue them in the legacy
   editor.
3. Multi-layout and pit lane remain deferred; layouts are ADR-0008.
4. Continue the non-editor workstreams: Speed Dreams physics, wheel/FFB
   validation, CP7 HID/audio, CP5/CP6 asset work.
5. Full gate, dev preview validation, then a PR to `dev` for the next batch.

## Next steps

**Race manager MVP (done):** the Race chunk foundation is merged to `dev`
(PR #44). The local lap/sector/progress tracker and HUD are merged to `dev` as
`45fa03d` (PR #47); track-ed PR #7 is merged and the parent pin is `1b22f5a`.
See the resume checkpoint above and `docs/features/race-manager.md`. AI
opponents are the next batch; online contact follows (ADR-0013).

1. **Editor consolidation (next)**: continue
   `docs/features/editor-consolidation-plan.md`. A1, A2/A3, B1, B3, C1–C3 and
   D1 are done. Next: D2 kernel/Worker work (bake is ~8.4 s on a 10-section
   venue), then remove the legacy editor after the ADR-0010 deprecation window
   closes. Do not write more legacy junction geometry. Remove the legacy editor
   only after the ADR-0007 gates pass.
2. **Speed Dreams physics**: parse `sc-deckard-conejo.xml` into
   `CarPhysics`, including mass, CG, torque curve, gears, final drive, brakes,
   aero, springs/dampers, and tire mu. Confirm the reverse-light split visually.
   Move per-car audio and cockpit metadata into the pack before adding more
   Speed Dreams cars.
3. **CP6.5 — wheel/FFB validation**: run the CAMMUS C5 checklist, tune the feel
   mixer, and decide whether the main-thread adapter stays or moves into the
   HID worker.
4. **Vendored `webhid-wheel` validation**: run multi-device profile,
   pedal/button mapping, and worker-compatibility checks; then complete the
   CAMMUS C5 hardware checklist.
5. **CP7 — HID/audio integration**: move the tested FFB adapter into the HID
   worker, add per-car audio profiles or an optional sample layer, expand the
   hardware matrix, and run performance traces. The physics worker, input
   mailbox, snapshot ring, FFB mailbox, and audio worklet spike are in place.
6. **CP5 — real car**: finish the GLB path with decimation, palette/vertex-colour
   tuning, wheel geometry cleanup, and licensed AC `.ini` physics when a
   suitable pack is available. Add a simple steering-wheel/interior placeholder.
   Add real image decoding/KTX2 only where textures are the right choice. WebXR
   remains a later phase.
7. **CP6 — real track**: finish the AC path with full texture decoding
   (PNG/JPEG/DDS/KTX2 where useful), fence/tent collision policy, outer-ring
   collision or streaming, and performance measurement.
8. **Texture pipeline**: add full texture decoding to the GLB and AC importers,
   bake-time texture compression (KTX2/Basis with fallbacks), and texture atlases
   for compatible materials.
9. Replace per-frame JSON telemetry with typed arrays/SAB.
10. Add content-hash cache invalidation.
11. Add Rapier or Jolt only as a collision/query backend if the custom BVH
    becomes limiting.
12. Benchmark a real converted track: draw calls, triangles, texture memory,
    load time, and p99 physics.

## Key files

- `GOALS.md` — vision, goals, success criteria, and non-goals.
- `SPEC.md` — architecture, contracts, and engine policy.
- `PLAN.md` — checkpoint plan.
- `docs/features/editor-consolidation-plan.md` — active editor consolidation
  and track-ed refactoring plan.
- `docs/features/track-race-mvp.md` — track-side race layout/race chunk plan.
- `docs/features/race-manager.md` — lap progress, race state, and HUD plan.
- `docs/editor-consolidation-report.md` — editor analysis and recommendations.
- `docs/adr/0007-promote-track-ed-as-sole-editor.md` — sole-editor decision.
- `docs/adr/0008-defer-layouts-as-route-layer.md` — layout deferral decision.
- `docs/adr/0009-agent-runtime-policy-and-editor-domain.md` — agent limits,
  default agent, permissions and editor domain.
- `docs/adr/0011-race-layout-race-chunk.md` — race layout/race chunk decision.
- `docs/adr/0012-contract-roots-and-agent-permissions.md` — directory-based
  contract ownership and mirror roots.
- `docs/adr/0013-ai-racing-and-online-contact.md` — AI racing and online-contact
  direction.
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
