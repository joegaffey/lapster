# Editor consolidation report — legacy procedural editor and track-ed

Status: research complete. No code changes were made for this report.
Updated: 2026-11-20 with dependency, LOC/complexity, performance/Rust-wasm and
track-ed refactoring findings.
Date: 2026-11-20
Owner: architect
Related: ADR 0003, ADR 0004, ADR 0005, ADR 0006, ADR 0007, ADR 0008,
`docs/track-ed-integration.md`, `web/track-ed-src`,
`crates/sim-render/src/track_editor.rs`

## 1. Executive summary

Lapster currently ships two track editors:

- the **legacy procedural editor** (`web/track-editor/` plus the Rust bake in
  `crates/sim-render/src/track_editor.rs`);
- **track-ed** (`web/track-ed-src`, a pinned submodule of `joegaffey/track-ed`).

The legacy editor is mature in authoring UX and in-engine preview, but its
junction geometry on `dev` is still the independent-ribbon bake recorded as
TD-0008. A serious clipped/stitched attempt exists on the unmerged
`feat/junction-road-geometry` branch; it works for straight two-point branches
but explicitly rejects curved branches and was never landed.

track-ed is the only stack with working junctions and general venue topology.
Its success is primarily the result of:

1. an explicit **planar-arrangement** geometry model (union footprints, clip
   terrain, conform shared boundaries, triangulate constrained regions, weld and
   validate);
2. a **consumer-neutral kernel** separated from the three.js app and the Lapster
   export profile;
3. a **research/spec/decision/fixture process** that made junction correctness
   executable and testable from the start.

The language difference (TypeScript versus Rust/wasm) is a contributing factor
in iteration speed, but it is not the root cause. The legacy editor could have
solved junctions in Rust; it did not because the general arrangement model was
retrofitted too late into a coupled, ribbon-based production bake.

Recommendation: **consolidate on track-ed as the single editor**, remove the
legacy editor after a migration/deprecation window, do not merge the stale
junction branch, add missing hosted-E2E and cross-repo CI coverage, and clean up
track-ed's own redundant legacy authoring UI. Repository topology (standalone
submodule versus vendoring into Lapster) remains a separate strategic decision.

## 2. Scope and method

Reviewed:

- `web/track-editor/main.ts` (4,249 lines), `index.html` (641 lines);
- `crates/sim-render/src/track_editor.rs` (6,913 lines on `dev`, 61 tests);
- `web/tests/track-editor.spec.ts` (29 browser tests) and `editor-drive.spec.ts`;
- track-ed's `src/` (18,417 non-test TypeScript lines, 3,467 test lines, 168
  passing tests), `SPEC.md`, `docs/decisions.md`, contracts, research report,
  conformance/performance reports;
- ADRs 0003, 0004, 0005, 0006, `docs/features/venue-graph-plan.md`,
  `docs/track-ed-integration.md`;
- the unmerged `feat/junction-road-geometry` branch (8,637-line
  `track_editor.rs`, 72 tests);
- Pages/build/CI integration and the `track-pack`/`track-pack-wasm` path.

Verification performed for this report:

- `npm test` in `web/track-ed-src`: 24 files, **168 tests passed**;
- `npx tsc --noEmit` in `web/track-ed-src`: clean;
- `cargo test -p sim-render --lib track_editor`: **61 tests passed**;
- `npm audit` in both web packages: 0 vulnerabilities at the time of analysis;
- dependency inventories, transitive package/crate counts and LOC/function
  metrics were collected from the checked-in manifests and source;
- both worktrees clean.

Not performed: full repo gate, live browser checks, or production-preview
validation. This is a research report, not a release batch.

## 3. Current status

### 3.1 Legacy editor

**What it is**

- Vanilla TypeScript UI plus an HTML HUD and a wasm renderer/bake
  (`TrackViewer.create_editor`).
- Rust owns the project model, validation, spline sampling, terrain, kerbs,
  props, racing line, mountains, pack writing and export.
- The TypeScript side communicates with Rust through JSON string getters and
  async mutate/rebuild calls.

**Strengths**

- Mature authoring interactions: points, start/finish, kerbs, props, guide-rail
  painting, camera views, draggable car marker.
- Accurate in-engine preview and one-click Test drive using the same wgpu
  renderer as Drive.
- 61 Rust unit tests including determinism and export checks.
- Broad Playwright coverage of real UI flows.
- Model supports layouts, route legs, start/finish, sectors, cameras and
  decorations, even where the UI does not expose all of them.

**Current gaps**

- Junction road geometry is absent on `dev`. Sections are baked as independent
  full-width ribbons, so angled joins overlap asphalt/dirt/grass, z-fight and
  leave holes (TD-0008).
