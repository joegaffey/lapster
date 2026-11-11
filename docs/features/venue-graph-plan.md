# Plan: canonical venue graph rebuild

Status: agreed implementation plan, not started
Created: 2026-11-11
Owner: @joegaffey
Reference branch: `feat/multi-circuit-track` (`e03e138`)
Target branch: `feat/venue-graph`, branched from `main`
Related: `docs/adr/0006-drivable-venue-network.md` (reference only),
`docs/features/multi-layout-track-editor.md` (reference only),
`docs/tech-debt.md`, `PROGRESS.md`

## 1. Why rebuild

The reference branch proved the canonical-segment model but kept too many
coexisting models: `@3` loop/shared-point documents, a separate `EditorProject`,
layout-level decoration APIs, route-level paint fallbacks, and legacy `pitLane`
compatibility. Props and kerbs remain inconsistent because decoration is
modelled more than once.

`main` is the known-good single-loop baseline. The rebuild starts there and
adds multi-layout capability incrementally, with no compatibility layer between
models.

## 2. Locked decisions

- The only authoring model is the canonical venue graph.
- Junctions are shared endpoints.
- Segments are canonical connected splines between junctions.
- Layouts are closed loops of segment ids.
- Paths are open segment chains; the pit lane is a path, not a layout.
- The baked world is the union of every unique segment referenced by any layout
  or path. Shared segments are baked once.
- Layout selection changes spawn and metadata only; it never filters geometry.
- Decoration is stored as **canonical decoration paths**, not per-segment
  spans.
- Decoration paths are independent of layouts and shared by every layout that
  uses the referenced segments.
- Route direction is canonical. Whole-route reversal, per-segment width, and
  asymmetric profiles remain deferred.
- There is no persistent `@3` document, no `EditorProject` adapter, no
  route-level decoration API, and no compatibility `pitLane` field.
- `main` remains deployable throughout. Multi-layout ships only after the new
  path is validated.

## 3. Schema

Use a fresh tag: `lapster/track-project@5`.

Do not reuse `@4`; the reference branch uses that tag for a different dual-model
implementation. `@5` has one document:

```text
TrackProject {
  schema: "lapster/track-project@5"
  name
  settings
  junctions: [{ id, position, kind, profile? }]
  segments: [{
    id, from, to, points, fromTangent?, toTangent?,
    startProfile?, endProfile?
  }]
  layouts: [{
    id, name, route: [segmentId...],
    startFinish: { segmentId, fraction },
    spawns, sectors, cameras
  }]
  paths: [{
    id, kind, route: [segmentId...], boxes
  }]
  decorations: [DecorationPath...]
}
```

Decoration path:

```text
DecorationPath {
  id
  kind: kerb | prop
  side/lane
  segments: [segmentId...]       // ordered, contiguous canonical chain
  start: { segmentId, fraction }
  end:   { segmentId, fraction }
  elements: [length...]          // variable kerb element lengths
  phase
  propKind?                      // prop paths only
  spacing?                       // prop paths only
}
```

Per-segment decoration slices are derived at bake time. They are never edited
directly and never serialized as a second source of truth.

### Schema rules

- Unique ids for junctions, segments, layouts, paths, and decoration paths.
- Finite points and tangents; tangents normalised on load.
- Segment endpoints reference known junctions.
- Layout routes close and traverse segments in canonical direction.
- Path routes are open canonical chains.
- Decoration chains are ordered and contiguous.
- Decoration start/end anchors reference known segments and fractions in
  `[0, 1]`.
- Decoration path endpoints and element lengths must be finite and positive.
- No two decoration paths may overlap on the same side/lane and segment range.
- Every segment referenced by a decoration path is part of the baked world even
  if no layout or path references it.

## 4. Decoration paths

- Kerbs and props use the same path object; `kind` selects the bake rules.
- Kerb element lengths are physical metres and may vary per element.
- Kerb colour alternation follows element order/phase across segment boundaries.
- Prop placement uses lane, kind, spacing, and phase; spacing continues across
  segment boundaries.
- Decoration paths refer to lanes, not absolute world offsets, so width/profile
  changes move them with the road.
- Junction clearance and mouth clipping are bake-time concerns.
- Editing a decorated segment reparameterises the path; it does not rewrite it
  into per-segment spans.
- Splitting a segment replaces one id with two in every affected decoration
  chain and rebases endpoint anchors.
- Merging segments concatenates chains only when the result is unambiguous.
- Deleting a segment splits affected decoration paths or blocks the deletion;
  it never silently joins decoration across the gap.
