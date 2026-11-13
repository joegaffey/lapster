# 0004. B-spline control-point venue curves

- Status: accepted
- Date: 2026-11-12
- Deciders: architect, @joegaffey
- Related: ADR-0003, `docs/features/venue-graph-plan.md`

## Context

ADR 0003 chose a Catmull-Rom spline through the control points. That makes
every control point lie on the road, which is not the intended authoring model:

- control points define the shape of the road but do not have to lie on it;
- junctions must lie on the road; start/finish is a point on the route and may be
  an off-road handle whose contact point defines the line;
- the route must use the same curve as the section, so the layout and section
  overlays overlap;
- the existing `limit_centerline_curvature` silently bends the route away from
  the authored section (by up to ~9.5 m on the default loop), so the driven road
  and the authored section disagree.

## Decision

Replace Catmull-Rom with a cubic B-spline evaluator. Authored control points are
**handles**: they shape the curve and may sit off it. On-road anchors are
derived, not authored:

- start/finish: the nearest point on the layout route to the start/finish
  handle;
- junction: the nearest point on a designated host section to the junction
  handle. Attached sections use that contact point as their connection.

Open-section endpoints are clamped at their derived connection anchors. The host
section itself is never clamped, so a junction handle does not create a kink on
it.

Rules:

- Open sections are clamped at their anchors; interior anchors split the section
  into spans.
- Closed sections with no anchors are periodic B-spline loops. Closed sections
  with anchors are split into spans between anchors, with a seam.
- The curve is at least C1 continuous across anchors.
- The route is the ordered evaluation of section curves. There is no separate
  route spline, and no curvature limiter.
- Tight radii are validation and decoration-clearance concerns, not silent
  geometry edits.
- The editor draws the curve, a dashed control polygon, and a dotted
  perpendicular from each off-curve handle to its nearest contact point on the
  curve. On-curve handles show no dotted line.
- Authored points are handles. Junction and start/finish points may be handles.
  Open-section endpoints are derived anchors. A junction's on-road connection
  point is derived from a designated host section; attached sections use it as
  their endpoint.

## Consequences

### Positive

- Matches the intended authoring model: handles shape the road, anchors connect
  it.
- Section and route overlays overlap exactly.
- Authored geometry is the baked geometry; no hidden route correction.
- Junctions and start/finish are derived contact points; host sections stay
  smooth.
- The default loop has no anchors, so it is a smooth periodic B-spline with no
  start/finish clamp or kink.

### Negative / risks

- The evaluator is more complex (clamped/periodic B-spline, anchor spans, C1
  continuity).
- The default venue shape changes; handles need tuning.
- Contact points are a visual approximation (nearest point on the sampled
  curve), not a mathematical property of a B-spline handle.
- Anchor status is global, so a point cannot be an anchor in one section and a
  handle in another.

## Alternatives considered

- **Catmull-Rom through points** — rejected; controls must be on-curve and the
  authoring model is handles.
- **Bézier chain** — viable, but less local control and fussier for closed
  loops.
- **Fitted spline through anchors** — most flexible, hardest to keep
  deterministic and local.
- **Keep the curvature limiter** — rejected; it hides authored geometry and
  makes route and section disagree.

## Follow-up

- Update ADR-0003 and the venue-graph plan.
- Implement the evaluator, remove the curvature limiter, and make section and
  route share one evaluator.
- Tune the default handles so the default venue remains recognisable.
- Add tests for anchors on-curve, handles off-curve, C1 continuity, periodic
  loops, start/finish contact points, and route/section equality.
- Implement junction handles with a derived host-section contact point when the
  branch work starts; do not clamp the host section.
