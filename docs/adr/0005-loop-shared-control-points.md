# 0005. Loop and shared-control-point model

- Status: accepted
- Date: 2026-11-09
- Supersedes: ADR 0004
- Deciders: @joegaffey, architect
- Related: `docs/features/multi-layout-track-editor.md`,
  `crates/sim-render/src/track_editor.rs`

## Context

ADR 0004 introduced explicit edges, routes, and a variants UI. That model is
more machinery than the editor needs. The natural authoring model is:

- a venue has loops;
- a loop is a closed sequence of control points;
- control points can be shared by several loops;
- shared control points produce shared spline pieces;
- kerbs and props are sub-ranges of a loop segment.

The explicit edge list and route-range UI should be removed.

## Decision

### Topology

- **Loop:** a closed ordered list of control-point ids. A loop is the layout
  and the unit the user selects and test-drives.
- **Control point:** a shared object `{ id, position, kind, tangent? }`. Moving
  it updates every loop that references it.
- **Segment:** the ordered consecutive pair `(A -> B)` inside a loop.
- **Shared segment:** a continuous piece of center spline used by more than one
  loop. Operationally, the piece between two consecutive shared control points
  traversed in the same direction by those loops.
- A single shared control point is a junction, not a shared segment. If a shared
  stretch begins or ends mid-segment, a control point is inserted at that
  boundary.
- Segments are derived from loops; there is no separate route/edge list in the
  editor.

### Spline and tangents

- Control points are handles, not guaranteed to lie on the final best-effort
  centerline.
- Each control point has one tangent shared by every loop that references it.
  An explicit tangent may be stored; otherwise it is the normalized average of
  incident directions across all loops.
- A shared segment is sampled once from its shared endpoints and tangents, so it
  is geometrically identical in every loop.
- Loop-specific curvature relaxation may be applied only to unique pieces.

### Decorations

- Kerb spans and prop spans belong to a segment and use 0..1 fractions along it.
- If a segment is shared, its decorations are shared.
- A loop that needs different decoration on a shared piece must make the piece
  unique by making a boundary control point unique.


### Branch creation

A user can create a new loop by branching from an existing loop:

1. Select control point `J` on a loop.
2. Create a branch control point `P1` perpendicular to the loop at `J`.
3. Append further branch points at the open end.
4. Select control point `K` on the original loop to close.
5. The new loop is `J -> P1 -> ... -> Pn -> K -> (original points after K, back to J)`.

The original loop stays `J -> ... -> K -> ... -> J`. The branch replaces the
`J -> K` arc for the new loop; the other original arc is shared, traversed in
the original direction.

Branch direction selects the shared arc: a `J -> K` branch shares the `K -> J`
arc; a `K -> J` branch shares the `J -> K` arc.

Perpendicular placement is only the initial seed. After the user drags or adds
points, the branch is a free spline. The simple tangent rule is:

- a branch edge at a junction starts/ends along the edge direction;
- a main-loop edge keeps the main-loop tangent;
- the original loop geometry is unchanged when a branch is added;
- junction pads blend main and branch cross-sections.

Branch edges are marked with role `branch` in the project; main edges use role
`main`. Shared segments keep one canonical tangent.

Start/finish can be shared or per-loop. The first version assumes one shared
start/finish when it lies on the shared arc, but the model keeps a per-loop
start/finish reference.

### Width and profiles

- One global track width/profile for now. Per-control-point width is deferred.
- Offset splines (asphalt, runoff, kerb, prop) are relative to the loop
  centerline and follow it when control points move.

### Start/finish and pit

- Start/finish is a control-point kind; it may be shared or unique to one loop.
- One shared pit lane later, modelled as an open path between pit-entry and
  pit-exit control points.

### Project document

Bump the editor project to `lapster/track-project@3`:

```json
{
  "schema": "lapster/track-project@3",
  "name": "Venue",
  "settings": { "width": 12.0, "kerb": {} },
  "loops": [
    { "id": "full", "name": "Full", "controlPoints": ["cp1", "cp2", "cp3"] }
  ],
  "controlPoints": [
    { "id": "cp1", "x": 0, "z": 0, "kind": "start_finish" }
  ],
  "segments": [
    { "from": "cp1", "to": "cp2", "kerbs": [], "propSpans": [] }
  ],
  "pitLane": null
}
```

Asset schema remains v4. The baker may derive an internal graph from loops and
segments; that graph is not part of the editor model.

### Editor UX

- Loop list replaces the layout/variant list.
- Select a loop to edit.
- Shared control points are visually distinct.
- Drag a shared point: all dependent loops update.
- **Make unique** copies a shared point for the selected loop only.
- **Duplicate loop** initially shares all control points; use Make unique to
  diverge.
- Add/delete points on the selected loop.
- Kerb/prop painting attaches to the selected segment; shared segment means
  shared decoration.
- Remove the variants panel, route ranges, edge list, and shared/unique badges.

### Deletion

Start with: delete from the selected loop only; keep the point if another loop
references it; refuse if the loop would drop below three points; remove
unreferenced points. Experiment before adding global delete.

### Migration

Migrate local `@2` projects: each route becomes a loop of control-point ids
derived from its edge chain; segment decorations come from the old edges.

## Consequences

### Positive

- Matches the user's mental model.
- Much smaller editor surface.
- Shared sections arise naturally from shared points.
- Moving a shared handle updates all dependent loops.
- The geometry baker can still sample shared segments once.

### Negative / risks

- Shared-segment identity depends on the same consecutive ordered pair.
- Shared tangents and best-effort sampling need careful implementation.
- Duplicate/make-unique UX must be clear.
- Migration from `@2` and local saved projects.

## Alternatives considered

- Keep ADR 0004 edges/routes/variants UI: rejected as too complex.
- Geometric overlap detection for shared pieces: rejected; require shared
  control points at shared-piece boundaries.
- Off-curve B-spline control points: deferred; current handles remain
  best-effort/on-curve enough for the editor.

## Follow-up

- Rewrite the feature brief around loops and shared points.
- Replace the public `@2` project with `@3`.
- Keep asset schema v4 and the existing shared-geometry bake underneath.
- Implement loop/point UI and tests.