- `feat/junction-road-geometry` has a clipped/stitched implementation with
  host-shoulder windows, ear-clipped pads, matching collision and 30/45/90°
  tests, but:
  - it was never merged;
  - it is based on an older `dev`;
  - it rejects curved junction branches with a typed error;
  - kerb/prop/decor junction clearance remains deferred.
- No front-end `tsconfig`; UI TypeScript is bundled by esbuild without a
  type-check gate.
- PR CI runs only a small smoke subset of the editor browser suite; the full
  suite is on demand.
- No undo/redo.
- The wasm editor API has an async mutate/read reentrancy footgun (TD-0007).

### 3.2 track-ed

**What it is**

- Standalone, browser-first TypeScript editor.
- Clean layering: `src/kernel/` (consumer-neutral geometry), `src/model/`
  (`BakedTrack`), `src/export/lapster/` (profile), `src/app/` (three.js UI).
- Hosted at `/web/track-ed/` with `VITE_LAPSTER_HOST=1`; **Test drive in
  Lapster** compiles exported GLB+JSON through `track-pack-wasm`, stores the
  pack in IndexedDB, and opens Drive.

**Strengths**

- Working junctions and venue sections: closed loops, dead-ended open sections,
  open/closed and multi-way junctions, loose-end joins/merges, splitting,
  curved spline branches.
- Explicit planar arrangement: footprint union, terrain difference, boundary
  conforming, constrained triangulation, welding and topology validation.
- 19 checked-in fixtures pass parse/bake/validate/export and Lapster
  conformance; the fixture matrix asserts zero interior boundary edges, zero
  non-manifold edges, zero T-junctions and zero overlaps.
- Deterministic GLB writer and export that refuses invalid baked tracks.
- Strict TypeScript (`noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`,
  `noFallthroughCasesInSwitch`) and `tsc --noEmit` in CI.
- Staged schema support (`track-ed/project@1`–`@5`) with strict unknown-field
  rejection.
- Already integrated with Lapster's pack compiler and Drive.

**Gaps and risks**

- App layer is still monolithic: `main.ts` 5,896 lines, `section-edit.ts` 1,963,
  `scene.ts` 1,620. `attachEditing` (~1,500 lines) and `createTrackView`
  (~1,360 lines) concentrate pointer and scene complexity; see section 6.4.
  Pointer/drag interactions are mostly covered only by pure helper tests.
- No Playwright/E2E suite inside track-ed.
- No hosted track-ed browser test in Lapster CI; only `track-import.spec.ts`
  covers the compiler/drive path.
- Hidden legacy Kerbs/Walls/Props panels, modes, drag states and update
  functions remain wired alongside the newer Roadside mode. This is roughly
  1,000 lines of redundant UI/authoring code.
- No autosave/browser project persistence (legacy had localStorage save/load).
- No car-camera or draggable car-marker preview.
- No tree-detail/card modes comparable to the legacy high/low/cards preview.
- Large-venue bake cost grows quickly with section and terrain-hole count; the
  documented synthetic cases reach seconds to tens of seconds.
- Production build depends on a submodule pinned to track-ed `main`; CI does
  not run track-ed's tests when the pin changes.

## 4. Why junctions worked in track-ed

### 4.1 What the legacy editor attempted

The legacy model was not naive. ADR 0003 defines a section-point graph with
global points, open/closed sections, derived junction contacts and route legs;
ADR 0004 adds B-spline handles and derived on-road anchors. ADR 0005 explicitly
identifies the real geometry problem and proposes clipping and stitching.

The unmerged `feat/junction-road-geometry` branch implemented much of ADR 0005:

- derive a 2–3-road junction mouth;
- clip host and branch ribbons;
- remove the host shoulder window;
- build one ear-clipped stitched junction patch;
- bake collision from the same surfaces;
- test 30/45/90° straight branches.

It did not generalise to curved branches. The branch itself says:

> `"junction branch '{}' is curved; only straight branches are supported in this pass"`

and still defers kerb/barrier/prop/decor clearance.

### 4.2 What track-ed did differently

track-ed starts from the research recommendation to build one **planar
arrangement** before triangulating:

1. sample every section centreline;
2. build closed asphalt/shoulder rings;
3. union the road footprints (with a Clipper2 fallback);
4. subtract asphalt from shoulder, and shoulder from the terrain rectangle;
5. conform all shared boundaries by inserting anchor points;
6. triangulate each region with the shared boundary vertices;
7. weld and validate boundary edges, non-manifold edges, T-junctions and
   overlaps;
8. refuse to export if validation fails.

That approach handles arbitrary branch angles and curved splines because the
junction is a consequence of the union/difference/conform step, not a
special-cased mouth patch. It also supports multiple sections and multi-way
junctions with the same pipeline.

### 4.3 Verdict: approach and architecture, not language

