# Car Categories and Same-Category Grids

- **Status:** local same-category grids implemented on `feat/car-categories`;
  multiplayer batch pending
- **Date:** 2026-11-25
- **Owner:** architect
- **Domains:** architect, assets, physics, render, web, multiplayer
- **Related:** ADR-0022, ADR-0011, ADR-0014, ADR-0018, ADR-0021,
  `docs/features/ai-field.md`, `docs/features/ai-race.md`,
  `docs/features/online-contact.md`, `docs/tech-debt.md`

## 1. Goal

Let the player choose a car category, then a car from that category, and race
against AI or online opponents who may use any car from the same category.

> Road cars race road cars. GP 67 cars race GP 67 cars. Within a category, the
> player, AI, and remote players may pick different cars.

Category selection happens on the main launcher. The garage only shows cars in
the selected category. Cross-category grids and balance of performance are out
of scope for this phase.

## 2. Scope

In scope:

- a `road` / `gp67` category in the car asset manifest and the web catalog;
- category selection on the landing launcher, persisted and URL-addressable;
- garage, Drive, Race, and Multiplayer restricted to the selected category;
- same-category AI grids that mix car models, with per-AI physics and visuals;
- same-category multiplayer rooms where players may use different cars;
- loading and caching strategy for multiple car packs;
- tests, an ADR, and gate updates.

Out of scope:

- cross-category grids, class standings, or balance of performance;
- new race-layout, snapshot, or SAB contract changes;
- damage, weather, or pit stops;
- expanding the road car catalog beyond what is needed for testing.

## 3. Current constraints

- `Simulation::new` takes one car pack and clones its physics, collision box,
  and shift thresholds for every AI (`crates/sim-worker/src/lib.rs`).
- `DriveViewer::create` takes one car pack and builds all rigs from that visual
  (`crates/sim-render/src/lib.rs`).
- `Scene::build_garage` already accepts an array of distinct cars and maps
  `car_index = index % cars.len()` (`crates/sim-render/src/garage.rs`), and
  `GarageViewer.create_concrete` already loads multiple packs.
- `CarOption` has no category (`web/src/ui/car-selection.ts`).
- The relay room fixes one car id and rejects a different car
  (`crates/relay-server/src/hub.rs`).
- `RemoteEngineAudio` uses one engine/vehicle profile for all voices
  (`web/src/audio/remote-engine-audio.ts`).
- A full GP 67 field can load 20-35 MB of packs; TD-0016 tracks catalog loading
  cost and TD-0024 tracks per-slot textured paint memory.

## 4. Contracts and interfaces

No `RaceData`, `RaceState`, SAB, or snapshot change.

`sim-types`:

```rust
pub enum CarCategory {
    Road,
    Gp67,
}

pub struct AssetManifest {
    // ...
    #[serde(default)]
    pub category: Option<CarCategory>,
}
```

`SCHEMA_VERSION` stays 3 because the manifest is JSON and the field is
optional. New car bakes require a category; track packs and legacy car packs
leave it `None`.

Mirrors:

- `web/src/assets/types.ts`: `CarCategory` and optional `category`.
- `web/src/assets/loader.ts`: accept an absent field; validate known values.
- `web/src/ui/car-selection.ts`: `CarOption.category`.
- `agents/contracts.md`: record the field against the asset schema contract.

Asset compiler:

- `CarSource` and `bake_car` emit the category.
- `bake-glb` gains `--category <road|gp67>`.
- `tools/speed-dreams/build-car.sh` reads `category` from `import.json`.

Worker and renderer:

```rust
// Internal wasm API, not a versioned contract.
Simulation::new(track_pack_bytes, player_car_pack_bytes, ai_car_packs, ...)
DriveViewer::create(..., slot_car_packs, ...)
```

`AiCar` gains per-car physics, collision box, and shift thresholds. `DriveViewer`
builds rigs in slot order and dedupes identical packs.

Net protocol (second stage):

- `JoinRequest` and `Welcome` carry `category`.
- `PlayerInfo` carries `car`.
- Room identity becomes `category + track`; the car id is per player.

## 5. Workstreams

### 5.1 Contract and asset pipeline

- Add `CarCategory` and the optional manifest field.
- Update the TS mirror and loader validation.
- Extend `CarSource`, `bake_car`, and `bake_car_glb_with_physics`; add the
  `--category` flag.
- Add `category` to each `fixtures/sources/cars/*/import.json`:
  - `road`: `mazda-rx7`, `sc-deckard-conejo`.
  - `gp67`: all six `67gp-*` cars.
- Re-bake the eight car packs. Do not re-bake tracks.
- Compiler tests for category emission and legacy-missing-category behavior.

### 5.2 Launcher, catalog, and garage

- Add category helpers, persistence (`lapster.category.v1`), and
  `?category=` parsing to `web/src/ui/car-selection.ts`.
- Landing page (`index.html`): add Road/GP 67 selectors to the Drive and Race
  panels, derive the car summary from the category, and pass `category` to
  Drive, Garage, and Multiplayer.
- Garage (`web/garage/main.ts`): filter `CAR_OPTIONS`, `CAR_URLS`, bays, and
  navigation to the selected category. Handle a two-car road category.
