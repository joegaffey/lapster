# 0006. Drivable venue network with canonical segments and loop-centric routes

- Status: accepted
- Date: 2026-11-10
- Supersedes: ADR 0005
- Deciders: @joegaffey, architect
- Related: `docs/adr/0004-venue-graph-shared-segments.md`,
  `docs/features/multi-layout-track-editor.md`,
  `crates/sim-render/src/track_editor.rs`, `crates/sim-types/src/asset.rs`

## Context

ADR 0005 modelled a venue as loops that share control-point ids and derived
shared spline pieces. That model is authoring-friendly, but it has to derive a
graph from the loops, choose an owner loop for every shared piece, and maintain
canonical tangents at junctions. The editor and baker therefore carry two
models at once: the public loop/point document and the internal edge graph.

The product requirement is now clearer: **all layouts must be drivable in one
continuous world**. A car must be able to leave one layout at a junction and
continue onto another, with the pit lane available as a normal drivable path.
A bundle of complete per-layout packs cannot support that: it would swap the
world at a junction, duplicate collision at shared sections, and make
cross-layout driving impossible.

The minimum model for continuous driving, one shared road, and a multi-layout
pack is a canonical road network. Layouts are routes over that network; they
are not independent geometry.

## Decision

Adopt a **drivable venue network** model.

### Topology

- **Junction:** a shared endpoint with an id and a position. Junctions may have
  any degree. A junction with three or more incident segments is a fork or
  merge; its asphalt is completed by a junction pad.
- **Segment:** a canonical piece of road between two junctions. It owns its
  interior control points, endpoint tangents, width/profile, and decorations.
  A segment is sampled once and appears exactly once in the baked world.
- **Route (layout):** a selectable, closed, ordered list of segment ids. A
  route owns its name, start/finish reference, spawns, sectors, and cameras. It
  does not own geometry.
- **Path:** a drivable, non-selectable, ordered list of segment ids. The pit
  lane is a path. Paths may be open. They are part of the world and use the same
  segment geometry, collision, and junction rules.
- **World:** the union of every unique segment referenced by any route or path.
  All layouts are drivable simultaneously because every unique segment is
  present exactly once.

### Segment geometry and tangents

- A segment's sampled shape depends only on its own point chain, its endpoint
  tangents, and the global width/profile. It does not depend on which routes
  reference it.
- Endpoint tangents belong to the segment, not to the junction. A branch may
  use a different tangent at a junction than the segment it joins; the junction
  pad blends the incident cross-sections. New branches seed their junction
  tangent from the incident segment and may be edited later.
- A shared segment is sampled once. Routes referencing it receive identical
  vertices, collision, kerbs, and props.
- Kerb spans and prop spans are stored on the segment and use 0..1 fractions.
  A route that needs different decoration on a shared piece must clone the
  segment and repoint the route.
- The first implementation keeps one global width/profile. Per-segment width is
  deferred.

### Routes

- Routes are closed and traverse segments in each segment's canonical
  direction. Reversing an entire route or a segment is deferred; create
  reversed segments if needed.
- Consecutive route segments must meet at the same junction:
  `segment[i].to == segment[i + 1].from`, with wrap-around.
- Every route references a start/finish segment and fraction.
- Route metadata is used for spawn, timing context, sectors, and cameras.
  `?layout=` selects this metadata and spawn; it does not filter geometry.

### Paths

- Exactly one shared pit lane per venue for now, modelled as a path.
- Paths are not listed as layouts, are not selectable by `?layout=`, and are
  never used as a session default.
- Pit boxes belong to the pit path and are mirrored into the baked layout
  metadata for runtime compatibility.

### Editor UX

The editor remains loop-centric even though segments are canonical underneath:

- The user selects a route and edits its junctions, segments, and decorations.
- Editing a shared segment updates every route that references it.
- **Make unique** clones a segment for one route and repoints that route; the
  original segment remains for the other routes.
- Snapping is used to compose routes and attach paths at junctions. It is not
  used to keep copies in sync.
- The route list is the primary UI. There is no separate edge/route/variant
  panel, and no shared/unique badges.

### Project document

Bump the editor project to `lapster/track-project@4`. The canonical Rust type is
the venue graph, serialized with these wire names:

- `controlPoints` — junctions (`id`, position, kind, profile, branch tangents);
- `edges` — canonical segments (`id`, `from`, `to`, interior `points`,
  `fromTangent`, `toTangent`, kerbs, prop spans);
- `layouts` — routes (`id`, `name`, `route` of edge ids, `startFinish`, spawns,
  sectors, cameras);