| Factor | Contribution | Evidence |
|---|---|---|
| Geometry model | **Decisive** | track-ed's global arrangement handles curved/multi-way; legacy branch's local mouth/patch supports straight two-point branches only. |
| Kernel/contract boundary | **Decisive** | track-ed's pure `project -> kernel -> BakedTrack -> export` path is unit-tested in Node in milliseconds; legacy geometry is coupled to wasm/renderer and the editor. |
| Research/process | **Strong** | track-ed had a research brief/report, spec, decision record, fixture matrix, conformance and performance gates; legacy junction work was a later Phase 3d after ribbon assumptions spread. |
| Robust geometry libraries | Contributing | `polygon-clipping`/`clipper2-ts`/`poly2tri` were important, but equivalent robust options exist in Rust; the choice of arrangement/conforming was the key, not the language. |
| Language/stack | Contributing, not decisive | TypeScript iteration in Node is faster than Rust→wasm→browser, but the legacy editor could have implemented the same arrangement in Rust. |
| Product scope pressure | Contributing | Legacy editor had accumulated layouts, decorations, cameras, pack export and renderer coupling; track-ed was a focused spike with explicit promotion/kill criteria. |

The defensible conclusion is: **track-ed succeeded because of a better
architecture and a better development approach; the technology choice helped
iteration speed, but swapping Rust for TypeScript alone would not have fixed the
legacy editor.**

## 5. Capability comparison

| Capability | Legacy editor | track-ed | Consolidation implication |
|---|---|---|---|
| Single closed loop + branches | Yes | Yes | Keep track-ed. |
| General multi-section venues | Broken at junctions on `dev` | Yes | Keep track-ed. |
| Curved spline junctions | No on `dev`; straight-only on unmerged branch | Yes | Do not merge the stale branch. |
| Multi-way junctions | Partial/model support | Yes | Keep track-ed. |
| Loose ends, joins, merges, split | Yes | Yes | Keep track-ed; port any UX details if missed. |
| Splines | B-spline, 120 fixed samples | Catmull-Rom, configurable samples | No blocker. |
| Terrain | Sine heightfield + grid; built-in mountains | TIN points; `conform`/`follow`; Lapster supplies backdrop | track-ed is stronger for authoring; confirm backdrop parity in preview. |
| Kerbs/walls/props | Yes | Yes, plus free roadside splines | Keep track-ed; remove its hidden old UI. |
| Racing line | Yes | Yes, per-section visual-only | Keep track-ed. |
| Undo/redo | No | Yes | track-ed is stronger. |
| Autosave/project persistence | Yes (localStorage) | No | **Agreed follow-up:** add to track-ed before legacy removal. |
| JSON save/open/import | Yes | Yes; supports @1–@5 | Add legacy-project migration decision. |
| Validation inspector | Basic stats/meshes | Topology, boundary, T-junction, overlap, collision, triangle inspector | track-ed is stronger. |
| Test drive | In-editor, wasm pack | Hosted Test drive, track-pack wasm | track-ed path is already proven; add CI coverage. |
| Car camera / marker | Yes | No | Open product question; note in this report. |
| Tree detail high/low/cards | Yes | Layered geometry only | Open product question; not a blocker. |
| Layouts/routes/start-finish/sectors | Model supports; UI limited | Not in current schema | Treat as deprecated per prioritisation answer; confirm in ADR. |
| Exact in-game preview | Yes (wgpu) | Test drive only; preview is three.js | Note as open requirement; Test drive is authoritative for now. |
| Front-end type-check in CI | No | Yes | track-ed is stronger. |
| Browser E2E | Yes, small CI subset | No | Add focused track-ed/hosted smoke suite. |
| Build isolation | Editor code in main wasm | Separate app bundle and lazy pack wasm | Removing legacy reduces main wasm/API surface. |

### 5.1 Layouts

Layouts are the one legacy model feature with no direct track-ed equivalent, but
the legacy support is largely dormant:

- the legacy model has multiple layouts, route legs, per-layout start/finish,
  spawns, sectors and cameras;
- the legacy UI only edits the active layout and exposes set-start/finish;
- there is no add/switch/delete layout UI;
- `editor_track_pack()` exports one pack from one active layout;
- Drive has no layout selector, so multiple layouts are not consumable
  end-to-end.

track-ed has no layout layer: one spawn, all sections baked, track length as the
sum of section lengths, per-section racing lines, and no start/finish, sectors
or route legs. `cameras` exist in the project schema and manifest but have no
authoring UI.

ADR-0008 records the decision: layouts are deprecated for the current scope and,
if they return, must be a route/metadata layer over track-ed's existing
sections, not a second geometry model. The practical current loss is
start/finish authoring; multi-circuit selection was not a working workflow in
the legacy editor either.

## 6. Code quality, maintainability and evolvability

