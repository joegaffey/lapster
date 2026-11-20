# Editor consolidation plan

Status: active
Date: 2026-11-20
Owner: architect

Progress (2026-11-20):

- A1 done in track-ed `d466d94`: hidden Kerbs/Walls/Props modes, panels, drag
  states and update functions removed; Roadside remains the only
  decoration-authoring path. Net reduction is ~1,900 lines including the new
  span helper/tests, with `@1`–`@4` still import-only.
- C1/C2/C3 done: hosted Playwright smoke (track-ed `2ffdb7a` plus
  `web/tests/track-ed-hosted.spec.ts`), `track-ed` and `track-ed-hosted` CI
  jobs, `npm audit --omit=dev`, and exact-pin/lockfile/notice checks
  (`web/scripts/build-track-ed.sh`, `web/scripts/check-track-ed-deps.py`).
- A2/A3 done in track-ed `93c50ad` and `8396ea8`: `attachEditing` is a thin
  composition over `editing/`, and `createTrackView` is a thin facade over
  `scene/` core/surfaces/overlays/picking/lifecycle. Public APIs and behaviour
  are unchanged; pure helpers have tests and the hosted smoke still passes.
- B1 done in track-ed `739472d`: autosave/restore with a versioned
  `localStorage` envelope, invalid-data warning, storage-failure fallback and
  New-loop clearing; hosted reload tests cover restore and clear.
- B3 decided in ADR-0010: one-release read-only legacy window. The legacy nav
  card is removed and the page carries an export/re-author notice; a converter
  is not being built. E is deferred until the window closes (review
  2026-12-20).
- D1 done in track-ed `a4ee32f`: a deterministic 10-section venue (2
  junctions, terrain holes, kerbs, tree/tyre runs) bakes in ~8.4 s and
  re-bakes in ~8.4 s; validate ~1.5 s, export ~1.7 s. This confirms the
  older 9-section baseline order and warrants D2 (kernel/Worker) before any
  Rust/wasm port. Results: `web/track-ed-src/docs/performance-large-venue.md`.
- The final E removal remains deferred until the ADR-0010 window closes.
Related: `docs/editor-consolidation-report.md`, ADR-0007, ADR-0008,
`docs/track-ed-integration.md`, `docs/features/venue-graph-plan.md`

## 1. Goal

Consolidate track authoring on **track-ed** as the single editor, then remove
the legacy procedural editor without losing required capability or user data.

This plan also records the track-ed refactorings identified in the consolidation
report:

- remove the hidden legacy Kerbs/Walls/Props authoring paths;
- split the two oversized app functions (`attachEditing`, `createTrackView`);
- add browser/CI coverage and dependency gates;
- add autosave;
- benchmark large venues before considering a Worker or Rust/wasm kernel port.

## 2. Non-goals

- Do not refactor the legacy editor toward track-ed.
- Do not merge `feat/junction-road-geometry`; it is superseded by track-ed.
- Do not implement layouts now; ADR-0008 defines them as a future route layer.
- Do not port the legacy layout/start-finish/sector model during consolidation.
- Do not change repository topology (standalone submodule versus vendoring)
  without a separate decision.
- Do not port the kernel to Rust/wasm without a profiling result that justifies
  it.

## 3. Decision inputs

- **ADR-0007** — track-ed is the sole supported editor; legacy removal is gated
  on migration, autosave, CI coverage and a final parity check.
- **ADR-0008** — layouts are deferred; if added later they are a route/metadata
  layer over existing sections.
- Consolidation report — architecture, capability, dependency, LOC/complexity
  and performance findings.

## 4. Workstream A — track-ed app refactors

### A1. Remove hidden legacy authoring UI — done

**Target:** `web/track-ed-src/src/app/main.ts` and its inline template.

- Delete hidden Kerbs/Walls/Props mode buttons, panels, update functions, drag
  states and branches left over from `project@3`/`@4`.
- Keep the current Roadside mode and fixed kerb-edge splines.
- Remove dead status/selection state tied only to the hidden modes.

**Acceptance**

- No `mode-kerbs`, `mode-walls`, `mode-props` paths remain.
- `project@5` remains the only authoring path.
- `npm test`, `npx tsc --noEmit` and `npm run build` pass.
- A smoke check covers Roadside kerb/tree painting after the removal.
- Net reduction is roughly 1,000 lines or the remaining equivalent is
  documented.

