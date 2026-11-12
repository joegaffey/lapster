# Plan: section-point venue graph rebuild

Status: `@5` Phases 1–2 are complete and committed. The model is reset to
`lapster/track-project@6` by `docs/adr/0003-section-point-venue-graph.md`.
Phase 3 (`@6` migration and loop-first editor) is next; the `@5` editor is
superseded.
Created: 2026-11-11
Updated: 2026-11-12
Owner: @joegaffey
Reference branch: `feat/multi-circuit-track` (`e03e138`)
Target branch: `feat/venue-graph`
Related: `docs/adr/0003-section-point-venue-graph.md`,
`docs/features/multi-layout-track-editor.md` (reference only, on the reference
branch), `docs/tech-debt.md`, `PROGRESS.md`

## 1. Why rebuild

The first `@5` pass proved the canonical graph, but it modelled junctions as
section endpoints and regenerated the default loop as one junction per control
point. A single loop therefore appeared as eight empty sections with no interior
control points, and connecting a branch forced a split. That is not the
authoring model:

- one loop is one layout;
- points shape a spline and are not necessarily connections;
- a junction is a connection between sections and may sit mid-section;
- start/finish is a layout choice, not the loop's parameter seam;
- loose branches render and drive;
- decoration is a surface overlay, not a graph point;
- per-point width is deferred.

`@6` keeps the proven graph/bake work but changes the model. `main` stays
deployable; multi-layout ships only after the new path is validated.

## 2. Locked decisions

- The only authoring model is the `@6` section-point graph.
- **Points** are global and never duplicated. Sections reference point ids.
- **Sections** are one spline each, open or closed. A closed section has a
  **seam**: the first point in its ordered list. The seam is a parameter origin
  only; it is never a junction or start/finish.
- **Junctions** are point roles: a point shared by two or more sections. A
  junction may sit mid-section. Creating one does not split the authored
  section; the renderer may subdivide internally at the anchor for bake and
  collision.
- **Start/finish** is a layout reference to a point id. A point may be both a
  junction and a layout's start/finish. Start/finish may differ per layout.
- **Routes are legs**: ordered `{ sectionId, entry, exit, wrap? }` where
  entry/exit are a point id or the seam. Layouts are closed leg loops; paths are
  open or closed leg chains.
- **Loose ends** are open-section endpoints without a junction role. They are
  valid in the editor and the baked track; default end treatment is `cap`.
- **Decorations** are ordered section-fraction chains. They never use
  control-point anchors, may start and end in different sections, and may wrap a
  closed section's seam.
- **All sections are baked**, including loose branches not referenced by a
  layout or path.
- Spline: Catmull-Rom through points with per-section endpoint tangents and a
  derived wrap-around closure tangent for closed sections.
- Per-control-point width, asymmetric profiles, and closed decoration loops are
  deferred.
- No `@3`/`@4` compatibility layer, no `EditorProject`, no route-level
  decoration API, and no persistent `@5` document.
- `main` remains deployable throughout.

## 3. Schema

Use a fresh tag: `lapster/track-project@6`.

```text
TrackProject {
  schema: "lapster/track-project@6"
  name
  settings
  points: [{
    id, position,
    junction?: { kind },
    end?: "cap" | "open" | "barrier"     // loose-end treatment
  }]
  sections: [{
    id, closed,
    points: [pointId...],                // ordered control points
    fromTangent?, toTangent?             // open ends; closed derives closure
  }]
  layouts: [{
    id, name,
    route: [Leg...],                     // closed leg loop
    startFinish: pointId,
    spawns, sectors, cameras
  }]
  paths: [{
    id, kind,
    route: [Leg...],                     // open or closed
    boxes
  }]
  decorations: [{
    id, kind,
    chain: [DecorationLeg...],           // ordered, contiguous
    elements: [length...],
    phase,
    propKind?, spacing?
  }]
}

Leg            { sectionId, entry: pointId | seam, exit: pointId | seam, wrap? }
DecorationLeg  { sectionId, entryFraction, exitFraction, wrap? }
```

