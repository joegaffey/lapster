# 0005. Junction road geometry

- Status: accepted
- Date: 2026-11-14
- Deciders: architect, @joegaffey
- Related: ADR-0003, ADR-0004, `docs/features/venue-graph-plan.md`, TD-0008

## Context

ADR 0003 and ADR 0004 define junctions as point roles with a derived on-road
contact point. That graph model is sound, but it does not define a junction
**surface**.

The current `make_junction` action is a graph-editing primitive: it adds the
junction role and a short open branch section. The renderer then bakes every
section as an independent full-width ribbon:

- asphalt from `-ASPHALT_HALF_WIDTH..ASPHALT_HALF_WIDTH`;
- dirt shoulders from `ASPHALT_HALF_WIDTH..OUTER_EDGE_RADIUS` on both sides;
- no clipping, stitching, or shared boundary vertices at junctions.

Consequences at a junction:

- host and branch ribbons overlap;
- dirt and grass from one section draw over another section's tarmac at angles;
- depth fighting and holes appear where the overlapping triangles meet;
- collision surfaces disagree with the visible surface.

Local corrections (trim the branch, lift it, cut host shoulder triangles) can
hide one junction at one angle, but they do not generalize: the next angle,
branch width, or multiple-junction case exposes another seam. This is a geometry
topology problem, not a lift or material tweak.

The venue-graph plan already notes “junction clearance and mouth clipping are
bake-time concerns” and lists mid-section T-junction baking and collision
continuity as the highest-complexity risk. It has no phase that implements
them. TD-0008 records the gap.

## Decision

Add a dedicated junction geometry bake stage (venue-graph Phase 3d) based on
**clipped and stitched road surfaces**, not overlapping ribbons.

For each junction:

1. **Derive the junction mouth.** Use the host contact and the attached section
   centreline to build the host and branch asphalt/shoulder boundary loops, then
   compute the junction mouth polygon where the asphalt surfaces meet, with a
   small clearance.
2. **Clip the ribbons.** Host and attached section ribbons are trimmed back to
   the mouth. Overlapping road surfaces are not emitted.
3. **Build one junction pad.** Triangulate the mouth polygon and stitch it to
   the clipped asphalt boundaries. Share boundary vertex positions so there are
   no T-junctions or gaps. Build shoulder and grass transitions around the pad
   the same way.
4. **Emit one surface per area.** The junction tarmac is one stitched surface;
   shoulders and grass are stitched around it. No two triangles may occupy the
   same road area.
5. **Bake collision from the same surfaces.** Collision uses the same pad and
   clipped ribbons with asphalt/shoulder surface IDs, so visual and collision
   stay aligned.
6. **Support real angles and topologies.** Handle 30°, 45°, and 90° joins,
   T/Y junctions, multiple attached sections, and closed/open hosts. Do not
   assume a perpendicular branch.
7. **Keep branch direction and length as authoring/graph concerns.** They are
   not geometry workarounds and may remain capped until the branch connects.
8. **Defer kerb, prop, and decoration clearance.** Those need mouth clipping
   too, but they are a later step. The junction pad must leave room for them.

For 2–3-road junctions, implement deterministic polygon clipping and ear
clipping in-house. Evaluate a vetted 2D polygon/triangulation crate (for
example `lyon` or `geo`) in a separate ADR if N-way junctions or non-simple
mouths make the in-house path too complex. Do not introduce a mesh-CSG library
for this first pass.

## Consequences

### Positive

- No overlapping road ribbons at junctions, so angled joins do not leak dirt or
  grass onto tarmac.
- Watertight visual surfaces with no z-fighting or holes.
- Collision matches the visible asphalt/shoulder surface.
- Generalizes to real angles, multiple branches, and later pit lanes.
- Provides clear boundaries for kerb/prop/decor mouth clearance.

### Negative / risks

- Junction baking is substantially more complex than emitting independent
  ribbons.
- Polygon clipping/triangulation and boundary stitching need careful
  deterministic tests.
- The first pass should stay limited to simple 2–3-road mouths; N-way junction
  support may need a library decision.
- Kerbs, props, decorations, and barriers still cross the junction until the
  clearance step.
- Performance and bake determinism need tests as junction count grows.

## Alternatives considered

- **Keep overlapping full-width ribbons and patch with trims/lifts/cuts** —
  rejected; the current angled junctions show overlaps, z-fighting, and holes,
  and each patch is local.
- **Draw the branch as a decal/overlay** — rejected; hides the visual overlap
  but leaves collision and holes underneath.
- **Boolean/CSG union of all track meshes** — rejected for the first pass;
  heavy, harder to keep deterministic, and overkill for simple mouth polygons.
- **Endpoint-only junctions with section splits** — rejected by ADR 0003;
  it forces splits and loses the mid-section junction model.
- **Split every section at every junction** — rejected; duplicates authored
  intent and creates seams in decoration chains and routes.

## Follow-up

- Add Phase 3d to `docs/features/venue-graph-plan.md` before Phase 4.
- Record TD-0008 and keep it scheduled with the Phase 3d work.
- Implement the render bake: mouth derivation, clipping, pad triangulation,
  shoulder/grass stitching, collision, and tests.
- Add tests for angled junctions, watertight boundaries, surface continuity,
  multiple branches, and deterministic bake output.
- Validate junction pairs at 30°, 45°, and 90° in the dev preview.
- Add kerb/prop/decor mouth clearance as a later phase.