- Multiplayer lobby (`web/multiplayer/main.ts`): show the category, validate
  the selected car, and carry category in the invite URL.
- Drive (`web/drive/main.ts`): resolve and validate the category; a mismatched
  `?car=` falls back to a car in the selected category.
- Tests for persistence, filtering, four-car/six-car/ten-car categories, and
  mismatch handling.

### 5.3 Physics: per-AI car packs

- Change the worker init payload to player plus ordered AI packs.
- Each `AiCar` uses its own physics, collision box, redline, gear count, and
  shift thresholds.
- Keep `MAX_AI = 9` and the existing grid clamp.
- Keep setup/assist calls global for v1.
- Add category-aware AI target speed if GP cars exceed the current `32 m/s`
  cap in `sim-core/src/ai.rs`.
- Tests with mixed packs in one simulation and contact across different car
  sizes.

### 5.4 Renderer: per-slot rigs

- Change `DriveViewer::create` to accept ordered per-slot packs.
- Dedupe identical packs before uploading textures.
- Replace the single paintability flag with per-slot hints, preserving the
  `fix/deckard-ai-palette` behavior.
- Keep player camera and cockpit offsets from the player pack; per-car
  open-wheel/cockpit flags already derive from mass.
- Expose a slot-to-car-id roster in `info()`/telemetry.
- Tests for rig identity per slot, texture dedupe, per-slot paint hints, and
  updated drive/garage snapshots.

### 5.5 Drive wiring, AI roster, loading, and audio

- Build a deterministic same-category roster: player plus up to nine AI;
  allow repeats with different paint when the category has fewer cars.
- Load packs in parallel, reuse the player pack, and cache in IndexedDB.
  Consider lazy AI-pack loading or a curated low-detail AI set if the initial
  load is too heavy.
- Pass the player and AI packs to `PhysicsClient.create` and
  `DriveViewer.create`.
- Extend `RemoteEngineAudio` to per-slot engine/vehicle profiles.
- Add `?category=` and an optional `?aiCars=` test override.
- Tests for roster validation, loading dedupe/cache, and audio profiles.

### 5.6 Multiplayer same-category

- Change the relay room identity from `car` to `category + track`.
- Add `category` to `JoinRequest`/`Welcome` and `car` to `PlayerInfo`.
- Reject joins with a different category or track.
- Load each remote player's pack on the client and assign it to the existing
  slot-indexed remote rig; drive per-slot remote audio.
- Tests: same category/different cars, cross-category rejection, remote pack
  rendering.

### 5.7 Docs, tests, and gate

- This feature doc and ADR-0022.
- Update `docs/tech-debt.md` for multi-pack loading and per-slot paint.
- Full gate, including viewer tests, WebGL2 fallback, track-ed, and hosted
  build smoke.

## 6. Effort

Focused agent-days; calendar depends on gate and review.

| Batch | Effort |
|---|---|
| Category contract, compiler, re-baked car packs | 1.5-2.5 |
| Launcher/catalog/garage filtering | 1.5-2.5 |
| Physics per-AI packs | 1.5-2.5 |
| Renderer per-slot rigs and paint | 2-3 |
| Drive roster/loading/audio | 2-3 |
| AI category tuning | 0.5-1 |
| Tests, ADR, gate, browser validation | 1.5-2 |
| **Local same-category grids** | **~10-16 days (~2-3 weeks)** |
| **Multiplayer same-category** | **+5-9 days (~1-2 weeks)** |

## 7. Sequencing

1. Wait for `fix/deckard-ai-palette` to merge and sync `dev`; it edits the same
   renderer paint-hint code.
2. Batch A: category field, compiler flag, re-bake eight car packs, ADR, mirrors.
3. Batch B: launcher category selection plus garage and multiplayer filtering.
4. Batch C: local same-category grids (physics, render, Drive roster/loading).
5. Batch D: multiplayer room category and remote car packs.
6. Polish: per-car audio, AI pace tuning, docs and tech-debt updates.

## 8. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Multi-pack loading exceeds boot/memory budget (TD-0016) | Dedupe packs, IndexedDB cache, lazy AI loading, or a curated AI set |
| Per-slot paint conflicts with `fix/deckard-ai-palette` | Sequence after that merge or agree the API first |
| Road category has only two cars | Allow repeats with different paint, or expand the catalog before a full road grid |
| GP cars outdrive the AI speed caps | Add a category-aware AI target-speed baseline |
| Relay category is client-declared | Accept for the non-authoritative prototype; validate shape and consistency |
| Legacy packs lack a category | Keep them loadable, re-bake fixtures, and hide them from filtered UI |

## 9. Acceptance criteria

- The launcher offers Road and GP 67; the choice persists and is URL-addressable.
- The garage shows only the selected category's cars.
- A race can start with the player and up to nine AI using different cars from
  the same category.
- AI cars have the correct per-car physics, collision, visuals, and paint.
- A multiplayer room accepts different cars in one category and rejects a
  different category or track.
- Full gate passes, including the WebGL2 fallback and hosted smoke.
