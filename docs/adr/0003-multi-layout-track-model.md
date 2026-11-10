# 0003. Multi-layout track model

- Status: superseded by ADR 0004
- Date: 2026-11-09
- Deciders: @joegaffey, architect
- Related: `docs/adr/0004-venue-graph-shared-segments.md`, `docs/features/multi-layout-track-editor.md`,
  `crates/sim-types/src/asset.rs`, `web/src/assets/types.ts`

## Context

The procedural track editor builds one fixed-width closed loop and the asset
schema stores a single set of spawns, pit boxes, cameras, and sectors. We want
a venue-based editor where one pack contains several independent circuits that
share terrain and scenery, with a selected circuit used for test drive. The
product brief (`docs/features/multi-layout-track-editor.md`) asks the architect
to choose the contract shape before dispatch.

Two shapes were considered:

1. Extend `TrackMetadata` with `layouts` and `default_layout`.
2. Add an optional `Layouts` chunk.

An optional chunk would avoid re-baking existing packs, but it adds a new
`ChunkKind`, a second metadata surface, and a two-step lookup for every
consumer. The venue metadata is small (spawns, cameras, sectors per circuit),
so a separate chunk buys little.

## Decision

Extend `TrackMetadata` in the canonical asset contract:

- Add `LayoutSpec { id, name, length, start_finish, spawns, pit_boxes,
  cameras, sectors }`.
- Add `TrackMetadata.layouts: Vec<LayoutSpec>` and
  `TrackMetadata.default_layout: u32`.
- Keep the existing top-level `name`, `length`, `spawns`, `pit_boxes`,
  `cameras`, and `sectors` as a mirror of `layouts[default_layout]` for
  consumers that only care about the selected circuit.
- Add `TrackMetadata::normalize_layouts`, `layout`, `layout_by_id`,
  `default_layout_spec`, and `spawns_for_layout` helpers so every constructor
  produces a valid default layout.
- Bump `SCHEMA_VERSION` from 3 to 4 in `crates/sim-types/src/asset.rs` and
  `ASSET_SCHEMA_VERSION` in `web/src/assets/loader.ts`. Update the mirror
  `web/src/assets/types.ts` with the layout and metadata types.

The runtime selects a layout by id at the API boundary (URL `?layout=`), and
falls back to `default_layout` when the parameter is absent or unknown. Physics
uses `spawns_for_layout`, so an empty or invalid selection still spawns the car.

The editor persists an authored project as `lapster/track-project@1`, a
versioned JSON document owned by the render editor and stored by the web
client:

```json
{
  "schema": "lapster/track-project@1",
  "name": "Venue",
  "defaultLayout": "layout-1",
  "layouts": [
    {
      "id": "layout-1",
      "name": "Circuit 1",
      "width": 12.0,
      "controlPoints": [[0.0, 0.0]],
      "kerbs": [],
      "propLines": [],
      "startFinish": 0.0,
      "spawns": [],
      "pitBoxes": [],
      "cameras": [],
      "sectors": []
    }
  ]
}
```

`controlPoints` are `[x, z]` pairs; height is derived from the deterministic
terrain function. `width` is the full asphalt width in metres (default 12 m,
replacing the fixed constants). Kerb/prop `start` and `end` remain fractional
sample indices for the layout's 120-sample centerline.

## Consequences

### Positive

- One metadata chunk keeps pack parsing and runtime selection simple.
- Existing consumers keep reading top-level fields for the default circuit.
- Layouts are an asset-level concept, so a single pack drives every viewer.
- The project schema can evolve independently of the baked asset schema.

### Negative / risks

- Schema v4 invalidates every checked-in pack. All track fixtures must be
  re-baked, including `skidpad05`, which needs the gitignored raw AC assets.
- The top-level fields duplicate `layouts[default_layout]`; `normalize_layouts`
  enforces the invariant, but malformed source data can still drift.
- Multi-circuit geometry rebuilds the whole venue. A large venue may need
  caching or region grouping later.

## Alternatives considered

- Optional `Layouts` chunk: avoids a schema bump but adds a chunk kind and
  duplicates the metadata lookup path. Rejected for the first slices.
- One pack per circuit: duplicates shared terrain and scenery and breaks the
  "select a circuit in one venue" product decision.
- Shared-section variants (full/short loops): deferred to slice 3; this model
  treats each layout as an independent closed circuit.

## Follow-up

- Re-bake `fixtures/tracks/{minimal,drift-sketchfab,skidpad05}`.
- Add editor save/load/import/export for `lapster/track-project@1`.
- Add `?layout=` selection to Drive and the physics worker.
- Defer pit-lane authoring, shared sections, and timing to later slices.