### 6.1 Legacy editor

**Strengths**

- Rust type safety and deterministic geometry tests.
- Real UI flows are exercised by Playwright.
- In-engine preview reduces the risk of “editor looks right, game looks wrong”.
- Rich model for future layouts/sectors, even if not surfaced.

**Weaknesses**

- Two languages and a JSON string boundary; TypeScript interfaces duplicate Rust
  shapes and are not type-checked against them.
- No front-end type-check gate.
- One 6.9k-line Rust module mixes geometry, materials, textures, terrain,
  props, validation, serialisation and export.
- `EditorTrack` is consumer-specific (`TrackVisual`, `TrackCollision`,
  `AssetManifest`) rather than a neutral baked model, so geometry changes touch
  renderer, collision and export concerns together.
- Async mutate/read reentrancy hazard (TD-0007).
- No undo/redo.
- Large unmerged junction branch is now stale and conflicts with the newer
  track-ed work.
- Duplicates terrain/tree/texture/racing-line logic that track-ed has ported.

### 6.2 track-ed

**Strengths**

- Clean domain boundaries; no Lapster names in the kernel.
- Strict runtime schema parsing and rejection of unknown fields.
- Dense kernel tests and executable fixture acceptance.
- Validation gate before export.
- Deterministic output.
- Clear path for a second export profile or a future kernel replacement.

**Weaknesses**

- App layer is monolithic and manual-DOM; UI state and drag logic are large.
- Kernel tests are strong, but browser interaction coverage is weak.
- Hidden legacy authoring UI is a second code path.
- Schema support spans @1–@5 with import-only exceptions; parser complexity will
  keep growing unless old versions are retired.
- No autosave.
- Bake performance needs attention as venues grow.
- Host/build integration has no cross-repo automated test.

### 6.3 Evolvability and potential

track-ed is the stronger base for future work because:

- geometry can evolve without touching the app or the Lapster export profile;
- the `BakedTrack` contract supports new exporters;
- the kernel can move to a Web Worker or a Rust/wasm implementation later
  without changing the authoring UI;
- fixtures provide a regression harness for new junction/terrain shapes;
- deterministic export is already a product-quality property.

Its main evolvability constraints are UI modularity and the lack of E2E
coverage, not the geometry core. The legacy editor's evolvability is constrained
by the coupled bake and the unresolved junction geometry; fixing those would
require the same architectural work track-ed already completed.

### 6.4 Lines of code and complexity

The legacy editor has fewer editor-specific lines, but its complexity is more
concentrated and more coupled. track-ed has more lines because it owns more of
the stack, but its kernel is better factored.

| Area | Legacy editor | track-ed |
|---|---:|---:|
| Main UI TypeScript | 4,249 (`web/track-editor/main.ts`) | 9,909 app TS (`main.ts` 5,896, `scene.ts` 1,620, `section-edit.ts` 1,963, etc.) |
| Rust geometry/model | 4,604 implementation lines in `track_editor.rs` | — |
| Markup/CSS | 641 (`index.html`, inline CSS) | 712 (`style.css`) |
| Kernel/geometry | included in the Rust module | 6,973 |
| Export/GLB/manifest | reused from `sim-asset`/`sim-types` | 1,470 |
| Schema/model | serde plus shared Rust types | 1,985 parser plus 65 model |
| **Editor-specific implementation** | **~9,500** | **~19,100** |
| Non-test files | 2 core editor files plus 1 Rust module | 24 TypeScript files plus CSS |
| Functions | 125 TS plus 138 Rust implementation functions | ~394 TS functions |
| Tests | 61 Rust tests (2,309 lines) plus 29 Playwright tests (2,547 lines) | 168 Vitest tests (3,467 lines); no E2E |

track-ed is roughly **2× the implementation LOC** of the legacy editor. That is
not a like-for-like comparison:

- the legacy editor reuses `sim-render`, `sim-asset`, `sim-types` and the
  renderer; its editor-specific LOC excludes a large amount of platform code;
- track-ed owns its kernel, schema parser, validation, GLB writer and export
  profile, which the legacy editor got from Rust crates and shared asset types;
- track-ed implements working general junctions, multi-section venues,
  multi-way junctions, TIN terrain, roadside splines and export refusal;
- track-ed still contains roughly 1,000 lines of hidden legacy Kerbs/Walls/Props
  UI and drag handling.

The largest implementation functions show where complexity actually lives:

| Legacy | Lines | track-ed | Lines |
|---|---:|---|---:|
| `drawControlPoints` | ~335 | `attachEditing` | ~1,500 |
| `normalize_in_place` | ~326 | `createTrackView` | ~1,360 |
| `build_editor_track` | ~291 | `splitClosedSection` | ~347 |
| `drawDecorationOverlay` | ~170 | `validateSemantics` | ~297 |
| `computeResizeUpdate` | ~160 | `bakeAllSections` | ~142 |

