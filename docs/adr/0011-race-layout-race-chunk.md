# 0011. Race layout chunk: layout as race route, with section-owned start/finish and grid

- Status: accepted
- Date: 2026-11-21
- Deciders: architect, @joegaffey
- Related: ADR-0008, ADR-0006, `docs/features/track-race-mvp.md`,
  `agents/contracts.md`

## Context

Lapster is a solo hotlap simulator today. The track pack carries visuals,
collision, metadata, spawn, surfaces, and sectors, but no usable lap route.
Track-ed bakes a visual racing line per section, but it has no start/finish or
layout concept. AI racing and online contact need a shared track-side
foundation: an ordered closed course, a start/finish, grid slots, lap sector
metadata, and a track identity that clients can compare.

The existing terminology was ambiguous. A layout, a race route, and a racing
line were being treated as separate layers. On reflection:

- A layout **is** the race route. It is the ordered course; the baked path is
  its runtime representation.
- Start/finish and grid markings are track features anchored to a section.
- Grid slots and sectors are race setup metadata, not path geometry.
- The visual racing line is an AI/visual preference, not the lap reference.

ADR-0008 defers the full layout/route-layer implementation. The MVP therefore
needs a smaller contract that still leaves room for layouts later.

## Decision

1. Add an optional `Race` chunk to the asset pack contract.
   - `ChunkKind::Race` and an optional `EntryPoints.race`.
   - The chunk carries `RaceData`:
     `layout`, `start_finish`, `grid`, and `sectors`.
   - `RaceData` has its own internal `version`.

2. Treat the layout as the race route.
   - `Layout` carries the course identity, direction, baked centerline points,
     and half-width.
   - The baked points are ordered in the direction of travel and start at the
     start/finish.
   - The layout does not own the start/finish marker, grid markings, grid
     slots, or sectors.

3. Keep track features and race setup separate from the layout.
   - The start/finish marker is anchored to a section as
     `section_id + fraction`; the pack carries a baked copy for runtime.
   - Painted grid boxes are section decorations.
   - Logical grid slots are race setup, derived from the start/finish and road
     width for the MVP.
   - Sectors are timing metadata for the course.

4. Use an implicit layout for the MVP.
   - If the spawn centerline is closed, treat it as a one-leg layout.
   - Use the spawn section and fraction as the section-local start/finish
     anchor.
   - Bake the centerline at roughly 2 m spacing, rotate it to start at the
     start/finish, derive grid slots, and generate four equal sectors.
   - Do not add a track-ed project schema change or a layout editor yet.

5. Extend the track-ed source manifest from `sim-track-glb-v1` to
   `sim-track-glb-v2` with an optional `race` block. `track-pack` accepts both
   versions.

6. Keep the global pack `SCHEMA_VERSION` at 3.
   - It is shared by car and track packs, and committed car packs cannot all be
     rebuilt from repository sources.
   - The Race chunk is additive and optional; existing packs remain readable.
   - `RaceData.version` handles future race-layout evolution.

7. Update the contract registry and mirrors.
   - Canonical: `crates/sim-types/src/asset.rs` and
     `crates/sim-types/src/race.rs`.
   - Mirrors: `web/src/assets/types.ts` and `web/src/assets/race.ts`.

## Consequences

### Positive

- AI, lap logic, race positions, and online contact get one shared track
  contract.
- Start/finish and grid ownership matches the track model instead of being
  buried in the route path.
- Existing packs and car assets remain valid; no global asset rebuild.
- The future layout layer can resolve one or more layouts to the same baked
  layout path without changing the runtime contract.
- The visual racing line remains independent, so AI line work does not change
  lap timing or race positions.

### Negative / risks

- New packs are not readable by older application builds because older readers
  do not know `ChunkKind::Race`. Static deployment is atomic, so this is
  accepted for the MVP.
- The MVP only supports a closed spawn section. Multi-section layouts and
  route legs remain deferred.
- Section-level start/finish and grid-marking authoring are deferred; the
  spawn is the MVP shortcut.
- Default track fixtures such as Skidpad do not have a race layout until a
  route source is added.
- The track-ed submodule must be updated and pinned after the producer changes.

## Alternatives considered

- **Bump the global `SCHEMA_VERSION` to 4.** Rejected for the MVP: it would
  invalidate committed car packs that cannot all be rebuilt from repository
  sources.
- **Put start/finish and grid inside the race route.** Rejected: they are
  track/race features, not path properties, and multiple routes can share the
  same physical line.
- **Implement the full layout model and editor now.** Rejected for the MVP:
  ADR-0008 already defers it, and the implicit one-leg layout proves the
  contract without a project schema change.
- **Store race data in `TrackMetadata`.** Rejected: changing the rkyv layout
  would break existing track packs.
- **Use a sidecar file instead of a pack chunk.** Rejected: it complicates
  caching, distribution, and online identity.

## Follow-up

- Define the full layout/route-leg layer over sections.
- Add section-level start/finish and grid-marking authoring.
- Add AI, race manager, and online contact batches that consume `RaceData`.
- Add an optional racing-line chunk or runtime derivation for AI.
- Add AC `fast_lane.ai` import for real-track race layouts.