`JunctionKind` covers normal, fork, pit entry, and pit exit. `StartFinish` is
not a junction kind.

### Schema rules

- Unique ids for points, sections, layouts, paths, and decorations.
- Finite points and tangents; tangents normalized on load.
- Sections reference known points. Open sections need at least two points;
  closed sections need at least three.
- A shared point is the same point object. A junction role requires two or more
  section memberships; a loose end has none.
- Layout routes are contiguous closed leg loops: each leg's exit matches the
  next leg's entry. Start/finish references a point on a section in the route.
- Path routes are contiguous leg chains.
- Decoration chains are ordered and contiguous; anchors are fractions in
  `[0, 1]`; element lengths are finite and positive; phase is finite.
- No two decoration paths may overlap on the same side/lane and section range.
- All sections are part of the baked world.

## 4. Points, sections, and junctions

- Clicking a section point selects the shared point; dragging it moves every
  section that references it.
- Clicking a section selects the section; its points become editable handles.
- Hovering/clicking the road inserts a new interior point at that fraction.
- **Make junction** on a point adds the junction role and creates a short open
  branch section from that point to a loose end, perpendicular to the local
  tangent by default. The parent section is not split.
- The loose end renders with a cap until it is connected to another junction.
- A point may be marked as the active layout's start/finish.
- Deleting a point is blocked when it is shared, a junction, a start/finish, or
  would leave a section below its minimum point count.
- Section tangents are per section; a shared junction may curve differently in
  each section.

## 5. Decorations

- Kerbs and props use the same decoration object; `kind` selects bake rules.
- Anchors are section + fraction, never point ids.
- Chains may span sections and cross junctions; `wrap` handles a closed
  section's seam.
- Kerb element lengths are physical metres and may vary per element.
- Kerb colour alternation follows element order/phase across section boundaries.
- Prop placement uses lane, kind, spacing, and phase; spacing continues across
  section boundaries.
- Decorations refer to lanes, not absolute world offsets, so width/profile
  changes move them with the road. Width variation is deferred.
- Junction clearance and mouth clipping are bake-time concerns.
- Editing a point reparameterises the decoration; it does not rewrite it into
  point anchors.
- If sections are split by an explicit tool later, split affected decoration
  chains and rebase anchors; never silently join across a gap.

## 6. Reuse from the reference branch

Port selectively; do not merge the branch.

Useful:
- canonical section sampling and shared-edge baking ideas
- junction pad and collision continuity work
- path baking and pit-box derivation
- path editing operations and wasm bindings
- editor selection, make unique, and point editing structure
- `@4` Playwright test structure and helpers
- validation and migration test ideas

Not reusable:
- `@3`/`EditorProject` compatibility layer
- `@4` dual schema / legacy `pitLane`
- junction-only-at-endpoints assumptions
- layout-level kerb/prop APIs
- route-level painting fallback
- per-segment decoration as canonical storage

## 7. Phases

Every phase must leave the editor and test drive working. Run the full gate at
the end of each code phase.

### Phase 1 — `@5` graph and single-loop parity (historical)

Complete on `feat/venue-graph`: canonical junctions, segments, one layout loop,
default procedural track, bake, spawn, pack export. Superseded by the `@6`
model reset.

### Phase 2 — `@5` canonical decoration paths (historical)

Complete: section-chain decoration paths, bake-time slicing, continuous phase,
junction clearance, split reparameterisation. The decoration data model carries
forward with fraction anchors.

### Phase 3 — `@6` section-point model and loop-first editor

#### 3a — Model and bake migration (render)

- Replace `Segment`/endpoint junctions with global points and open/closed
  sections.
- Add seam and wrap-around closure tangents.
- Make junctions point roles, including mid-section anchors; subdivide at bake
  time without splitting the authored section.
- Allow loose ends and cap them; bake all sections.
- Add route legs with point/seam entry/exit and wrap.
- Convert decorations to section-fraction chains with cross-section and seam
  wrap support.