Rough conditional-token counts are ~1,048 in the legacy implementation versus
~2,773 in non-test track-ed TypeScript. The growth is roughly proportional to
LOC and reflects more features, schema variants and validation. It is not a
formal cyclomatic score.

Conclusion: the legacy editor is smaller because it delegates more and does
less. track-ed's geometry kernel is the cleanest part, with most functions under
100 lines; its app layer is the hotspot. `attachEditing` and `createTrackView`
are the two refactoring targets.

### 6.5 Dependencies

| Dimension | Legacy editor | track-ed |
|---|---|---|
| Extra npm runtime deps | None | 5 direct: `clipper2-ts`, `earcut`, `poly2tri`, `polygon-clipping`, `three` |
| Extra npm dev deps | None beyond shared web tooling | 4 direct: `@types/three`, `typescript`, `vite`, `vitest` |
| Transitive npm packages | ~6 in `web/` | ~52 |
| Extra Rust crates | None beyond the existing `sim-render` stack | None standalone; `track-pack-wasm` for hosted Test drive |
| Rust footprint | `sim-render`: ~13 direct deps, ~225-crate tree | `track-pack-wasm`: ~3 direct deps, ~133-crate tree |
| Build tools | Rust, `wasm32-unknown-unknown`, `wasm-bindgen` 0.2.129, `wasm-opt`, esbuild | Node/Vite/TypeScript/Vitest plus Rust for the pack compiler |
| Bundle cost | ~135 KB editor JS; ~4.1 MB shared wasm; 640 KB tree-card atlas embedded in wasm | ~920 KB JS plus ~8.5 KB CSS; lazy pack wasm |
| Audit state | `npm audit`: 0 vulnerabilities (shared `web/`) | `npm audit --omit=dev`: 0 vulnerabilities |

Important asymmetries:

- The legacy editor's marginal dependency cost is low because it reuses the
  renderer stack. Removing it does not remove `wgpu`, `glam`, `wasm-bindgen`,
  `serde` or `web-sys`.
- The legacy editor does add editor code, compile time and the embedded tree-card
  atlas to the main wasm.
- track-ed adds an isolated npm/toolchain surface. Its licenses are compatible:
  MIT, ISC, BSD-3-Clause, BSL-1.0 and MIT for three. Notices are shipped.
- `clipper2-ts` (BSL-1.0, pre-release-style version) and `polygon-clipping`
  (older) are the geometry dependencies with the least maintenance certainty;
  `earcut`, `poly2tri` and three are widely used or small and stable.
- Hosted track-ed still uses Rust/wasm through `track-pack-wasm`; it does not
  eliminate the Rust toolchain from the production path.

### 6.6 Performance and the Rust/wasm question

The current evidence does not support a claim that the legacy editor is
significantly faster for equivalent large tracks:

- track-ed fixtures bake in 0.1–524 ms, export in 1.3–191 ms, and preview at
  ~60 FPS;
- synthetic large venues reach about 2.0 s, 9.7 s and 31.1 s for 4, 9 and 16
  sections respectively, dominated by polygon clipping and triangulating terrain
  with many holes;
- there is no fair legacy-versus-track-ed large-track benchmark;
- the legacy `dev` bake does less work: fixed 120 samples, independent ribbons,
  a static terrain grid and no general junction arrangement;
- the unmerged legacy junction branch added 2,658 lines to `track_editor.rs` and
  still rejected curved branches.

Rust/wasm is likely faster for tight CPU-bound geometry and may be 2–5× faster
than equivalent JavaScript in places, but that is not proven for this workload.
The old editor also crosses a wasm/JSON boundary and rebuilds the scene on edits,
so its end-to-end editing loop may be slower despite a faster kernel.

If large-track bake becomes a real UX problem, use this order:

1. measure a representative 8–16-section venue: full bake, incremental edit,
   validate, export, preview FPS and memory;
2. optimise the TS kernel (fewer boolean passes, cached unchanged regions,
   better terrain-hole triangulation);
3. move bake to a Web Worker to keep the UI responsive;
4. port only the kernel to Rust/wasm behind the same `BakedTrack` and export
   contract, if profiling proves CPU-bound geometry is the limit.

Do not refactor the old editor to get Rust performance. Porting track-ed's
kernel is the lower-risk path if that step is ever needed.

## 7. Technical debt findings