- `paths` — drivable non-selectable paths (`id`, `kind`, `route`, boxes).

The prose in this ADR uses the conceptual names junction/segment/route/path; the
wire names above are the implementation names and the web mirror must match
them. `startFinish` and `pitLane` remain as compatibility fields, but per-route
`startFinish` and `paths` are authoritative. The legacy `segments` decoration
list is not serialized in `@4`.

```json
{
  "schema": "lapster/track-project@4",
  "name": "Venue",
  "settings": { "splines": [], "kerb": {}, "pit": {} },
  "controlPoints": [
    { "id": "j1", "position": [0, 0], "kind": "normal" }
  ],
  "edges": [
    {
      "id": "s1",
      "from": "j1",
      "to": "j2",
      "points": [[10, 0], [20, 4]],
      "fromTangent": [1, 0],
      "toTangent": [1, 0],
      "kerbs": [],
      "propSpans": []
    }
  ],
  "layouts": [
    {
      "id": "full",
      "name": "Full",
      "route": ["s1", "s2", "s3"],
      "startFinish": { "edge": "s1", "fraction": 0.0 },
      "spawns": [],
      "sectors": [],
      "cameras": []
    }
  ],
  "paths": [
    {
      "id": "pit",
      "kind": "pit_lane",
      "route": ["p1", "p2"],
      "boxes": []
    }
  ]
}
```

`points` contains the segment's interior control points; the endpoints are the
junction positions. Endpoint tangents are XZ vectors and are normalized on
load. There is no separate `EditorProject` layer: the `@4` graph is the editor
document, and the `@3` loop/shared-point document remains only as migration
input.

### Asset contract

- Asset schema stays **v4**. `TrackMetadata.layouts` and `LayoutSpec` remain the
  runtime surface for per-layout spawns, sectors, and cameras.
- The visual and collision chunks contain the union of all unique segments and
  paths, baked once.
- No per-layout mesh groups or per-layout collision chunks are required for
  continuous driving. Runtime layout selection is metadata-only.
- The `@4` document is the canonical graph used by the baker. There is no
  second editor model or graph-derivation step.

### Migration and validation

- Migrate `@3` projects by converting each loop's consecutive point pairs into
  segments, deduplicating shared ordered pairs, and creating a route per loop.
  Preserve decorations where possible. Accept that migrated geometry may be
  re-sampled.
- `@1`/`@2` migration may be routed through the existing `@3` parser if it
  remains small; otherwise record the dropped support as debt.
- Validate: unique ids, finite points and tangents, known junction/segment
  references, route closure, canonical direction, start/finish membership,
  span fractions, non-empty routes, and paths that reference known segments.

## Consequences

### Positive

- All layouts are drivable in one continuous world.
- Shared sections have one geometry, one collision surface, and one set of
  decorations.
- Layouts no longer own geometry, so shared sections cannot drift.
- The editor has one source of truth instead of a loop document plus a derived
  graph.
- The pack and runtime side is already close: one visual/collision chunk plus
  `LayoutSpec` metadata.

### Negative / risks

- The editor and baker need a substantial rewrite from `@3`.
- Junction pads and collision continuity become acceptance-critical: a car must
  cross a junction between layouts without a seam, gap, or double surface.
- Migrating `@3` projects may change geometry because segments are re-sampled.
- Route direction is canonical; whole-route reversal remains deferred.
- Per-segment width and asymmetric profiles remain deferred.

## Alternatives considered

- **Bundle of complete per-layout packs:** simplest code, but cannot support
  continuous cross-layout driving or deduplicated shared collision.
- **One pack with per-layout mesh groups:** supports layout selection, but
  duplicates shared geometry and still cannot drive across layouts without one
  shared world.
- **ADR 0005 loop/shared-point model:** keeps authoring convenient, but requires
  owner-loop sampling and derived shared segments; it is a layer over the model
  actually needed for continuous driving.
- **ADR 0004 explicit edge/route/variant UI:** the canonical edge part is
  retained; the per-edge profile and variant-range UI remain rejected.

## Follow-up

- Rewrite the feature brief around routes, segments, junctions, and paths.
- Replace the `@3` project model and remove the loop-to-graph conversions and
  owner-loop sampling.
- Keep the existing segment/junction/kerb/prop geometry where possible.
- Promote junction pads and cross-layout collision validation to a blocking
  requirement.
- Update the editor Playwright suite for the route/segment UI.
- Bump the project schema to `@4`; keep asset schema v4.
