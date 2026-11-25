# 0022. Car categories and same-category grids

- Status: proposed
- Date: 2026-11-25
- Deciders: architect, @joegaffey
- Related: ADR-0011, ADR-0014, ADR-0018, ADR-0021,
  `docs/features/car-categories.md`, `agents/contracts.md`,
  `docs/tech-debt.md` (TD-0016, TD-0024)

## Context

The car catalog now spans two product categories: road cars (currently the
Mazda RX-7 and Deckard Conejo) and 1967 Grand Prix cars (six `67gp-*` cars).
The category is only implied by the asset id prefix. `CarOption` in
`web/src/ui/car-selection.ts` has no category field, and `AssetManifest` in
`crates/sim-types/src/asset.rs` carries no category metadata.

AI and multiplayer are single-car paths today:

- `Simulation::new` takes one car pack and clones its physics, collision box,
  and shift thresholds for every AI (`crates/sim-worker/src/lib.rs`).
- `DriveViewer::create` takes one car pack and builds every local/AI/remote rig
  from that visual (`crates/sim-render/src/lib.rs`).
- The relay room is keyed by one fixed car id and rejects joins with a
  different car (`crates/relay-server/src/hub.rs`).

The product owner wants category selection before car selection, a garage
filtered to the selected category, and AI or online opponents drawn from any
car in that category. Cross-category racing is out of scope for now: only
cars in the same category race together.

## Decision

1. **Category is asset metadata and catalog metadata.**
   - Add `CarCategory { Road, Gp67 }` to `crates/sim-types/src/asset.rs` with
     lowercase serde names `road` and `gp67`.
   - Add an optional `category: Option<CarCategory>` field to `AssetManifest`,
     serialized with `#[serde(default)]`.
   - Keep `SCHEMA_VERSION = 3`. The manifest is JSON, so an additive optional
     field keeps existing packs readable. This is the same compatibility
     posture as the optional Race chunk (ADR-0011), but for a manifest field
     rather than a chunk.
   - Track packs and legacy car packs omit the field (`None`). New car bakes
     require a category; the loader tolerates `None` so old packs still load.
   - Mirror the type in `web/src/assets/types.ts` and validate it in
     `web/src/assets/loader.ts`. Record the field in `agents/contracts.md`.

2. **Category is selected before the car.**
   - The landing launcher offers Road or GP 67.
   - The garage lists only cars in the selected category.
   - Drive, Race, and Multiplayer resolve the player's car from that category
     and reject a car/category mismatch.

3. **AI grids are same-category and may mix car models.**
   - The physics worker accepts the player pack plus an ordered per-slot array
     of AI packs from the same category. Each AI uses its own `CarPhysics`,
     `CollisionBox`, gearbox, and shift thresholds.
   - The renderer builds one rig per slot from the assigned pack and dedupes
     identical packs for texture memory. Per-slot paintable data replaces the
     single global paintability flag (see ADR-0018 and TD-0024).
   - Slot ids stay `0 = player`, `1..=n = AI`; the 64-float snapshot, SAB ABI,
     `RaceData`, and `RaceState` are unchanged.

4. **Multiplayer rooms are keyed by category and track.**
   - A room fixes its category on first join. Later joins must use the same
     category and track but may choose any car in that category.
   - `JoinRequest`/`Welcome` carry the category; `PlayerInfo` carries each
     player's car id so clients can load the remote packs they render.
   - This is an additive net-protocol change and is staged after the local
     same-category grid works.

5. **No cross-category racing and no balance of performance yet.**
   - Within a category, cars may be selected freely and may repeat when the
     category has fewer cars than grid slots.
   - Cross-category grids and class standings are deferred.

## Consequences

### Positive

- Category filtering and same-category opponent selection match the intended
  product experience without a disruptive schema bump.
- Existing tracks, race chunks, snapshots, and single-car paths stay valid.
- The renderer and garage already support multiple distinct car visuals, so
  the main work is threading per-slot packs, not rebuilding the scene path.
- Same-category racing avoids cross-class balance work for now.

### Negative / risks

- A full GP 67 grid can require six or more 3-6 MB packs, so multi-pack loading
  and caching become the schedule risk (TD-0016).
- Per-slot paint hints collide with the in-flight `fix/deckard-ai-palette`
  branch, which edits the same renderer files.
- The relay trusts a client-declared category, which is consistent with the
  non-authoritative prototype but is not enforced against asset contents.
- Legacy car packs without a category remain loadable but cannot be selected in
  a category-filtered UI; they must be re-baked or given a catalog fallback.

## Alternatives considered

- **Derive category from the id prefix.** Rejected: brittle, and multiplayer
  needs metadata that does not depend on naming.
- **Bump `SCHEMA_VERSION` to 4 and re-bake every pack.** Rejected: the manifest
  is JSON and the field is optional, so re-baking large track packs adds risk
  without compatibility benefit.
- **Add a `Category` chunk instead of a manifest field.** Rejected for now:
  the manifest is already JSON metadata, and an optional field is simpler than
  a new chunk and entrypoint.
- **Mix categories in one grid with balance of performance.** Deferred: the
  product owner wants same-category racing first.

## Follow-up

- Implement the local same-category grid first; then the multiplayer room
  category and remote car packs.
- Add category-aware AI target speed if GP cars exceed the current AI speed
  caps in `crates/sim-core/src/ai.rs`.
- Decide whether the road catalog needs more cars before a full ten-car road
  grid, or whether repeated cars with different paint are acceptable.