| Finding | Severity | Area | Recommendation |
|---|---|---|---|
| Legacy junction geometry is broken on `dev` (TD-0008). | P1 | render/web | Superseded by track-ed; do not fix further in legacy. |
| `feat/junction-road-geometry` is a stale, straight-only solution. | P2 | render | Archive/close; record as historical; do not merge. |
| Legacy editor TypeScript is not type-checked in CI. | P2 | web | Moot after removal; do not invest. |
| Legacy wasm editor API reentrancy (TD-0007). | P3 | web | Moot after removal. |
| track-ed hidden Kerbs/Walls/Props UI and drag paths remain wired alongside Roadside. | P2 | track-ed app | Remove after `project@5` is the only authoring format. |
| track-ed has no browser/E2E suite. | P1 | track-ed app/CI | Add focused Playwright smoke tests, including hosted Test drive. |
| Lapster CI does not run track-ed tests on submodule bumps. | P1 | CI | Add a track-ed job or merge the repo into the monorepo gate. |
| No legacy project migration path. | P2 | data/migration | Decide converter, read-only legacy window, or unsupported archive. |
| track-ed app monolith (`main.ts` ~5.9k lines). | P2 | track-ed app | Split by concern; prioritise pointer/drag modules. |
| track-ed app hotspots: `attachEditing` ~1.5k lines and `createTrackView` ~1.36k lines. | P2 | track-ed app | Split by mode/handler and into renderer/overlays/picking/lifecycle. |
| No dependency/license audit gate in CI for the submodule. | P3 | CI | Add `npm audit` and license/notice checks to the track-ed gate. |
| Large-venue bake cost. | P3 | track-ed kernel | Measure as fixtures grow; move to Worker if UX-visible. |
| Production depends on a submodule pinned to `main`. | P3 | release | Pin to a release tag once track-ed releases exist. |
| No autosave in track-ed. | P2 | track-ed app | Agreed follow-up: add before legacy removal. |
| Preview parity (three.js preview vs in-engine) and tree LOD. | P3 | product | Record as open questions; Test drive is authoritative for now. |

## 8. Prioritisation decisions recorded

The following answers were provided for this report:

1. **Multi-layout, routes, start/finish and sectors** — answered “ok”. This
   report treats that as approval to **deprecate** them from the consolidated
   editor unless a future product need is raised. This is now recorded in
   ADR-0008.
2. **Legacy project migration** — deferred until this analysis was complete.
   The analysis is now complete; the migration decision remains **open**. The
   options and recommendation are in section 9.
3. **Preview expectations** — to be **noted in the report**. Recorded here as an
   open product question: track-ed's three.js preview is an approximation; the
   hosted **Test drive** is the authoritative in-engine check. If car-camera or
   exact-renderer sightline validation is required while editing, add a
   lightweight car marker/camera or rely on Test drive. This is not a blocker
   for initial consolidation.
4. **Autosave/browser persistence** — agreed as a parity requirement, to be
   implemented after this report and before legacy removal.
5. **Tree detail/cards** — still open. Not a blocker for removal; evaluate only
   if large-tree performance or visual parity requires it.
6. **Repository topology** — still open. Keeping the standalone submodule
   preserves the multi-consumer goal; vendoring into Lapster simplifies CI and
   deployment. Either can satisfy “single editor codebase”.
7. **Second consumer** — still open. If the consumer-neutral goal fades,
   vendoring becomes more attractive.
8. **Removal timing** — still open. Production usage evidence should decide
   whether legacy remains for one release as a fallback.

ADRs recorded after this analysis:

- **ADR-0007** accepts track-ed as the sole supported editor, deprecates the
  legacy editor and gates removal on migration, autosave, CI coverage and a
  final parity check.
- **ADR-0008** accepts deferral of layouts and defines the future route-layer
  shape if multi-circuit authoring is prioritised.

## 9. Legacy project migration options

Do not block the geometry decision on migration, but decide before removing the
legacy page.

**Option A — one-time converter (best long-term user outcome)**

Add a converter from `lapster/track-project@6` to `track-ed/project@5`.

- Maps global points/sections and junction roles to track-ed centerlines,
  point ids and junction members.
- Maps legacy decorations to roadside splines/runs.
- Collapses multiple layouts to one (or rejects multi-layout projects).
- Drops start/finish, sectors and cameras if deprecated.
- Reports lossy conversions explicitly.

Effort: medium. Needs its own ADR/tests because the two schemas differ in
junction representation and decoration storage.

**Option B — read-only legacy window (lowest risk)**

Keep `/web/track-editor/` for one deprecation window, remove it from primary
navigation, and document that old projects must be re-authored or exported
before removal. Avoids a converter but leaves a migration cliff.

**Option C — declare legacy projects unsupported**

Archive the editor and document the break. Acceptable only if there is no
meaningful user data or if projects are considered ephemeral.

**Recommendation:** choose **A** if any user projects matter, otherwise **B**
for one release. Do not keep the legacy editor as a second active authoring path
indefinitely.

## 10. Recommendations

### 10.1 Primary recommendation