### A2. Split `attachEditing` — done

**Target:** the ~1,500-line `attachEditing` function.

Split by mode and interaction concern:

- track/point drag, insert, delete and selection;
- terrain point editing;
- roadside/kerb/wall/prop painting and handle resize;
- spawn and camera/overlay handling;
- keyboard/delete/frame shortcuts.

Keep pointer-state transitions in small helpers with explicit input/output
types. Do not change behaviour in the same batch.

**Acceptance**

- Each handler module is independently reviewable and testable.
- At least the pure state/transition helpers have unit tests.
- Browser smoke coverage covers point edit, junction creation and roadside
  painting.
- No increase in bundle size beyond the module overhead.

### A3. Split `createTrackView` — done

**Target:** the ~1,360-line `createTrackView` function.

Split into:

- renderer/scene/camera setup;
- baked-track meshes and materials;
- overlay construction (centerlines, boundary/T-junction, collision, kerbs,
  props, racing line, terrain);
- picking/hit-testing;
- lifecycle/disposal and resize.

**Acceptance**

- `scene.ts` exposes a smaller `TrackView` façade over the split modules.
- No behaviour change in existing viewers.
- Unit tests cover geometry/overlay builders where possible.
- Browser smoke coverage covers boot, overlays and selection.

### A4. Decompose remaining hotspot functions

Evaluate after A2/A3:

- `splitClosedSection` (~347 lines);
- `validateSemantics` (~297 lines);
- `updateRoadsideUI` (~161 lines).

Split only where it improves testability or review size; avoid refactoring the
kernel for style.

## 5. Workstream B — parity before removal

### B1. Autosave/browser persistence

Add project autosave and restore-on-boot to track-ed while keeping explicit
Save JSON/Open JSON. Use the existing browser storage conventions and handle
storage failure without losing the in-memory project.

**Acceptance:** edit → reload restores the project; invalid stored data is
ignored with a visible warning; reset/new clears storage.

### B2. Preview decision

Record whether track-ed needs:

- car marker and camera presets;
- a built-in mountain backdrop;
- tree detail/LOD/card modes.

Test drive remains the authoritative in-engine check. Port only features that
are explicitly required.

### B3. Migration decision — done

Decision: ADR-0010 chooses a one-release read-only legacy window. Legacy
projects are local-only and already exportable as `lapster/track-project@6`
JSON; the legacy card is removed from navigation, the page carries a
deprecation/export notice, and a converter is deferred until evidence of a
project corpus justifies it.

Choose one:

- converter from `lapster/track-project@6` to track-ed's project format;
- one-release read-only legacy window;
- documented unsupported archive.

Do not remove the legacy page before this is decided.

## 6. Workstream C — CI and dependency gates

### C1. Hosted track-ed browser coverage — done

Add a Lapster Playwright smoke test that:

1. opens hosted track-ed;
2. edits a point;
3. creates a junction;
4. exports;
5. runs Test drive;
6. asserts no page errors and a loaded Drive asset.

### C2. Run track-ed tests on pin changes — done

- Add a CI job for `web/track-ed-src` `npm ci`, `npm test`, `npx tsc --noEmit`
  and `npm run build` when the submodule pointer changes.
- Or vendor track-ed into the monorepo gate if repository topology changes.

### C3. Dependency/licence gate — done

- Add `npm audit` for track-ed.
- Verify the shipped third-party notices remain complete.
- Keep exact pins in `package.json` and commit the lockfile.
- Treat geometry-library replacements as contract-sensitive work requiring the
  fixture/conformance gate.

Implemented as `npm audit --omit=dev` in the `track-ed` CI job plus
`web/scripts/check-track-ed-deps.py`, which checks exact pins, lockfile sync,
runtime notices and the three license files. The Pages build also runs the
check before compiling the hosted editor.

## 7. Workstream D — performance

### D1. Benchmark representative large venues — done

