# 0003. Section-point venue graph with mid-section junctions

- Status: accepted; spline evaluation superseded by ADR-0004
- Date: 2026-11-12
- Deciders: architect, @joegaffey
- Related: `docs/features/venue-graph-plan.md`, `AGENTS.md`, ADR-0002

## Context

The `@5` model treated junctions as section endpoints. Every control point in
the default loop became a junction, so a single loop appeared as eight
independent sections with no interior control points. That does not match the
intended authoring model:

- one loop is one layout;
- control points shape a spline and may or may not be connection points;
- a junction is a connection between sections and can sit mid-section;
- start/finish is a layout choice, independent of the loop's parameter seam;
- loose-ended branches are valid and render/drive;
- decoration is a surface overlay, not a graph point;
- per-control-point width is deferred.

## Decision

Adopt `lapster/track-project@6` with a section-point graph:

- **Global points.** One `points` list; sections reference point ids. Points are
  never duplicated. A shared point is the same object in every section.
- **Junctions are point roles.** A point is a junction when two or more
  sections connect there. It may be mid-section. Creating a junction does not
  split the authored section; the renderer may subdivide internally at the
  anchor for baking and collision.
- **Start/finish is per layout.** `layout.startFinish` references a point id.
  A point may be both a junction and a layout's start/finish.
- **Sections are open or closed.** A closed section has a seam: the first point
  in its ordered list. The seam is a parameter origin only, never a junction or
  start/finish. Closed sections derive a wrap-around closure tangent.
- **Routes are legs.** Layout and path routes are ordered legs over sections.
  A leg's entry/exit is a point id or the section seam, with an optional `wrap`.
- **Decorations are section-relative.** A decoration is an ordered chain of
  section legs with fraction anchors, element lengths, and phase. It never uses
  control-point anchors, may start and end in different sections, and may wrap a
  closed section's seam.
- **Loose ends are valid.** An open-section endpoint without a junction role is
  a loose end. It renders and drives; the default end treatment is `cap`.
- **Bake all sections.** Sections are baked whether or not a layout or path
  references them, so loose branches render.
- **Deferred.** Per-control-point width, asymmetric profiles, and closed
  decoration loops are not part of `@6`.

Remove the `Segment` type name and `JunctionKind::StartFinish`. There is no
`@3`/`@4` compatibility model, no `EditorProject`, and no route-level decoration
API.

## Consequences

### Positive

- The authoring model matches the design: loop-first sections, points, and
  connection points.
- No duplicated points, so geometry cannot drift between sections.
- Mid-section junctions avoid forced splits and make shared road networks
  natural.
- Loose branches and dead ends are representable.
- Decorations stay independent of the graph and follow the road surface.

### Negative / risks

- Mid-section T-junction baking is harder than endpoint junctions; the renderer
  must subdivide at anchors and blend surface, kerb, and collision.
- Route legs and seam wrapping add validation and sampling complexity.
- Loose-end caps need a collision and visual policy.
- Width remains global/constant; decoration lanes assume it.
- `@5` projects are not migrated; this is a pre-release in-place reset.

## Alternatives considered

- **Junctions only at section endpoints, split on connection.** Rejected: it
  forces splits, duplicates intent, and produced the `@5` junction-per-point
  default.
- **One self-loop section at a junction seam.** Rejected: the seam is not a
  junction, and a seam junction would conflate parameterization with
  connectivity.
- **Decoration anchors on control points.** Rejected: decoration is
  surface-relative and must start/end anywhere, including across sections.
- **Routes as whole-section lists.** Rejected: cannot express entering or
  leaving a loop at a mid-section junction.
- **Per-point width now.** Deferred; it changes bake, lane offsets, and the
  editor, and should be its own ADR.

## Follow-up

- Replace `@5` with `@6` in the render and web layers (pre-release, no
  compatibility layer).
- Update `docs/features/venue-graph-plan.md`, `PROGRESS.md`, and tech debt.
- Implement `@6` bake: closure tangents, mid-section junction subdivision,
  loose-end caps, route legs, decoration fraction chains.
- Re-scope the editor around loop-first sections and points.
- Consider closed decoration loops and per-point width in later ADRs.