**Make track-ed the single editor and remove the legacy editor after a controlled
deprecation window.**

Benefits:

- one geometry pipeline and one set of geometry invariants;
- one export profile and one validation gate;
- removal of Rust/TS duplicate implementations of terrain, textures, props,
  trees, racing line and pack/export logic;
- smaller editor-only surface in the main wasm and fewer editor APIs;
- one authoring UX to test and document;
- junctions remain solved rather than partially reimplemented.

### 10.2 Do not merge the stale junction branch

`feat/junction-road-geometry` solved straight junctions but not curved branches,
predates current `dev`, and is superseded by track-ed. Archive it as historical
evidence and record the outcome. Do not spend more time bringing it forward.

### 10.3 Clean up track-ed before or during consolidation

- Remove hidden Kerbs/Walls/Props modes, panels, drag states and update
  functions after `project@5` is the only authoring format.
- Add autosave/browser project persistence (agreed).
- Add a focused browser smoke suite:
  - hosted track-ed boots;
  - edit a point;
  - create a junction;
  - export;
  - Test drive;
  - no page errors.
- Add track-ed tests to the Lapster gate on submodule changes, or merge the
  codebase into the monorepo.
- Apply the app refactors in section 10.5; keep the kernel and export layers
  unchanged.
- Pin production to a track-ed release tag once available.
- Retire old project schema versions after a migration window to keep the parser
  from growing indefinitely.

### 10.4 What to keep from the legacy editor

Keep the platform and selected assets/UX, not the legacy editor core.

| Keep | Rationale |
|---|---|
| `sim-render` renderer and viewers | Used by Drive/Garage/Track Viewer; not editor-specific. |
| `track-pack` / `track-pack-wasm` | Shared GLB-to-pack compiler; already powers hosted Test drive. |
| `track-camera`, HUD, loading, FPS, versioned wasm, pack cache | Shared viewer utilities used by other pages. |
| Hosted Test drive | Authoritative in-engine check; already integrated with track-ed. |
| Tree-card atlas and LOD renderer support | Keep only if track-ed later needs card-tree LOD; renderer support is generic. |
| Mountain backdrop algorithm | Useful if standalone track-ed preview needs parity. |
| Texture/prop/racing-line algorithms | Already ported into track-ed; keep attribution, not duplicate code. |
| Playwright scenarios | Reuse test intent for track-ed/hosted E2E. |
| Legacy project fixtures | Keep a representative corpus only if a migration tool is chosen. |
| ADR/junction-branch history | Keep as historical evidence; do not keep active code. |

Port to track-ed only where the product value is clear, in this order:
autosave/browser persistence; car marker/camera presets or Test drive only;
tree detail/LOD only if performance or visual parity requires it; FPS/draw-call
HUD; mountain backdrop only if standalone preview needs it.

Do not keep the legacy UI, project model, editor bake, editor wasm APIs, or its
duplicate terrain/props/textures/racing-line implementation.

### 10.5 Track-ed refactorings

Before legacy removal, make track-ed's app layer match the quality of its
kernel:

1. **Remove hidden legacy authoring paths** — delete the hidden Kerbs/Walls/Props
   modes, panels, drag states and update functions once `project@5` is the only
   authoring format. This is roughly 1,000 lines of redundant UI/code.
2. **Split `attachEditing` (~1,500 lines)** by mode: track/point editing,
   terrain, roadside painting, spawn and camera/overlay handling. Keep the
   pointer-state transitions testable.
3. **Split `createTrackView` (~1,360 lines)** into renderer/scene setup,
   surface overlays, picking and lifecycle/disposal modules.
4. **Decompose `splitClosedSection` (~347 lines)** and other large section-edit
   operations if they keep growing.
5. **Add browser smoke coverage** for boot, point edit, junction creation,
   export, Test drive and no-page-errors, in track-ed or Lapster CI.
6. **Add autosave/browser persistence** while preserving explicit JSON
   save/open.
7. **Add dependency/license gates** for the submodule: `npm audit`, notice
   check and track-ed units/build on pin changes.
8. **Benchmark a large venue** before considering a Worker or Rust/wasm kernel
   port.

### 10.6 Performance and Rust/wasm decision

Do not keep or refactor the legacy editor for performance reasons. The existing
evidence is not a fair comparison, and the legacy bake does less work. If
track-ed's large-venue bake becomes a UX bottleneck, profile first, then move
bake to a Web Worker, then port only the kernel to Rust/wasm behind the same
`BakedTrack`/export contract. An offline native compile remains a slow path for
enormous tracks, not the authoring loop.

### 10.7 Keep Test drive as the correctness anchor

The three.js preview is good for authoring, but the hosted Test drive exercises
the actual compiled pack and game renderer. Keep it in the release checklist for
editor changes, and make it part of automated CI where possible.

