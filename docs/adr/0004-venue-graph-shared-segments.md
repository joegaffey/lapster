# 0004. Venue graph, shared segments, and cross-section profiles

- Status: superseded by ADR 0005
- Date: 2026-11-09
- Supersedes: ADR 0003
- Deciders: @joegaffey, architect
- Related: `docs/features/multi-layout-track-editor.md`,
  `crates/sim-types/src/asset.rs`, `crates/sim-render/src/track_editor.rs`

## Context

ADR 0003 modelled a pack with multiple independent circuits sharing terrain and
scenery. That model gives each layout its own control points, width, kerbs, and
props, so it cannot express full/short variants that reuse the same asphalt, nor
a shared pit lane. It also makes per-layout width changes on a reused section
impossible by construction.

The product target is a venue with:

- one base circuit and variants that share sections;
- one shared start/finish;
- one shared pit lane and shared pit boxes;
- one global racing surface profile with a pit-lane exception;
- manual kerb and prop placement;
- a path to variable width and asymmetric splines later.

## Decision

Adopt a **venue graph** model.

### Topology

- **Control point:** an authored station with an id, position, kind, profile,
  and optional per-edge branch tangents. Kinds: `normal`, `fork`, `pit_entry`,
  `pit_exit`, `start_finish`.
- **Edge:** a spline between two control points. Edges own manual kerb spans and
  prop spans. Edges are the unit of shared geometry.
- **Layout:** an ordered, closed route of edge ids plus sectors and cameras.
  Layouts do not own geometry, width, kerbs, or props.
- **Pit lane:** one shared branch route of edges between the pit-entry and
  pit-exit control points, with shared pit boxes.
- **Venue:** the graph, global settings, shared start/finish, and shared pit
  lane.

Layouts always traverse shared edges in the canonical direction. Independent
per-layout reversal is not allowed. Whole-venue reversal is deferred.

### Cross-section profile

The road is a set of named longitudinal splines with roles:

- `surface` splines define ordered bands and their surface/material;
- `prop` splines define placement paths relative to a surface spline;
- `racing` splines define the racing line.

The default profile has asphalt-left/right and runoff-left/right. Each band
between adjacent surface splines has a surface id. Kerbs are not splines; they
attach to the asphalt edge and use global width, depth, and element-length
settings, with manual spans and optional per-span element-length overrides.

Profiles resolve in this order:

1. edge `startProfile` / `endProfile` override;
2. `from` / `to` control-point profile;
3. venue default profile.

Edges interpolate between their resolved endpoint profiles. Spline ordering and
ids must match across the interpolation. At normal nodes, incident edges must
match at the shared profile. At forks and merges, branches match the node
profile and taper their own width after the junction. Explicit branch tangents
keyed by edge id control each branch direction; missing tangents fall back to
neighbour-derived sampling. Junction pads provide the asphalt surface patch and
collision at every node with three or more incident edges.

### Prop placement

A prop spline has no kind. `kind` belongs to a prop span:

```json
{ "spline": "outside-line", "start": 0.1, "end": 0.3, "kind": "blocks" }
```

This allows different props on the same lateral path without duplicate splines.

### Pit lane

- Exactly one pit lane per venue.
- Shared by every layout, with shared pit boxes.
- Uses the global pit profile for width/runoff and optional kerbs.
- Pit entry and exit are normal fork/merge nodes; the pit edges taper between
  the racing profile and the pit profile.
- Pit boxes belong to the pit lane and may be mirrored into `LayoutSpec` at bake
  time for runtime compatibility.

### Contracts and versions

- Keep asset schema v4 and `LayoutSpec`. Shared geometry is flattened into the
  visual and collision chunks.
- Bump the editor project document to `lapster/track-project@2`.
- Migrate local `@1` projects: each layout becomes a closed route of its own
  edges with no sharing.
- No new asset chunk is required for the first implementation.

### Deferred

- Asymmetric left/right UI.
- Per-control-point width editing UI.
- Variable-width tapering UI.
- Whole-venue reverse.
- Boolean/CSG junction stitching.
- Pit timing, pit speed limits, and multiplayer layout propagation.

## Consequences

### Positive

- Variants reuse real geometry instead of duplicating overlapping loops.
- Shared pit lane and boxes are modelled once.
- Width becomes a profile, enabling variable width later without a data-model
  change.
- Prop kinds and manual placement remain flexible.
- Runtime layout selection, pack export, and physics spawn selection survive.

### Negative / risks

- The render editor and web project model need a substantial rewrite.
- Fork/merge junction pads are the main geometry risk.
- Junction profile matching and taper rules must be validated.
- Spline profiles and route graph increase editor complexity.
- Local `@1` projects need migration.

## Alternatives considered

- Keep ADR 0003 and add shared sections later: rejected because layout-owned
  geometry makes true sharing impossible without deduplication.
- Boolean/CSG union of overlapping loops: rejected as fragile and hard to keep
  deterministic.
- One pack per layout: rejected; it duplicates shared terrain and scenery.
- Global single width for pit and race surfaces: rejected; the pit lane remains
  a profile exception.

## Follow-up

- Rewrite `docs/features/multi-layout-track-editor.md` around the graph model.
- Implement graph/core geometry before junction pads.
- Implement variants before the pit lane.
- Keep asset schema v4; project schema `@2`.
- Record the independent-loop prototype as superseded work in the debt register.