- Make unique clones the affected decoration slices onto the new segment so the
  selected layout keeps its decoration while others keep the original.

## 5. Reuse from the reference branch

Port selectively; do not merge the branch.

Useful:
- canonical segment sampling and shared-edge baking
- junction pad and collision continuity work
- path baking and pit-box derivation
- path editing operations and wasm bindings
- editor route/segment selection, make unique, and segment point editing
- `@4` Playwright test structure and helpers
- validation and migration test ideas

Not reusable:
- `@3`/`EditorProject` compatibility layer
- `@4` dual schema/legacy `pitLane`
- layout-level kerb/prop APIs
- route-level painting fallback
- per-segment decoration as canonical storage

## 6. Phases

Every phase must leave the editor and test drive working. Run the full gate at
the end of each code phase.

### Phase 1 — Graph model and single-loop parity

- Add junctions, canonical segments, one layout loop.
- Build the default procedural track from the graph.
- Keep current single-loop visuals, physics collision, spawn, and pack export.
- No new authoring UI required beyond loading the graph.

Acceptance:
- default track drives and renders with parity to `main`;
- shared-edge bake is deterministic;
- layout metadata and start/finish are produced from the graph;
- full gate passes.

### Phase 2 — Canonical decoration paths

- Add decoration paths (kerb and prop) over segment chains.
- Bake per-segment slices from paths.
- Support variable kerb element lengths and continuous phase.
- Apply junction clearance at bake time.
- No route-level decoration APIs are introduced.

Acceptance:
- a decoration path crossing segment boundaries bakes without duplicated
  geometry;
- kerb/prop patterns remain physically consistent;
- split/junction edits reparameterise paths correctly;
- full gate passes.

### Phase 3 — Project persistence and segment editing

- Save/load/export/import `@5`.
- Drag junctions; add/delete/drag segment interior points; edit tangents.
- Split and merge segments with decoration path updates.
- Keep `test drive` working after every edit.

Acceptance:
- round-trip project;
- validation rejects dangling references and bad anchors;
- edits preserve layout closure and decoration continuity;
- full gate passes.

### Phase 4 — Multiple loops over shared segments

- Add/duplicate/delete layouts; route selector.
- Layouts reference shared segments; no geometry duplication.
- Make unique clones a segment for one layout.
- Per-layout start/finish and spawn metadata.

Acceptance:
- shared segments appear once in visual and collision;
- switching layouts changes spawn/metadata only;
- make unique diverges one layout without changing the other;
- full gate passes.

### Phase 5 — Pit lane path

- Add/append/remove/delete path UI.
- Bake pit paths and pit boxes.
- Paths are not listed as layouts or selectable via `?layout=`.

Acceptance:
- pit lane is drivable;
- pit path geometry is baked once;
- pit boxes reach layout metadata;
- full gate passes.

### Phase 6 — Cleanup and release

- Remove dead reference scaffolding and stale docs.
- Update ADRs, `PROGRESS.md`, and tech debt.
- Run the full gate.
- Validate the dev preview.
- Release to `main` through the protected PR flow.

## 7. Test strategy per phase

- Rust: graph validation, bake determinism, shared-edge dedupe, decoration path
  slicing, junction clearance, split/merge, path baking.
- Web unit: schema round-trip, validation, editor state helpers.
- Playwright: load graph, edit junctions/segments, decoration paths across
  boundaries, multiple loops, make unique, pit path, legacy import only if
  enabled.
- Every phase: `cargo fmt --check`, clippy, workspace tests, wasm check,
  `npm --prefix web test`, `npm --prefix web run test:viewer`.

## 8. Risks

- Decoration path reparameterisation on split/merge is the highest-complexity
  area. Implement split/merge before multi-layout to avoid retrofitting.
- Junction clearance and kerb/prop continuity need visual validation on the dev
  preview.
- Closed decoration loops need a seam policy; defer if not needed for the first
  playable venue.
- Pack size and draw calls with all layouts in one world.
- Do not let legacy migration become a co-equal model. If migration is required,
  make it a one-way import path only.

## 9. Definition of done

- One `@5` document type only.
- One decoration path model only.
- All layouts drivable in one world without pack switching.
- Shared segments baked once.
- Decoration follows shared segments and survives edits.
- Pit lane drivable and not a layout.
- No `@3`/`@4` compatibility layer in the runtime or editor.
- Full gate green, dev preview validated, `PROGRESS.md` and tech debt updated.