### 10.8 Preserve the neutral boundary

If a future Rust/wasm kernel or a second consumer becomes desirable, the
current `BakedTrack`/export-profile boundary is the right seam. Do not collapse
Lapster-specific names into the kernel to speed up UI work.

## 11. Proposed phased plan

**Phase 0 — Decisions and ADR**

- ADR-0007 (track-ed sole editor) and ADR-0008 (deferred layouts) are accepted.
- Decide legacy-project migration option A/B/C.
- Decide repository topology and second-consumer strategy.
- Confirm tree LOD/car camera/removal timing.

**Phase 1 — Parity, refactoring and safety**

- Add autosave/browser persistence to track-ed.
- Remove hidden Kerbs/Walls/Props UI and drag paths from track-ed.
- Split `attachEditing` and `createTrackView`; decompose other large app
  functions where they block testing.
- Add track-ed/hosted Playwright smoke coverage to Lapster CI.
- Add track-ed unit/build and dependency/license checks to the gate on submodule
  changes.
- Benchmark a large venue and decide whether a Worker is needed.
- Decide preview parity (car camera/Test drive only).

**Phase 2 — Migration**

- Implement the chosen legacy-project migration path.
- Update docs and menu copy; remove the legacy editor from primary navigation.
- Keep `/web/track-editor/` only if option B is chosen.

**Phase 3 — Removal**

- Delete `web/track-editor/`, legacy editor tests that only cover it, and the
  Rust editor module/API.
- Remove editor-only menu copy, docs and contracts that are no longer current.
- Keep viewer/shared UI (`track-camera`, HUD, loading, versioned wasm) where
  other pages still use it.
- Run the full gate and verify Test drive and Drive.
- Update `AGENTS.md`, `PROGRESS.md`, the domain map, tech debt and ADR history.

**Phase 4 — Future work**

- Banking/crown, layouts (ADR-0008), pit lanes, prop libraries, GPU-instanced
  trees, additional exporters — only after the single editor is stable.

## 12. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Removing legacy while user projects are unmigrated. | Choose option A or B before deletion; keep a deprecation window. |
| track-ed UI regressions because it has no E2E suite. | Add Playwright smoke tests before removal. |
| Submodule bump breaks hosted integration silently. | Run track-ed tests and hosted E2E in Lapster CI, or vendor the repo. |
| Large-venue bake stalls. | Keep bakes deferred; move kernel to Web Worker if needed; measure. |
| Preview divergence from the game renderer. | Use Test drive as the acceptance check; consider a car camera if needed. |
| Removing legacy loses a useful feature (tree LOD, car camera, autosave). | Autosave is agreed; evaluate tree LOD/car camera explicitly before removal. |
| track-ed parser complexity grows. | Retire old schema versions after migration; keep strict parsing. |
| Production depends on track-ed `main`. | Pin to a release tag; document the bump process. |
| track-ed app refactors introduce regressions. | Split incrementally; add browser smoke coverage before and during the split. |
| Geometry dependency maintenance stalls. | Keep the Clipper2 fallback and fixture conformance; add audit/license gates; replace only behind the kernel with proof. |
| A Rust/wasm port is justified by assumption rather than data. | Benchmark the equivalent large venue first; port only the kernel and keep the contract. |

## 13. Open items

Blocking before removal:

- legacy-project migration decision (section 9);
- autosave implementation (agreed);
- hosted track-ed/browser CI coverage;
- confirmation that multi-layout/routes/start-finish/sectors are deprecated.

Non-blocking but should be recorded:

- car-camera/exact-preview requirement;
- tree detail/cards requirement;
- repository topology and second-consumer strategy;
- removal timing and production usage evidence;
- track-ed app refactorings (`attachEditing`, `createTrackView`, hidden UI);
- large-venue performance benchmark and Worker/Rust-kernel decision;
- dependency/license audit gate for the track-ed submodule.

## 14. Bottom line

The legacy editor is not an inferior codebase because it is Rust; it is a
mature, coupled, production-shaped editor whose junction engineering arrived
late and never generalised. track-ed is the better editor base because its
geometry model, kernel boundary and validation process solve the actual problem:
one welded, testable venue surface with working spline junctions.

Its tradeoffs are explicit: roughly 2× the editor-specific implementation LOC,
an isolated npm dependency set, and an app layer with two oversized functions.
Those are tractable refactors. The legacy editor's smaller LOC reflects
delegation to shared platform code and a less complete feature set, not lower
complexity.

Consolidate on track-ed. Do not merge the stale junction branch. Preserve the
legacy editor only long enough to migrate projects and close the parity gaps
(autosave, CI coverage, and any explicitly required preview/tree features).
Refactor track-ed's app layer before or alongside legacy removal; do not
refactor the legacy editor toward track-ed.
