# Project Progress

Last updated: 2026-11-21
Current phase: **Editor consolidation — track-ed is the sole editor; legacy
removal is gated on migration, parity and CI coverage**

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
- Garage: `http://127.0.0.1:8123/web/garage/`
- Track: `http://127.0.0.1:8123/web/track-viewer/`

Expected browser: Chromium/Chrome. WebGPU is used when available; WebGL2 is the
fallback.

Deployment:

- Live site: https://joegaffey.com/lapster/
- Dev preview: https://joegaffey.com/lapster-dev/ via
  `gh workflow run pages.yml -R joegaffey/lapster-dev -f ref=<branch-or-tag>`

## Current focus

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
- `cargo test --workspace` — 180 tests
- `cargo check ... --target wasm32-unknown-unknown` including `track-pack-wasm`
- `npm test` — 50 tests
- `CI=1 npm run test:viewer` — 106 passed, 1 skipped
- `npm --prefix web/track-ed-src test` — 219 passed, 1 skipped
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

**Track race MVP (foundation complete):** the optional Race chunk,
layout-as-race-route contract, section-owned start/finish/grid, implicit-layout
export, race-loop demo fixture, and launcher entry are landed on
`feat/track-race-mvp`; see the resume checkpoint above and
`docs/features/track-race-mvp.md` §3.1. AI and online contact are later
batches.

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
- `docs/editor-consolidation-report.md` — editor analysis and recommendations.
- `docs/adr/0007-promote-track-ed-as-sole-editor.md` — sole-editor decision.
- `docs/adr/0008-defer-layouts-as-route-layer.md` — layout deferral decision.
- `docs/adr/0009-agent-runtime-policy-and-editor-domain.md` — agent limits,
  default agent, permissions and editor domain.
- `docs/adr/0011-race-layout-race-chunk.md` — race layout/race chunk decision.
- `docs/adr/0012-contract-roots-and-agent-permissions.md` — directory-based
  contract ownership and mirror roots.
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