Measured with the opt-in benchmark in
`web/track-ed-src/src/kernel/large-venue-bench.test.ts` and recorded in
`web/track-ed-src/docs/performance-large-venue.md`: 10 sections, 2 junctions,
70 control points, 21 terrain points, kerbs plus tree/tyre runs. Median of
three runs: bake 8.38 s, incremental re-bake 8.37 s, validate 1.51 s, export
1.71 s. This warrants D2 (optimise the TS kernel, then use a Web Worker if
needed); do not port to Rust/wasm on this evidence.

Measure:

- full bake;
- incremental edit/bake;
- validate;
- export;
- preview FPS/draw calls;
- memory.

Use at least one 8–16-section venue with terrain holes, junctions and roadside
runs. Record results against the existing performance baseline.

### D2. Escalation order

1. Optimise the TS kernel.
2. Move bake to a Web Worker.
3. Port only the kernel to Rust/wasm behind the same `BakedTrack`/export
   contract.
4. Keep native/offline compile as a slow path for enormous tracks.

Do not keep or refactor the legacy editor as the performance answer.

## 8. Workstream E — legacy removal (deferred by ADR-0010)

ADR-0010 keeps `/web/track-editor/` for one read-only window; steps 1–2 are
done (decision, navigation removal and deprecation notice). Steps 3–6 run after
the window closes (review 2026-12-20) and a final parity check.

1. Decide migration (B3).
2. Update navigation: keep track-ed as **Track Editor**; remove the legacy card.
3. After the deprecation window, delete:
   - `web/track-editor/`;
   - legacy editor-only Playwright tests;
   - editor APIs in `crates/sim-render/src/lib.rs`;
   - `crates/sim-render/src/track_editor.rs`.
4. Keep shared renderer code and viewer utilities:
   `track-camera`, HUD, loading, FPS, versioned wasm, pack cache,
   `track-pack`/`track-pack-wasm`.
5. Update `PROGRESS.md`, `AGENTS.md`, the domain map, tech debt and ADR index.
6. Run the full gate and verify Test drive and Drive on the dev preview.

## 9. Order

1. A1 hidden UI removal (smallest, reduces surface immediately).
2. C1 hosted browser smoke + C2 track-ed CI + C3 dependency gate.
3. A2/A3 app splits, with browser coverage.
4. B1 autosave.
5. B3 migration decision and implementation.
6. B2 preview/LOD decision.
7. D1 large-venue benchmark; D2 only if the budget is exceeded.
8. Legacy navigation removal, deprecation window, then E removal.

## 10. Verification

For track-ed-only changes:

```bash
cd web/track-ed-src
npm ci
npm test
npx tsc --noEmit
npm run build
```

`npx tsc` must run with `web/track-ed-src` as the working directory;
`npx --prefix web/track-ed-src tsc` does not change cwd. Use
`npm --prefix web/track-ed-src run build` when in doubt; it runs the same
type-check.

For Lapster-side integration changes:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p sim-types -p sim-asset -p sim-core -p sim-render -p sim-worker \
  -p track-pack-wasm --target wasm32-unknown-unknown
npm --prefix web test
npm --prefix web run test:viewer
```

Docs-only changes require `git diff --check` and the agent sync check.

## 11. Risks

| Risk | Mitigation |
|---|---|
| App refactors change pointer behaviour | Split incrementally; add browser smoke before/during; keep behaviour-only batches separate. |
| Hidden UI removal drops a needed path | Confirm `project@5`/Roadside covers it; keep fixtures and smoke coverage. |
| Submodule bump breaks hosted integration | Run track-ed tests and hosted browser smoke in Lapster CI. |
| Dependency/licence regression | Gate `npm audit`, notices and lockfile; keep exact pins. |
| Large-venue bake stalls | Benchmark; Worker first; kernel port only with evidence. |
| Legacy removal breaks saved projects | Migration decision and deprecation window first. |

## 12. Open decisions

- ~~Legacy project migration option.~~ Decided in ADR-0010: one-release
  read-only legacy window with JSON export/re-author guidance; no converter
  now.
- ~~Removal timing.~~ After the ADR-0010 window (review 2026-12-20) and a final
  parity check.
- Repository topology: standalone submodule versus vendoring.
- Second-consumer strategy.
- Car camera/exact preview requirement.
- Tree detail/LOD requirement.
- D2 implementation choice: optimise terrain-hole triangulation/polygon
  clipping first, then Web Worker if the large-venue bake remains over budget.
