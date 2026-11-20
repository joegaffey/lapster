# 0007. Promote track-ed as the sole track editor

- Status: accepted
- Date: 2026-11-20
- Deciders: architect, @joegaffey
- Related: `docs/editor-consolidation-report.md`,
  `docs/features/editor-consolidation-plan.md`, ADR-0005, ADR-0006, ADR-0008,
  TD-0008, `docs/track-ed-integration.md`

## Context

Lapster ships two track editors:

- the legacy procedural editor at `web/track-editor/`, backed by
  `crates/sim-render/src/track_editor.rs`;
- track-ed at `web/track-ed-src`, hosted at `web/track-ed/` and already
  integrated with the `track-pack` compiler and Drive Test drive.

The consolidation report `docs/editor-consolidation-report.md` found that
track-ed is the only editor with working junctions and general venue topology.
The legacy editor still bakes independent full-width section ribbons on `dev`,
which produces overlaps, z-fighting and holes at angled joins (TD-0008). The
unmerged `feat/junction-road-geometry` branch implements stitched junction
surfaces for straight two-point branches only, rejects curved branches, and is
based on an older `dev`.

The report also found that track-ed's success is primarily architectural and
process-related: a planar-arrangement geometry model, a consumer-neutral kernel,
a deterministic export gate and an executable fixture matrix. The language
difference is a contributing iteration-speed factor, not the root cause.

Prioritisation decisions recorded with the report:

- multi-layout, routes, start/finish and sectors are treated as deprecated;
- autosave/browser project persistence is a required track-ed parity item;
- legacy project migration and repository topology remain open decisions;
- preview parity is an open product question, with hosted Test drive as the
  authoritative in-engine check.

This ADR records the promotion decision and the conditions under which the
legacy editor is removed. It does not itself deploy, merge or delete code.

## Decision

- **track-ed is the sole supported track editor.** New authoring, geometry and
  export work targets `web/track-ed-src` and the `track-pack` integration.
- The legacy procedural editor is **deprecated** and is retained only as a
  temporary migration and fallback path.
- No further junction geometry work is done in the legacy editor. Do not merge
  `feat/junction-road-geometry`; it is superseded by track-ed's general
  junction bake.
- Removal of the legacy editor is gated on:
  1. a legacy-project migration decision and its implementation or documented
     archive path;
  2. autosave/browser project persistence in track-ed;
  3. automated coverage for hosted track-ed and the compiler/Test drive path;
  4. a final parity check for any explicitly retained legacy-only feature.
- track-ed's hosted **Test drive in Lapster** remains the authoritative
  end-to-end check for editor output. The three.js preview is an authoring aid,
  not a substitute for the compiled game path.
- The multi-layout/route/start-finish/sector model is not carried into track-ed
  as part of this promotion. Its future handling is recorded in ADR-0008.
- Repository topology is not changed by this ADR. track-ed remains a pinned
  submodule for now. Vendoring it into the Lapster repository or moving to a
  different release model requires a separate ADR.
- Shared viewer code remains in Lapster even though the legacy editor also uses
  it: `web/src/ui/track-camera.ts`, the HUD/loading/FPS helpers, versioned wasm
  loading and the pack cache keys are not editor-only by definition.

## Consequences

### Positive

- One geometry pipeline, one baked-track contract, one export profile and one
  validation gate for track authoring.
- Working spline and multi-way junctions remain the supported behaviour.
- Removal of the legacy editor eliminates the duplicate Rust/TypeScript
  implementations of terrain, textures, props, trees, racing line and pack
  export.
- The main `sim_render` wasm can shed editor-only APIs and the large
  `track_editor` module after removal.
- One authoring UX, one project format direction and one set of editor tests.
- The consumer-neutral track-ed kernel remains available for a second export
  profile or a future kernel replacement.

### Negative / risks

- Production builds depend on the track-ed submodule and its pinned commit.
- track-ed currently has strong kernel tests but no browser/E2E suite of its
  own; hosted Test drive is not covered by Lapster CI.
- Legacy projects in `lapster/track-project@6` cannot be opened by track-ed
  until a migration path exists.
- track-ed lacks autosave and has no car-camera/exact-renderer preview while
  editing.
- Large-venue bake time may become a UX limit as authoring scope grows.
- The completion date depends on the open migration and repository decisions.

## Alternatives considered

- **Keep both editors indefinitely** — rejected; doubles authoring logic,
  project formats, tests, documentation and visual conventions while the legacy
  editor still cannot support general junctions.
- **Fix the legacy editor and keep it as the primary editor** — rejected; the
  general planar-arrangement work already exists in track-ed, and retrofitting
  it into the coupled production bake would repeat the work and carry the
  ribbon-based assumptions forward.
- **Merge `feat/junction-road-geometry` and defer curved branches** — rejected;
  it leaves the editor unable to author the spline branches track-ed already
  supports, and the branch is stale relative to current `dev`.
- **Port track-ed's kernel to Rust/wasm first** — rejected for this decision;
  it adds work without changing the product outcome. Revisit only if profiling
  shows the TypeScript kernel is a real bottleneck.
- **Vendor track-ed into Lapster immediately** — not rejected on merit, but
  deferred; repository topology is a separate strategic decision.

## Follow-up

1. Decide the legacy-project migration path: converter, read-only deprecation
   window, or documented unsupported archive.
2. Add autosave/browser project persistence to track-ed.
3. Add track-ed unit/build tests and a hosted track-ed/Test-drive browser smoke
   test to Lapster CI, or move track-ed into the monorepo gate.
4. Remove the legacy editor from primary navigation and update the Track Editor
   card copy.
5. After the migration window, delete `web/track-editor/`, the legacy editor
   Playwright cases that only cover it, the editor-only APIs in
   `crates/sim-render/src/lib.rs`, and `crates/sim-render/src/track_editor.rs`.
6. Keep any shared viewer utilities that other pages still use.
7. Update `AGENTS.md`, `PROGRESS.md`, the domain map, `docs/tech-debt.md` and
   this ADR index after removal.
8. Pin the submodule to a track-ed release tag once track-ed begins making
   releases.
9. Record the final removal and the migration outcome in a follow-up ADR
   before the legacy code is deleted.