- Default project: one closed section, zero junctions, one layout, start/finish
  on a point.
- Expose the `@6` editor wasm API (see section 10).

#### 3b — Loop-first editor (web)

- Select and drag points and sections.
- Hover/click the road to insert a point.
- **Make junction** adds the role and a short capped branch; do not split.
- Mark the layout start/finish point.
- Edit section tangents.
- Delete points/sections where safe.
- Save/load/export/import `@6`; keep Test Drive working.
- Rewrite the track-editor Playwright suite for the new model.

Acceptance:
- default track drives and renders with one closed section and no junctions;
- points edit every referencing section;
- making a junction adds a capped branch without splitting;
- loose ends render and drive;
- full gate passes.

### Phase 4 — Multiple loops over shared sections

- Add/duplicate/delete layouts; route selector.
- Route legs across shared sections and junctions.
- Make unique clones a section for one layout.
- Per-layout start/finish and spawn metadata.

Acceptance:
- shared sections appear once in visual and collision;
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

- Remove dead scaffolding and stale docs.
- Update ADRs, `PROGRESS.md`, and tech debt.
- Run the full gate.
- Validate the dev preview.
- Release to `main` through the protected PR flow.

## 8. Test strategy per phase

- Rust: point/section graph validation, shared-point dedupe, seam/closure
  tangents, mid-section junction subdivision, loose-end caps, route legs and
  wrap, decoration fraction chains and seam wrap, bake determinism.
- Web unit: `@6` schema round-trip, validation, editor state helpers.
- Playwright: default loop, point insert/move/delete, make junction + branch,
  start/finish, decoration chains across sections, multiple loops, make unique,
  pit path.
- Every phase: `cargo fmt --check`, clippy, workspace tests, wasm check,
  `npm --prefix web test`, `npm --prefix web run test:viewer`.

## 9. Risks

- Mid-section T-junction bake and collision continuity is the highest-complexity
  area. Subdivide at anchors and validate visually on the dev preview.
- Route legs and wrap can produce non-contiguous or doubled geometry if
  validation is loose.
- Loose-end caps need a visual and collision policy.
- Seam handling for decorations and element phase needs explicit tests.
- Width is still constant; per-point width is deferred and will touch bake,
  lanes, decoration, and editor when added.
- Pack size and draw calls with all sections baked.
- Do not let `@5` or the reference branch become a co-equal model.

## 10. Phase 3 `@6` wasm interface (proposed)

The web editor consumes the `@6` project through these `TrackViewer` bindings.
The JSON schema is the contract; exact signatures may be refined by the render
worker.

Read-only (JSON strings):

- `editor_project()` — normalized `@6` document.
- `editor_points()` — points with roles and positions.
- `editor_sections()` — sections with point ids and closed flag.
- `editor_route()` — active layout legs.
- `editor_section_centerlines()` — every section centerline keyed by id.
- `editor_route_centerline()` — active layout centerline.
- `editor_kerb_spans()` / `editor_prop_lines()` — bake-derived decoration.

Editing (async, rebuild before resolving):

- `set_editor_project(json)`.
- `insert_editor_point(sectionId, afterIndex, x, z) -> pointId`.
- `move_editor_point(pointId, x, z)`.
- `delete_editor_point(pointId)`.
- `make_editor_junction(pointId) -> branchSectionId` — add the role and a short
  capped branch; never split.
- `set_editor_section_tangents(sectionId, fromJson, toJson)`.
- `set_editor_start_finish(pointId)`.
- `editor_track_pack()`.

## 11. Definition of done

- One `@6` document type only.
- Global points with no duplication; junctions and start/finish are point
  references/roles.
- Sections are open or closed; the seam is parameterization only.
- Mid-section junctions bake without authored splits.
- Loose ends render and drive with caps.
- Route legs and decoration fraction chains, including seam wrap.
- Per-point width and closed decoration loops are explicitly deferred.
- No `@3`/`@4`/`@5` compatibility layer in the runtime or editor.
- Full gate green, dev preview validated, `PROGRESS.md` and tech debt updated.
